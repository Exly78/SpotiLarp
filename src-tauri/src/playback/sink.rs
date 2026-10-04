use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use librespot_playback::audio_backend::{Sink, SinkError, SinkResult};
use librespot_playback::config::AudioFormat;
use librespot_playback::convert::Converter;
use librespot_playback::decoder::AudioPacket;
use librespot_playback::{NUM_CHANNELS, SAMPLE_RATE};
use rodio::cpal::traits::HostTrait;
use rodio::{DeviceTrait, OutputStream, OutputStreamBuilder, Sink as RodioSink};

use super::equalizer;

const MAX_QUEUED_PACKETS: usize = 26;
const STALL_TIMEOUT: Duration = Duration::from_secs(3);

static OUTPUT_DEVICE: Mutex<Option<String>> = Mutex::new(None);
static OUTPUT_GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn set_output_device(name: Option<String>) {
    if let Ok(mut device) = OUTPUT_DEVICE.lock() {
        if *device != name {
            *device = name;
            OUTPUT_GENERATION.fetch_add(1, Ordering::SeqCst);
        }
    }
}

pub fn output_generation() -> u64 {
    OUTPUT_GENERATION.load(Ordering::SeqCst)
}

pub fn output_device_names() -> Vec<String> {
    rodio::cpal::default_host()
        .output_devices()
        .map(|devices| devices.filter_map(|device| device.name().ok()).collect())
        .unwrap_or_default()
}

pub struct FixedRodioSink {
    output: Option<(RodioSink, OutputStream)>,
    generation: u64,
}

pub fn open(_device: Option<String>, _format: AudioFormat) -> Box<dyn Sink> {
    Box::new(FixedRodioSink {
        output: None,
        generation: 0,
    })
}

pub fn open_stream() -> SinkResult<OutputStream> {
    let wanted = OUTPUT_DEVICE.lock().ok().and_then(|name| name.clone());
    let chosen = wanted.and_then(|name| {
        rodio::cpal::default_host()
            .output_devices()
            .ok()?
            .find(|device| device.name().is_ok_and(|n| n == name))
    });
    let stream = chosen
        .and_then(|device| OutputStreamBuilder::from_device(device).ok())
        .and_then(|builder| builder.open_stream_or_fallback().ok());
    let mut stream = match stream {
        Some(stream) => stream,
        None => OutputStreamBuilder::open_default_stream()
            .map_err(|e| SinkError::ConnectionRefused(format!("failed to open default audio output: {e}")))?,
    };
    stream.log_on_drop(false);
    Ok(stream)
}

fn open_output() -> SinkResult<(RodioSink, OutputStream)> {
    let stream = open_stream()?;
    let sink = RodioSink::connect_new(stream.mixer());
    Ok((sink, stream))
}

impl FixedRodioSink {
    fn ensure_output(&mut self) -> SinkResult<&RodioSink> {
        let generation = output_generation();
        if generation != self.generation {
            self.output = None;
        }
        if self.output.is_none() {
            self.output = Some(open_output()?);
            self.generation = generation;
        }
        match &self.output {
            Some((sink, _)) => Ok(sink),
            None => Err(SinkError::NotConnected("audio output isn't open".to_string())),
        }
    }
}

impl Sink for FixedRodioSink {
    fn start(&mut self) -> SinkResult<()> {
        self.ensure_output()?.play();
        Ok(())
    }

    fn stop(&mut self) -> SinkResult<()> {
        if let Some((sink, _)) = &self.output {
            sink.pause();
        }
        Ok(())
    }

    fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> SinkResult<()> {
        let samples = packet
            .samples()
            .map_err(|e| SinkError::OnWrite(e.to_string()))?;
        let equalized = equalizer::apply(samples);
        let samples_f32: &[f32] = &converter.f64_to_f32(equalized.as_deref().unwrap_or(samples));
        let source = rodio::buffer::SamplesBuffer::new(
            NUM_CHANNELS as rodio::ChannelCount,
            SAMPLE_RATE,
            samples_f32,
        );
        let sink = self.ensure_output()?;
        sink.append(source);

        let mut queued = sink.len();
        let mut last_progress = Instant::now();
        while queued > MAX_QUEUED_PACKETS {
            thread::sleep(Duration::from_millis(10));
            let Some((sink, _)) = &self.output else {
                break;
            };
            let now_queued = sink.len();
            if now_queued < queued {
                last_progress = Instant::now();
            } else if last_progress.elapsed() > STALL_TIMEOUT {
                self.output = None;
                return Err(SinkError::OnWrite("audio device stopped responding".to_string()));
            }
            queued = now_queued;
        }
        Ok(())
    }
}
