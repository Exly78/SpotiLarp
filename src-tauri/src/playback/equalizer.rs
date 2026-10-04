use std::f64::consts::PI;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use librespot_playback::{NUM_CHANNELS, SAMPLE_RATE};

pub const BAND_FREQUENCIES: [f64; 10] = [32.0, 64.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];
pub const MAX_GAIN_DB: f32 = 12.0;
const BAND_Q: f64 = 1.41;

#[derive(Clone, Copy)]
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

impl Biquad {
    fn peaking(frequency: f64, gain_db: f64, sample_rate: f64) -> Self {
        let a = 10f64.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * frequency / sample_rate;
        let alpha = w0.sin() / (2.0 * BAND_Q);
        let cos = w0.cos();
        let a0 = 1.0 + alpha / a;
        Self {
            b0: (1.0 + alpha * a) / a0,
            b1: -2.0 * cos / a0,
            b2: (1.0 - alpha * a) / a0,
            a1: -2.0 * cos / a0,
            a2: (1.0 - alpha / a) / a0,
        }
    }

    fn run(&self, x: f64, state: &mut FilterState) -> f64 {
        let y = self.b0 * x + self.b1 * state.x1 + self.b2 * state.x2 - self.a1 * state.y1 - self.a2 * state.y2;
        state.x2 = state.x1;
        state.x1 = x;
        state.y2 = state.y1;
        state.y1 = y;
        y
    }
}

#[derive(Clone, Copy, Default)]
struct FilterState {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

struct Equalizer {
    gains: Vec<f64>,
    filters: Vec<Biquad>,
    states: Vec<[FilterState; NUM_CHANNELS as usize]>,
    preamp: f64,
}

static EQUALIZER: Mutex<Equalizer> = Mutex::new(Equalizer {
    gains: Vec::new(),
    filters: Vec::new(),
    states: Vec::new(),
    preamp: 1.0,
});
static GENERATION: AtomicU64 = AtomicU64::new(0);

fn preamp(gains: &[f64]) -> f64 {
    let max_boost = gains.iter().copied().fold(0.0, f64::max);
    10f64.powf(-max_boost / 20.0)
}

pub fn configure(enabled: bool, gains_db: &[f32]) {
    let gains: Vec<f64> = BAND_FREQUENCIES
        .iter()
        .enumerate()
        .map(|(i, _)| gains_db.get(i).copied().unwrap_or(0.0).clamp(-MAX_GAIN_DB, MAX_GAIN_DB) as f64)
        .collect();
    let active = enabled && gains.iter().any(|gain| gain.abs() > 0.01);
    let Ok(mut eq) = EQUALIZER.lock() else {
        return;
    };
    GENERATION.fetch_add(1, Ordering::Release);
    if !active {
        eq.gains.clear();
        eq.filters.clear();
        eq.states.clear();
        return;
    }
    eq.filters = BAND_FREQUENCIES
        .iter()
        .zip(&gains)
        .map(|(&frequency, &gain)| Biquad::peaking(frequency, gain, SAMPLE_RATE as f64))
        .collect();
    let band_count = eq.filters.len();
    eq.states.resize(band_count, Default::default());
    eq.preamp = preamp(&gains);
    eq.gains = gains;
}

pub fn apply(samples: &[f64]) -> Option<Vec<f64>> {
    let mut eq = EQUALIZER.lock().ok()?;
    if eq.filters.is_empty() {
        return None;
    }
    let Equalizer { filters, states, preamp, .. } = &mut *eq;
    let mut output = samples.to_vec();
    for frame in output.chunks_exact_mut(NUM_CHANNELS as usize) {
        for (channel, sample) in frame.iter_mut().enumerate() {
            let mut value = *sample * *preamp;
            for (filter, state) in filters.iter().zip(states.iter_mut()) {
                value = filter.run(value, &mut state[channel]);
            }
            *sample = value;
        }
    }
    Some(output)
}

/// The same equalizer for a source at any sample rate and channel count (local
/// files), one sample at a time. Picks up setting changes as they're made.
pub struct Chain {
    generation: u64,
    sample_rate: u32,
    channels: usize,
    channel: usize,
    filters: Vec<Biquad>,
    states: Vec<FilterState>,
    preamp: f64,
}

impl Chain {
    pub fn new() -> Self {
        Self {
            generation: u64::MAX,
            sample_rate: 0,
            channels: 0,
            channel: 0,
            filters: Vec::new(),
            states: Vec::new(),
            preamp: 1.0,
        }
    }

    pub fn reset(&mut self) {
        self.channel = 0;
        self.states.fill(FilterState::default());
    }

    fn rebuild(&mut self, generation: u64, sample_rate: u32, channels: usize) {
        let gains = EQUALIZER.lock().map(|eq| eq.gains.clone()).unwrap_or_default();
        let nyquist = sample_rate as f64 / 2.0;
        self.filters = BAND_FREQUENCIES
            .iter()
            .zip(&gains)
            .filter(|(&frequency, _)| frequency < nyquist * 0.9)
            .map(|(&frequency, &gain)| Biquad::peaking(frequency, gain, sample_rate as f64))
            .collect();
        self.states = vec![FilterState::default(); self.filters.len() * channels];
        self.preamp = preamp(&gains);
        self.generation = generation;
        self.sample_rate = sample_rate;
        self.channels = channels;
        self.channel = 0;
    }

    pub fn process(&mut self, sample: f32, channels: u16, sample_rate: u32) -> f32 {
        let channels = usize::from(channels.max(1));
        let generation = GENERATION.load(Ordering::Acquire);
        if generation != self.generation || sample_rate != self.sample_rate || channels != self.channels {
            self.rebuild(generation, sample_rate, channels);
        }
        let channel = self.channel;
        self.channel = (channel + 1) % channels;
        if self.filters.is_empty() {
            return sample;
        }
        let mut value = sample as f64 * self.preamp;
        for (band, filter) in self.filters.iter().enumerate() {
            value = filter.run(value, &mut self.states[band * channels + channel]);
        }
        value as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_gain_band_passes_signal_through() {
        let filter = Biquad::peaking(1000.0, 0.0, SAMPLE_RATE as f64);
        let mut state = FilterState::default();
        for x in [0.5, -0.25, 0.75, 0.0] {
            assert!((filter.run(x, &mut state) - x).abs() < 1e-9);
        }
    }

    #[test]
    fn boost_raises_level_at_band_center() {
        let filter = Biquad::peaking(1000.0, 6.0, SAMPLE_RATE as f64);
        let mut state = FilterState::default();
        let rate = SAMPLE_RATE as f64;
        let mut peak: f64 = 0.0;
        for n in 0..(rate as usize) {
            let x = (2.0 * PI * 1000.0 * n as f64 / rate).sin();
            let y = filter.run(x, &mut state);
            if n > rate as usize / 2 {
                peak = peak.max(y.abs());
            }
        }
        assert!((peak - 2.0).abs() < 0.05, "peak {peak}");
    }
}
