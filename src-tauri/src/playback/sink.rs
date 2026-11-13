use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use librespot_playback::audio_backend::{Sink, SinkError, SinkResult};
use librespot_playback::config::AudioFormat;
use librespot_playback::convert::Converter;
use librespot_playback::decoder::AudioPacket;
use librespot_playback::{NUM_CHANNELS, SAMPLE_RATE};
use rodio::cpal::traits::HostTrait;
use rodio::mixer::Mixer;
use rodio::{DeviceTrait, OutputStream, OutputStreamBuilder, Sink as RodioSink, StreamError};

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

/// An open audio output, and whether its device has failed since.
///
/// cpal ends a stream for good on its first error, and rodio only prints it.
/// On Windows, Bluetooth headphones cause one whenever they reconnect or
/// switch profile, so the stream has to be watched and opened again.
pub struct Output {
    stream: OutputStream,
    failed: Arc<AtomicBool>,
}

impl Output {
    pub fn mixer(&self) -> &Mixer {
        self.stream.mixer()
    }

    pub fn failed(&self) -> bool {
        self.failed.load(Ordering::SeqCst)
    }

    /// For a device that stopped taking audio without saying so.
    fn mark_failed(&self) {
        self.failed.store(true, Ordering::SeqCst);
    }
}

pub fn open_stream() -> SinkResult<Output> {
    let failed = Arc::new(AtomicBool::new(false));
    let on_error = {
        let failed = failed.clone();
        move |_: rodio::cpal::StreamError| failed.store(true, Ordering::SeqCst)
    };
    let open = |device: rodio::cpal::Device| {
        OutputStreamBuilder::from_device(device)
            .and_then(|builder| builder.with_error_callback(on_error.clone()).open_stream_or_fallback())
    };

    let host = rodio::cpal::default_host();
    let wanted = OUTPUT_DEVICE.lock().ok().and_then(|name| name.clone());
    let chosen = wanted.and_then(|name| {
        host.output_devices()
            .ok()?
            .find(|device| device.name().is_ok_and(|n| n == name))
    });
    let mut stream = match chosen.and_then(|device| open(device).ok()) {
        Some(stream) => stream,
        // The default device, or failing that any that opens.
        None => host
            .default_output_device()
            .ok_or(StreamError::NoDevice)
            .and_then(|device| open(device))
            .or_else(|err| {
                host.output_devices()
                    .ok()
                    .and_then(|mut devices| devices.find_map(|device| open(device).ok()))
                    .ok_or(err)
            })
            .map_err(|e| SinkError::ConnectionRefused(format!("failed to open default audio output: {e}")))?,
    };
    stream.log_on_drop(false);
    Ok(Output { stream, failed })
}

struct OpenOutput {
    sink: RodioSink,
    output: Output,
    generation: u64,
    /// Whether the device has played anything yet. One that fails before it
    /// ever has isn't opened again on its own, so a broken device pauses
    /// playback instead of being reopened over and over.
    worked: bool,
}

pub struct FixedRodioSink {
    output: Option<OpenOutput>,
}

pub fn open(_device: Option<String>, _format: AudioFormat) -> Box<dyn Sink> {
    Box::new(FixedRodioSink { output: None })
}

impl FixedRodioSink {
    fn ensure_output(&mut self) -> SinkResult<&RodioSink> {
        let generation = output_generation();
        if let Some(open) = &self.output {
            if open.generation != generation {
                self.output = None;
            } else if open.output.failed() {
                let worked = open.worked;
                self.output = None;
                if !worked {
                    return Err(SinkError::NotConnected("audio device stopped working".to_string()));
                }
            }
        }
        if self.output.is_none() {
            let output = open_stream()?;
            let sink = RodioSink::connect_new(output.mixer());
            self.output = Some(OpenOutput {
                sink,
                output,
                generation,
                worked: false,
            });
        }
        match &self.output {
            Some(open) => Ok(&open.sink),
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
        if let Some(open) = &self.output {
            open.sink.pause();
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
            let Some(open) = &mut self.output else {
                break;
            };
            let now_queued = open.sink.len();
            if now_queued < queued {
                open.worked = true;
                last_progress = Instant::now();
            } else if open.output.failed() || last_progress.elapsed() > STALL_TIMEOUT {
                // The device stopped taking audio. Opening it again keeps the
                // song going (the half second queued for the dead stream is lost)
                // instead of pausing until someone presses play.
                open.output.mark_failed();
                self.ensure_output()?;
                break;
            }
            queued = now_queued;
        }
        Ok(())
    }
}
