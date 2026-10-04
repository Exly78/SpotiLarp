use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use librespot_playback::config::VolumeCtrl;
use librespot_playback::mixer::mappings::MappedCtrl;
use librespot_playback::mixer::MixerConfig;
use rodio::source::SeekError;
use rodio::{ChannelCount, Decoder, OutputStream, Sample, SampleRate, Sink, Source};
use tauri::{AppHandle, Manager};

use super::equalizer;
use super::events::{self, PlaybackEvent};
use super::sink;
use crate::state::AppState;

const TICK: Duration = Duration::from_millis(100);
const POSITION_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct LocalTrack {
    pub path: PathBuf,
    pub uri: String,
    pub name: String,
    pub artists: String,
    pub album: String,
    pub cover_url: Option<String>,
    pub duration_ms: u32,
}

enum Command {
    Load(LocalTrack, u32),
    Missing,
    Play,
    Pause,
    Seek(u32),
    Stop,
}

/// Plays local files, sending the same events as librespot does for Spotify songs.
pub struct LocalPlayer {
    commands: Sender<Command>,
}

impl LocalPlayer {
    pub fn spawn(app: AppHandle) -> Self {
        let (commands, receiver) = mpsc::channel();
        thread::spawn(move || Engine::new(app).run(receiver));
        Self { commands }
    }

    fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }

    pub fn load(&self, track: LocalTrack, position_ms: u32) {
        self.send(Command::Load(track, position_ms));
    }

    /// Stops and reports the song as unavailable, so the queue skips it.
    pub fn missing(&self) {
        self.send(Command::Missing);
    }

    pub fn play(&self) {
        self.send(Command::Play);
    }

    pub fn pause(&self) {
        self.send(Command::Pause);
    }

    pub fn seek(&self, position_ms: u32) {
        self.send(Command::Seek(position_ms));
    }

    pub fn stop(&self) {
        self.send(Command::Stop);
    }
}

struct Loaded {
    track: LocalTrack,
    sink: Sink,
    offset_ms: u32,
    playing: bool,
    ended: bool,
}

struct Engine {
    app: AppHandle,
    volume_ctrl: VolumeCtrl,
    output: Option<(OutputStream, u64)>,
    loaded: Option<Loaded>,
    last_position_event: Instant,
}

impl Engine {
    fn new(app: AppHandle) -> Self {
        Self {
            app,
            // Same curve librespot's soft mixer gives Spotify songs.
            volume_ctrl: MixerConfig::default().volume_ctrl,
            output: None,
            loaded: None,
            last_position_event: Instant::now(),
        }
    }

    fn run(mut self, commands: Receiver<Command>) {
        loop {
            match commands.recv_timeout(TICK) {
                Ok(command) => self.handle(command),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            self.tick();
        }
    }

    fn publish(&self, event: PlaybackEvent) {
        events::publish(&self.app, events::Source::Local, event);
    }

    fn volume(&self) -> f32 {
        let volume = self.app.state::<AppState>().volume.load(Ordering::Relaxed);
        self.volume_ctrl.to_mapped(volume) as f32
    }

    fn position_ms(&self) -> u32 {
        self.loaded.as_ref().map_or(0, |loaded| {
            let played = u128::from(loaded.offset_ms) + loaded.sink.get_pos().as_millis();
            played.min(u128::from(loaded.track.duration_ms)) as u32
        })
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Load(track, position_ms) => {
                let changed = track_changed(&track);
                match self.start(track, position_ms, true) {
                    Ok(position_ms) => {
                        self.publish(changed);
                        self.publish(PlaybackEvent::Playing { position_ms });
                    }
                    Err(_) => self.publish(PlaybackEvent::Unavailable),
                }
            }
            Command::Missing => {
                self.loaded = None;
                self.publish(PlaybackEvent::Unavailable);
            }
            Command::Play | Command::Pause => {
                let play = matches!(command, Command::Play);
                let Some(loaded) = self.loaded.as_mut().filter(|loaded| !loaded.ended) else {
                    return;
                };
                if play {
                    loaded.sink.play();
                } else {
                    loaded.sink.pause();
                }
                loaded.playing = play;
                let position_ms = self.position_ms();
                self.publish(if play {
                    PlaybackEvent::Playing { position_ms }
                } else {
                    PlaybackEvent::Paused { position_ms }
                });
            }
            Command::Seek(position_ms) => {
                if self.loaded.as_ref().is_some_and(|loaded| !loaded.ended) {
                    match self.restart_at(position_ms) {
                        Ok(position_ms) => self.publish(PlaybackEvent::Seeked { position_ms }),
                        Err(_) => self.publish(PlaybackEvent::Unavailable),
                    }
                }
            }
            Command::Stop => {
                self.loaded = None;
                // Lets go of the audio device while Spotify songs play.
                self.output = None;
                self.publish(PlaybackEvent::Stopped);
            }
        }
    }

