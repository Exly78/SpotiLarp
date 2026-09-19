use std::thread;
use std::time::Duration;

use librespot_playback::audio_backend::{Sink, SinkError, SinkResult};
use librespot_playback::config::AudioFormat;
use librespot_playback::convert::Converter;
use librespot_playback::decoder::AudioPacket;
use librespot_playback::{NUM_CHANNELS, SAMPLE_RATE};
use rodio::{OutputStream, OutputStreamBuilder, Sink as RodioSink};

pub struct FixedRodioSink {
    sink: RodioSink,
    _stream: OutputStream,
}

pub fn open(_device: Option<String>, _format: AudioFormat) -> Box<dyn Sink> {
    let mut stream =
        OutputStreamBuilder::open_default_stream().expect("failed to open default audio output");
    stream.log_on_drop(false);
    let sink = RodioSink::connect_new(stream.mixer());
    Box::new(FixedRodioSink {
        sink,
        _stream: stream,
    })
}

impl Sink for FixedRodioSink {
    fn start(&mut self) -> SinkResult<()> {
        self.sink.play();
        Ok(())
    }

    fn stop(&mut self) -> SinkResult<()> {
        self.sink.pause();
        Ok(())
    }

    fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> SinkResult<()> {
        let samples = packet
            .samples()
            .map_err(|e| SinkError::OnWrite(e.to_string()))?;
        let samples_f32: &[f32] = &converter.f64_to_f32(samples);
        let source = rodio::buffer::SamplesBuffer::new(
            NUM_CHANNELS as rodio::ChannelCount,
            SAMPLE_RATE,
            samples_f32,
        );
        self.sink.append(source);

        while self.sink.len() > 26 {
            thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }
}