    fn output(&mut self) -> Result<&OutputStream, String> {
        let generation = sink::output_generation();
        if self.output.as_ref().is_none_or(|(_, opened)| *opened != generation) {
            self.output = None;
            self.output = Some((sink::open_stream().map_err(|e| e.to_string())?, generation));
        }
        match &self.output {
            Some((stream, _)) => Ok(stream),
            None => Err("audio output isn't open".to_string()),
        }
    }

    fn start(&mut self, track: LocalTrack, position_ms: u32, play: bool) -> Result<u32, String> {
        self.loaded = None;
        let (source, offset_ms) = open(&track.path, position_ms)?;
        let volume = self.volume();
        let sink = Sink::connect_new(self.output()?.mixer());
        sink.set_volume(volume);
        if !play {
            sink.pause();
        }
        sink.append(source);
        self.loaded = Some(Loaded {
            track,
            sink,
            offset_ms,
            playing: play,
            ended: false,
        });
        self.last_position_event = Instant::now();
        Ok(offset_ms)
    }

    fn restart_at(&mut self, position_ms: u32) -> Result<u32, String> {
        let Some(loaded) = self.loaded.take() else {
            return Err("nothing is loaded".to_string());
        };
        let (track, playing) = (loaded.track.clone(), loaded.playing);
        drop(loaded);
        self.start(track, position_ms, playing)
    }

    fn tick(&mut self) {
        let Some(loaded) = &self.loaded else {
            return;
        };
        let device_changed = self
            .output
            .as_ref()
            .is_some_and(|(_, opened)| *opened != sink::output_generation());
        if device_changed && !loaded.ended {
            if self.restart_at(self.position_ms()).is_err() {
                self.publish(PlaybackEvent::Unavailable);
            }
            return;
        }

        let volume = self.volume();
        let Some(loaded) = self.loaded.as_mut() else {
            return;
        };
        loaded.sink.set_volume(volume);
        if !loaded.playing {
            return;
        }
        if loaded.sink.empty() {
            loaded.playing = false;
            loaded.ended = true;
            self.publish(PlaybackEvent::EndOfTrack);
        } else if self.last_position_event.elapsed() >= POSITION_INTERVAL {
            self.last_position_event = Instant::now();
            self.publish(PlaybackEvent::PositionChanged {
                position_ms: self.position_ms(),
            });
        }
    }
}

/// Seeks a fresh decoder before it starts playing, instead of rodio's Sink::try_seek,
/// which blocks until the audio thread answers (forever, if the device is gone).
fn open(path: &Path, position_ms: u32) -> Result<(LocalSource, u32), String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let byte_len = file.metadata().map_err(|e| e.to_string())?.len();
    let mut builder = Decoder::builder()
        .with_data(BufReader::new(file))
        .with_byte_len(byte_len)
        .with_seekable(true);
    if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
        builder = builder.with_hint(extension);
    }
    let mut decoder = builder.build().map_err(|e| e.to_string())?;
    let offset_ms = match position_ms {
        0 => 0,
        ms if decoder.try_seek(Duration::from_millis(ms.into())).is_ok() => ms,
        _ => 0,
    };
    let source = Equalized {
        inner: decoder,
        chain: equalizer::Chain::new(),
    };
    Ok((source, offset_ms))
}

fn track_changed(track: &LocalTrack) -> PlaybackEvent {
    PlaybackEvent::TrackChanged {
        name: track.name.clone(),
        artists: track.artists.clone(),
        primary_artist_id: None,
        artist_list: Vec::new(),
        track_id: None,
        uri: track.uri.clone(),
        album: track.album.clone(),
        duration_ms: track.duration_ms,
        cover_url: track.cover_url.clone(),
    }
}

type LocalSource = Equalized<Decoder<BufReader<File>>>;

/// Runs the user's equalizer over a local file, at whatever rate it was recorded.
struct Equalized<S> {
    inner: S,
    chain: equalizer::Chain,
}

impl<S: Source> Iterator for Equalized<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        let sample = self.inner.next()?;
        Some(self.chain.process(sample, self.inner.channels(), self.inner.sample_rate()))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<S: Source> Source for Equalized<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> ChannelCount {
        self.inner.channels()
    }

    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, position: Duration) -> Result<(), SeekError> {
        self.chain.reset();
        self.inner.try_seek(position)
    }
}
