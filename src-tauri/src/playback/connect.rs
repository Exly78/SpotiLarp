//! Spotify Connect, both ways.
//!
//! Other Spotify apps list SpotiLarp as a device and can play on it: librespot's
//! Spirc runs that, on a player of its own, since it takes over whatever the
//! player it's given plays and would fight SpotiLarp's queue for it.
//!
//! And SpotiLarp can play on another device. Its queue still decides what plays:
//! songs go to the device one at a time, as commands from SpotiLarp's own device
//! (the Web API turns away the playback login), and Spotify's device updates say
//! how it's getting on, turned into the same events the local player sends, so
//! the queue, Jams, history and the rest carry on as they do here. The Spotify
//! app on this computer can be one of those devices without Premium, through
//! a Spicetify extension (spotify_app.rs) instead of Spotify's servers.

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use librespot_connect::{ConnectConfig, Spirc};
use librespot_core::authentication::Credentials;
use librespot_core::config::DeviceType;
use librespot_core::dealer::protocol::Message;
use librespot_core::session::Session;
use librespot_core::SpotifyUri;
use librespot_metadata::audio::AudioItem;
use librespot_playback::mixer::Mixer;
use librespot_playback::player::{Player, PlayerEvent};
use librespot_protocol::connect::{
    Capabilities, Cluster, ClusterUpdate, Device as ProtoDevice, DeviceInfo, MemberType, PutStateReason, PutStateRequest,
};
use librespot_protocol::devices::DeviceType as SpotifyDeviceType;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::Method;
use protobuf::{EnumOrUnknown, Message as _, MessageField};
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use super::events::{self, PlaybackEvent, Source};
use super::spotify_app;
use crate::config::Settings;
use crate::state::AppState;

const MODE_EVENT: &str = "connect-mode";
const DEFAULT_NAME: &str = "SpotiLarp";
/// After a song is sent, the device may go on reporting the one before for a bit.
const SETTLE: Duration = Duration::from_secs(6);
/// A song this close to its end that gives way to another one finished, rather
/// than being replaced from somewhere else.
const END_MARGIN_MS: u32 = 4000;
const PRELOAD_BEFORE_MS: u32 = 30_000;
const SEEK_TOLERANCE_MS: u32 = 2500;
const TICK: Duration = Duration::from_secs(1);
/// A song this far past its end with no word from the device has ended.
const OVERDUE_MS: u32 = 5000;
/// Nothing came next in SpotiLarp after a song ended: the device is paused
/// rather than left playing songs of its own choosing.
const STOP_AFTER_END: Duration = Duration::from_secs(3);
/// Spirc picks up the account's last session by itself whenever Spotify
/// (re)connects it. Anything it starts this soon after isn't someone choosing SpotiLarp.
const TAKEOVER_WINDOW: Duration = Duration::from_secs(6);

/// Where SpotiLarp's music is coming out.
#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Output {
    /// SpotiLarp's own players.
    Here = 0,
    /// Another Spotify app is playing on SpotiLarp.
    Receiving = 1,
    /// SpotiLarp is playing on another device.
    Remote = 2,
}

#[derive(Default)]
pub struct ConnectState {
    output: AtomicU8,
    /// Bumped whenever SpotiLarp starts or stops playing on another device, so an old ticker stops.
    generation: AtomicU64,
    remote: Mutex<Option<Remote>>,
    connected_at: Mutex<Option<Instant>>,
    /// The account's devices and what's playing, as Spotify last said.
    cluster: Mutex<Option<Cluster>>,
    /// "This computer" was picked over the Spotify app here, for the rest of this run.
    here_by_choice: std::sync::atomic::AtomicBool,
}

impl ConnectState {
    pub fn output(&self) -> Output {
        match self.output.load(Ordering::SeqCst) {
            1 => Output::Receiving,
            2 => Output::Remote,
            _ => Output::Here,
        }
    }

    fn set_output(&self, output: Output) -> Output {
        match self.output.swap(output as u8, Ordering::SeqCst) {
            1 => Output::Receiving,
            2 => Output::Remote,
            _ => Output::Here,
        }
    }

    /// Moves from `from` to `to`, unless something else has moved it meanwhile.
    fn change(&self, from: Output, to: Output) -> bool {
        self.output
            .compare_exchange(from as u8, to as u8, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    fn just_connected(&self) -> bool {
        self.connected_at
            .lock()
            .ok()
            .and_then(|at| *at)
            .is_some_and(|at| at.elapsed() < TAKEOVER_WINDOW)
    }

    /// The Spotify app on this computer, when it's open: Spotify names its
    /// device after the computer.
    pub fn spotify_app(&self) -> Option<(String, String)> {
        if self.here_by_choice.load(Ordering::SeqCst) {
            return None;
        }
        let computer = computer_name()?;
        let cluster = self.cluster.lock().ok()?;
        cluster
            .as_ref()?
            .device
            .iter()
            .find(|(_, info)| {
                info.device_type.enum_value_or_default() == SpotifyDeviceType::COMPUTER
                    && info.name.eq_ignore_ascii_case(&computer)
                    && info.can_play
                    && !info.is_offline
            })
            .map(|(id, info)| (id.clone(), info.name.clone()))
    }

    pub fn here_by_choice(&self) -> bool {
        self.here_by_choice.load(Ordering::SeqCst)
    }

    /// Playing through the Spotify app's extension right now.
    pub fn on_spotify_app(&self) -> bool {
        self.output() == Output::Remote
            && self.remote_device().is_some_and(|(id, _)| id == spotify_app::DEVICE_ID)
    }

    fn remote_device(&self) -> Option<(String, String)> {
        let remote = self.remote.lock().ok()?;
        remote.as_ref().map(|r| (r.device_id.clone(), r.name.clone()))
    }
}

/// The other device SpotiLarp is playing on, and what it was last told and heard.
struct Remote {
    device_id: String,
    name: String,
    /// The song last sent to it.
    uri: Option<String>,
    duration_ms: u32,
    sent_at: Instant,
    /// What the UI was last told, as of `position_at`.
    playing: bool,
    position_ms: u32,
    position_at: Instant,
    volume: Option<u16>,
    preloaded: bool,
    ended_at: Option<Instant>,
    paused_after_end: bool,
    /// Whether the UI has been told what the song is (the Spotify app's
    /// extension says, once it's on).
    announced: bool,
    /// An ad is on (a free account's Spotify app).
    in_ad: bool,
    /// The device last said it was playing something else, while it was
    /// still settling: looked at again once it has.
    elsewhere: bool,
}

impl Remote {
    fn new(device_id: String, name: String) -> Self {
        let now = Instant::now();
        Self {
            device_id,
            name,
            uri: None,
            duration_ms: 0,
            sent_at: now,
            playing: false,
            position_ms: 0,
            position_at: now,
            volume: None,
            preloaded: false,
            ended_at: None,
            paused_after_end: false,
            announced: false,
            in_ad: false,
            elsewhere: false,
        }
    }

    fn expected_ms(&self) -> u32 {
        let moved = if self.playing {
            u32::try_from(self.position_at.elapsed().as_millis()).unwrap_or(u32::MAX)
        } else {
            0
        };
        self.position_ms.saturating_add(moved)
    }

    fn near_end(&self) -> bool {
        self.duration_ms > 0 && self.expected_ms().saturating_add(END_MARGIN_MS) >= self.duration_ms
    }

    fn settling(&self) -> bool {
        self.sent_at.elapsed() < SETTLE
    }

    fn anchor(&mut self, playing: bool, position_ms: u32) {
        self.playing = playing;
        self.position_ms = position_ms;
        self.position_at = Instant::now();
    }
}

/// SpotiLarp's own Spotify Connect device.
pub struct ConnectHandle {
    spirc: Spirc,
    pub device_id: String,
    _player: Arc<Player>,
    _mixer: Arc<dyn Mixer>,
}

impl ConnectHandle {
    pub fn pause(&self) {
        let _ = self.spirc.pause();
    }

    pub fn play(&self) {
        let _ = self.spirc.play();
    }

    pub fn seek(&self, position_ms: u32) {
        let _ = self.spirc.set_position_ms(position_ms);
    }

    pub fn next(&self) {
        let _ = self.spirc.next();
    }

    pub fn previous(&self) {
        let _ = self.spirc.prev();
    }

    pub fn set_volume(&self, volume: u16) {
        let _ = self.spirc.set_volume(volume);
    }

    /// Stops being the device the other apps are playing on, pausing it.
    pub fn disconnect(&self) {
        let _ = self.spirc.disconnect(true);
    }
}

impl Drop for ConnectHandle {
    fn drop(&mut self) {
        let _ = self.spirc.shutdown();
    }
}

#[derive(Clone, Serialize)]
pub struct Status {
    output: Output,
    /// The other device, while playing on one.
    device: Option<String>,
    device_id: Option<String>,
    /// Why SpotiLarp stopped playing on the other device, to show.
    reason: Option<String>,
}

#[derive(Serialize)]
pub struct Device {
    id: String,
    name: String,
    /// Spotify's word for it: "computer", "smartphone", "speaker", "tv"...
    kind: String,
    active: bool,
    /// SpotiLarp's own Connect device.
    here: bool,
    volume: Option<u8>,
    restricted: bool,
}

/// What Spotify's desktop app calls its device on this computer.
fn computer_name() -> Option<String> {
    #[cfg(windows)]
    let name = std::env::var("COMPUTERNAME").ok();
    #[cfg(target_os = "macos")]
    let name = std::process::Command::new("scutil")
        .args(["--get", "ComputerName"])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok());
    #[cfg(not(any(windows, target_os = "macos")))]
    let name = std::fs::read_to_string("/etc/hostname").ok();
    name.map(|name| name.trim().to_string()).filter(|name| !name.is_empty())
}

fn device_name(name: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        DEFAULT_NAME.to_string()
    } else {
        name.chars().take(64).collect()
    }
}

/// Starts SpotiLarp's Connect device. It connects the session (it has to be
/// listening before Spotify starts talking), so `session` mustn't be connected yet.
pub async fn start(
    app: &AppHandle,
    session: Session,
    credentials: Credentials,
    settings: &Settings,
    volume: u16,
) -> Result<ConnectHandle, String> {
    let handle = super::player::start(session.clone(), settings);
    let dealer = session.dealer();
    let cluster = dealer
        .add_listen_for("hm://connect-state/v1/cluster")
        .map_err(|e| e.to_string())?;
    let connections = dealer
        .add_listen_for("hm://pusher/v1/connections/")
        .map_err(|e| e.to_string())?;
    let config = ConnectConfig {
        name: device_name(&settings.connect_name),
        device_type: DeviceType::Computer,
        initial_volume: volume,
        ..ConnectConfig::default()
    };
    let (spirc, task) = Spirc::new(config, session.clone(), credentials, handle.player.clone(), handle.mixer.clone())
        .await
        .map_err(|e| format!("Spotify Connect didn't start: {e}"))?;
    tauri::async_runtime::spawn(task);

    let listener = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut connections = connections;
        while connections.next().await.is_some() {
            if let Ok(mut at) = listener.state::<AppState>().connect.connected_at.lock() {
                *at = Some(Instant::now());
            }
        }
    });
    let listener = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut cluster = cluster;
        while let Some(message) = cluster.next().await {
            let Some(cluster) = Message::from_raw::<ClusterUpdate>(message)
                .ok()
                .and_then(|update| update.cluster.into_option())
            else {
                continue;
            };
            let snapshot = from_cluster(&cluster);
            if let Ok(mut latest) = listener.state::<AppState>().connect.cluster.lock() {
                *latest = Some(cluster);
            }
            apply(&listener, snapshot);
        }
    });
    spawn_forwarder(app.clone(), &handle.player);

    Ok(ConnectHandle {
        spirc,
        device_id: session.device_id().to_string(),
        _player: handle.player,
        _mixer: handle.mixer,
    })
}

/// The Connect player's events, for the UI while another app is playing on SpotiLarp.
fn spawn_forwarder(app: AppHandle, player: &Player) {
    let mut channel = player.get_player_event_channel();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = channel.recv().await {
            let state = app.state::<AppState>();
            let starting = matches!(
                event,
                PlayerEvent::Loading { .. } | PlayerEvent::Playing { .. } | PlayerEvent::TrackChanged { .. }
            );
            if starting && state.connect.output() != Output::Receiving {
                if state.connect.just_connected() {
                    // Spirc picking the last session back up: SpotiLarp's own queue stays.
                    if let Some(connect) = state.playback.lock().await.as_ref().and_then(|h| h.connect.as_ref()) {
                        connect.disconnect();
                    }
                    continue;
                }
                start_receiving(&app).await;
            } else if matches!(event, PlayerEvent::Stopped { .. }) && state.connect.change(Output::Receiving, Output::Here) {
                // Another device took over.
                emit_mode(&app, None);
                continue;
            }
            match events::map_event(event) {
                // Spirc moves on by itself; SpotiLarp's queue mustn't.
                Some(PlaybackEvent::EndOfTrack | PlaybackEvent::PreloadNext | PlaybackEvent::Unavailable) | None => {}
                Some(mapped) => events::publish(&app, Source::Connect, mapped),
            }
        }
    });
}

async fn start_receiving(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.connect.set_output(Output::Receiving) == Output::Remote {
        // Spotify moved that device's session here; it has stopped by itself.
        if let Ok(mut remote) = state.connect.remote.lock() {
            *remote = None;
        }
        state.connect.generation.fetch_add(1, Ordering::SeqCst);
    }
    if let Some(handle) = state.playback.lock().await.as_ref() {
        handle.player.pause();
    }
    if state.local_active.load(Ordering::SeqCst) {
        if let Some(local) = state.local_player.get() {
            local.pause();
        }
    }
    emit_mode(app, None);
}

/// Takes playback back from the other Spotify app playing on SpotiLarp.
pub async fn stop_receiving(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.connect.change(Output::Receiving, Output::Here) {
        return;
    }
    if let Some(connect) = state.playback.lock().await.as_ref().and_then(|h| h.connect.as_ref()) {
        connect.disconnect();
    }
    emit_mode(app, None);
}

pub fn status(app: &AppHandle) -> Status {
    status_with(app, None)
}

fn status_with(app: &AppHandle, reason: Option<String>) -> Status {
    let state = app.state::<AppState>();
    let output = state.connect.output();
    let remote = (output == Output::Remote)
        .then(|| state.connect.remote_device())
        .flatten();
    Status {
        output,
        device_id: remote.as_ref().map(|(id, _)| id.clone()),
        device: remote.map(|(_, name)| name),
        reason,
    }
}

fn emit_mode(app: &AppHandle, reason: Option<String>) {
    let _ = app.emit(MODE_EVENT, status_with(app, reason));
}

/// Starts playing on another device. Nothing is sent until SpotiLarp plays a
/// song (the UI sends the one it was on straight away when it was playing).
pub async fn play_on(app: &AppHandle, device_id: String, name: String) {
    let state = app.state::<AppState>();
    state.connect.here_by_choice.store(false, Ordering::SeqCst);
    let previous = state.connect.output();
    if previous == Output::Receiving {
        stop_receiving(app).await;
    }
    if let Some((old, _)) = state.connect.remote_device().filter(|(old, _)| *old != device_id) {
        pause_device(app, old);
    }
    if let Some(handle) = state.playback.lock().await.as_ref() {
        handle.player.pause();
    }
    if state.local_active.load(Ordering::SeqCst) {
        if let Some(local) = state.local_player.get() {
            local.pause();
        }
    }
    if let Ok(mut remote) = state.connect.remote.lock() {
        *remote = Some(Remote::new(device_id, name));
    }
    let generation = state.connect.generation.fetch_add(1, Ordering::SeqCst) + 1;
    state.connect.set_output(Output::Remote);
    spawn_ticker(app.clone(), generation);
    emit_mode(app, None);
}

/// "This computer" picked by hand: stays here, even with the Spotify app open
/// and set to play through.
pub fn choose_here(app: &AppHandle) {
    app.state::<AppState>().connect.here_by_choice.store(true, Ordering::SeqCst);
    come_back(app, None);
}

/// Stops playing on the other device, pausing it, and comes back here.
pub fn come_back(app: &AppHandle, reason: Option<String>) {
    let state = app.state::<AppState>();
    if state.connect.output() != Output::Remote {
        return;
    }
    if let Some((device_id, _)) = state.connect.remote_device() {
        pause_device(app, device_id);
    }
    release(app, reason);
}

/// Back to playing here, leaving the other device as it is.
fn release(app: &AppHandle, reason: Option<String>) {
    let state = app.state::<AppState>();
    if !state.connect.change(Output::Remote, Output::Here) {
        return;
    }
    if let Ok(mut remote) = state.connect.remote.lock() {
        *remote = None;
    }
    state.connect.generation.fetch_add(1, Ordering::SeqCst);
    emit_mode(app, reason);
}

/// Forgets all of it (logging out).
pub fn reset(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let Ok(mut remote) = state.connect.remote.lock() {
        *remote = None;
    }
    state.connect.generation.fetch_add(1, Ordering::SeqCst);
    if state.connect.set_output(Output::Here) != Output::Here {
        emit_mode(app, None);
    }
}

fn pause_device(app: &AppHandle, device_id: String) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = player_command(&app, &device_id, json!({ "endpoint": "pause" })).await;
    });
}

/// Moves what the other Spotify app is playing on SpotiLarp over to another device.
pub async fn transfer(app: &AppHandle, device_id: &str) -> Result<(), String> {
    let (session, own_id) = own_device(app).await?;
    session
        .spclient()
        .transfer(&own_id, device_id, None)
        .await
        .map(|_| ())
        .map_err(|e| format!("Couldn't move the music there: {e}"))
}

/// Asks Spotify for the account's devices and what's playing. Its updates
/// only come when something changes, so with everything idle there may be
/// none yet: this signs in a hidden watcher alongside SpotiLarp's own device,
/// as Spotify's web player does, and the answer is the whole picture.
pub async fn refresh_cluster(app: &AppHandle) -> Result<(), String> {
    let (session, own_id) = own_device(app).await?;
    let watcher_id: String = format!("hobs_{}", own_id.replace('-', "")).chars().take(40).collect();
    let request = PutStateRequest {
        member_type: EnumOrUnknown::new(MemberType::CONNECT_STATE),
        put_state_reason: EnumOrUnknown::new(PutStateReason::NEW_DEVICE),
        client_side_timestamp: u64::try_from(now_ms()).unwrap_or_default(),
        device: MessageField::some(ProtoDevice {
            device_info: MessageField::some(DeviceInfo {
                device_id: watcher_id.clone(),
                name: DEFAULT_NAME.to_string(),
                device_type: EnumOrUnknown::new(SpotifyDeviceType::OBSERVER),
                capabilities: MessageField::some(Capabilities {
                    is_observable: true,
                    hidden: true,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut headers = HeaderMap::new();
    if let Ok(connection_id) = HeaderValue::from_str(&session.connection_id()) {
        headers.insert("x-spotify-connection-id", connection_id);
    }
    let answer = session
        .spclient()
        .request_with_protobuf(
            &Method::PUT,
            &format!("/connect-state/v1/devices/{watcher_id}"),
            Some(headers),
            &request,
        )
        .await
        .map_err(|e| format!("Couldn't ask Spotify for your devices: {e}"))?;
    let cluster = Cluster::parse_from_bytes(&answer).map_err(|e| format!("Spotify's device list didn't make sense: {e}"))?;
    let snapshot = from_cluster(&cluster);
    if let Ok(mut latest) = app.state::<AppState>().connect.cluster.lock() {
        *latest = Some(cluster);
    }
    apply(app, snapshot);
    Ok(())
}

/// The account's devices, fresh from Spotify, and the Spotify app here when
/// its extension is connected.
pub async fn devices(app: &AppHandle) -> Result<Vec<Device>, String> {
    let app_linked = spotify_app::connected();
    let mut devices = match connect_devices(app).await {
        Ok(devices) => devices,
        // Without Premium there's no Connect device, but the Spotify app here still works.
        Err(_) if app_linked => Vec::new(),
        Err(e) => return Err(e),
    };
    if app_linked {
        // The same app, reached the free way: listed once.
        let computer = computer_name().unwrap_or_default();
        devices.retain(|d| !(d.kind == "computer" && !d.here && d.name.eq_ignore_ascii_case(&computer)));
        let state = app.state::<AppState>();
        devices.insert(
            0,
            Device {
                id: spotify_app::DEVICE_ID.to_string(),
                name: spotify_app::DEVICE_NAME.to_string(),
                kind: "computer".to_string(),
                active: state.connect.remote_device().is_some_and(|(id, _)| id == spotify_app::DEVICE_ID),
                here: false,
                volume: None,
                restricted: false,
            },
        );
    }
    Ok(devices)
}

async fn connect_devices(app: &AppHandle) -> Result<Vec<Device>, String> {
    refresh_cluster(app).await?;
    let (_, own_id) = own_device(app).await?;
    let state = app.state::<AppState>();
    let cluster = state.connect.cluster.lock().map_err(|e| e.to_string())?;
    let Some(cluster) = cluster.as_ref() else {
        return Ok(Vec::new());
    };
    let mut devices: Vec<Device> = cluster
        .device
        .iter()
        .filter(|(_, info)| {
            // Web players watching along, and devices that have gone away.
            info.device_type.enum_value_or_default() != SpotifyDeviceType::OBSERVER && !info.is_offline
        })
        .map(|(id, info)| Device {
            here: *id == own_id,
            name: if info.name.is_empty() { "Device".to_string() } else { info.name.clone() },
            kind: device_kind(info.device_type.enum_value_or_default()),
            active: *id == cluster.active_device_id,
            volume: Some((u64::from(info.volume.min(u32::from(u16::MAX))) * 100 / u64::from(u16::MAX)) as u8),
            restricted: !info.can_play || !info.disallow_playback_reasons.is_empty(),
            id: id.clone(),
        })
        .collect();
    devices.sort_by(|a, b| b.active.cmp(&a.active).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(devices)
}

/// "computer", "smartphone", "speaker"..., as the Web API says them.
fn device_kind(kind: SpotifyDeviceType) -> String {
    format!("{kind:?}").to_lowercase().replace('_', "")
}

/// Plays a song on the other device. `session` reads what the song is; the
/// Spotify app's extension says itself, so it needs none (nor Premium).
pub async fn play(app: &AppHandle, session: Option<&Session>, uri: &str, position_ms: u32, paused: bool) -> Result<(), String> {
    let spotify_uri = SpotifyUri::from_uri(uri).map_err(|e| e.to_string())?;
    let device_id = with_remote(app, |remote| {
        let app_linked = remote.device_id == spotify_app::DEVICE_ID;
        remote.uri = Some(uri.to_string());
        remote.duration_ms = 0;
        remote.sent_at = Instant::now();
        remote.preloaded = false;
        remote.ended_at = None;
        remote.paused_after_end = false;
        remote.announced = false;
        // The app's own word says when it's playing (after any ad).
        remote.anchor(!paused && !app_linked, position_ms);
        remote.device_id.clone()
    })?;
    if device_id == spotify_app::DEVICE_ID || session.is_none() {
        if let Err(e) = player_command(app, &device_id, play_command(uri, position_ms, paused)).await {
            release(app, None);
            return Err(e);
        }
        return Ok(());
    }
    let session = session.ok_or_else(|| "Playback isn't connected".to_string())?;
    let (sent, item) = tokio::join!(
        player_command(app, &device_id, play_command(uri, position_ms, paused)),
        AudioItem::get_file(session, spotify_uri),
    );
    if let Err(e) = sent {
        release(app, None);
        return Err(e);
    }
    if let Ok(item) = item {
        let duration_ms = item.duration_ms;
        let _ = with_remote(app, |remote| {
            if remote.uri.as_deref() == Some(uri) {
                remote.duration_ms = duration_ms;
                remote.announced = true;
            }
        });
        if let Some(changed) = events::map_event(PlayerEvent::TrackChanged { audio_item: Box::new(item) }) {
            events::publish(app, Source::Remote, changed);
        }
    }
    events::publish(
        app,
        Source::Remote,
        if paused {
            PlaybackEvent::Paused { position_ms }
        } else {
            PlaybackEvent::Playing { position_ms }
        },
    );
    Ok(())
}

/// A context of just this song: the queue here decides what comes after.
fn play_command(uri: &str, position_ms: u32, paused: bool) -> serde_json::Value {
    json!({
        "endpoint": "play",
        "context": {
            "uri": uri,
            "url": format!("context://{uri}"),
            "metadata": {},
            "pages": [{ "tracks": [{ "uri": uri }] }],
        },
        "play_origin": { "feature_identifier": "spotilarp", "feature_version": env!("CARGO_PKG_VERSION") },
        "options": {
            "license": "on-demand",
            "skip_to": { "track_index": 0, "track_uri": uri },
            "seek_to": position_ms,
            "initially_paused": paused,
            "player_options_override": {},
        },
    })
}

pub async fn pause(app: &AppHandle) -> Result<(), String> {
    let (device_id, position_ms) = with_remote(app, |remote| {
        let position_ms = remote.expected_ms();
        remote.anchor(false, position_ms);
        (remote.device_id.clone(), position_ms)
    })?;
    events::publish(app, Source::Remote, PlaybackEvent::Paused { position_ms });
    player_command(app, &device_id, json!({ "endpoint": "pause" })).await
}

pub async fn resume(app: &AppHandle) -> Result<(), String> {
    let (device_id, position_ms) = with_remote(app, |remote| {
        let position_ms = remote.expected_ms();
        remote.anchor(true, position_ms);
        (remote.device_id.clone(), position_ms)
    })?;
    events::publish(app, Source::Remote, PlaybackEvent::Playing { position_ms });
    player_command(app, &device_id, json!({ "endpoint": "resume" })).await
}

pub async fn seek(app: &AppHandle, position_ms: u32) -> Result<(), String> {
    let device_id = with_remote(app, |remote| {
        remote.anchor(remote.playing, position_ms);
        remote.preloaded = remote.preloaded && position_ms.saturating_add(PRELOAD_BEFORE_MS) >= remote.duration_ms;
        remote.device_id.clone()
    })?;
    events::publish(app, Source::Remote, PlaybackEvent::Seeked { position_ms });
    player_command(
        app,
        &device_id,
        json!({ "endpoint": "seek_to", "value": position_ms, "position": position_ms }),
    )
    .await
}

pub async fn set_volume(app: &AppHandle, volume: u16) -> Result<(), String> {
    let device_id = with_remote(app, |remote| {
        remote.volume = Some(volume);
        remote.device_id.clone()
    })?;
    if device_id == spotify_app::DEVICE_ID {
        return spotify_app::set_volume(volume);
    }
    let (session, own_id) = own_device(app).await?;
    send(
        &session,
        Method::PUT,
        &format!("/connect-state/v1/connect/volume/from/{own_id}/to/{device_id}"),
        json!({ "volume": volume }),
    )
    .await
}

fn with_remote<T>(app: &AppHandle, f: impl FnOnce(&mut Remote) -> T) -> Result<T, String> {
    let state = app.state::<AppState>();
    let mut remote = state.connect.remote.lock().map_err(|e| e.to_string())?;
    remote
        .as_mut()
        .map(f)
        .ok_or_else(|| "SpotiLarp isn't playing on another device".to_string())
}

/// What a device update, or the Spotify app's extension, says is playing.
#[derive(Default)]
pub(super) struct Snapshot {
    pub(super) active_device_id: String,
    pub(super) uri: String,
    /// Playing, rather than paused or stopped.
    pub(super) playing: bool,
    pub(super) position_ms: u32,
    pub(super) volumes: Vec<(String, u16)>,
    /// An ad is on, instead of a song.
    pub(super) ad: bool,
    pub(super) duration_ms: u32,
    /// What the song is, when the device says (the Spotify app's extension does).
    pub(super) track: Option<PlaybackEvent>,
    /// From the Spotify app's extension, rather than Spotify's device updates.
    pub(super) from_app: bool,
}

/// The Spotify app's extension's word on what it's doing.
pub(super) fn report(app: &AppHandle, snapshot: Snapshot) {
    apply(app, snapshot);
}

/// A device SpotiLarp may be playing on went away.
pub(super) fn device_gone(app: &AppHandle, device_id: &str, reason: &str) {
    let state = app.state::<AppState>();
    if state.connect.remote_device().is_some_and(|(id, _)| id == device_id) {
        release(app, Some(reason.to_string()));
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn from_cluster(cluster: &Cluster) -> Snapshot {
    let mut snapshot = Snapshot {
        active_device_id: cluster.active_device_id.clone(),
        volumes: cluster
            .device
            .iter()
            .map(|(id, info)| (id.clone(), info.volume.min(u32::from(u16::MAX)) as u16))
            .collect(),
        ..Snapshot::default()
    };
    if let Some(player) = cluster.player_state.as_ref() {
        snapshot.playing = player.is_playing && !player.is_paused;
        snapshot.uri = player.track.as_ref().map(|t| t.uri.clone()).unwrap_or_default();
        let mut position = player.position_as_of_timestamp;
        if snapshot.playing && player.timestamp > 0 {
            // On Spotify's clock, so this computer's being off doesn't matter.
            let now = if cluster.server_timestamp_ms > 0 { cluster.server_timestamp_ms } else { now_ms() };
            let speed = if player.playback_speed > 0.0 { player.playback_speed } else { 1.0 };
            position += ((now - player.timestamp).max(0) as f64 * speed) as i64;
        }
        snapshot.position_ms = position.clamp(0, i64::from(u32::MAX)) as u32;
    }
    snapshot
}

/// Turns what the other device is doing into the events the local player would send.
fn apply(app: &AppHandle, snapshot: Snapshot) {
    let state = app.state::<AppState>();
    if state.connect.output() != Output::Remote {
        return;
    }
    let mut out = Vec::new();
    let mut lost = None;
    let mut resend = None;
    {
        let Ok(mut guard) = state.connect.remote.lock() else {
            return;
        };
        let Some(remote) = guard.as_mut() else {
            return;
        };
        // Each device has one voice: the Spotify app's extension, or Spotify's
        // device updates (which, on Premium, also tell of that app as a Connect device).
        if (remote.device_id == spotify_app::DEVICE_ID) != snapshot.from_app {
            return;
        }
        if let Some(&(_, volume)) = snapshot.volumes.iter().find(|(id, _)| *id == remote.device_id) {
            if remote.volume != Some(volume) {
                remote.volume = Some(volume);
                out.push(PlaybackEvent::VolumeChanged { volume });
            }
        }
        let ours = snapshot.active_device_id == remote.device_id
            && remote.uri.as_deref().is_some_and(|uri| uri == snapshot.uri);
        remote.elsewhere = !ours && !snapshot.ad && remote.settling();
        if snapshot.ad {
            // A free account's ad: SpotiLarp waits it out, its song's clock stopped.
            if remote.uri.is_some() && remote.ended_at.is_none() {
                if remote.near_end() {
                    remote.ended_at = Some(Instant::now());
                    remote.playing = false;
                    out.push(PlaybackEvent::EndOfTrack);
                } else if remote.playing {
                    let position_ms = remote.expected_ms();
                    remote.anchor(false, position_ms);
                    out.push(PlaybackEvent::Paused { position_ms });
                }
            }
            remote.in_ad = true;
        } else if remote.in_ad && remote.uri.is_some() && !ours {
            // The ad's over and Spotify went on with something else: SpotiLarp's song again.
            remote.in_ad = false;
            remote.sent_at = Instant::now();
            if let Some(uri) = remote.uri.clone().filter(|_| remote.ended_at.is_none()) {
                resend = Some((remote.device_id.clone(), uri, remote.position_ms));
            }
        } else if remote.uri.is_none() || remote.ended_at.is_some() {
            // Nothing sent yet, or the song ended and SpotiLarp's next one is on its way.
        } else if ours && !remote.announced {
            // The app's first word on the song sent: what it is, then how it's going.
            remote.in_ad = false;
            remote.announced = true;
            if let Some(track) = snapshot.track {
                out.push(track);
            }
            if snapshot.duration_ms > 0 {
                remote.duration_ms = snapshot.duration_ms;
            }
            out.push(if snapshot.playing {
                PlaybackEvent::Playing { position_ms: snapshot.position_ms }
            } else {
                PlaybackEvent::Paused { position_ms: snapshot.position_ms }
            });
            remote.anchor(snapshot.playing, snapshot.position_ms);
        } else if ours {
            remote.in_ad = false;
            if remote.playing && !snapshot.playing && remote.near_end() {
                // Its list of one song ran out.
                remote.ended_at = Some(Instant::now());
                remote.playing = false;
                out.push(PlaybackEvent::EndOfTrack);
            } else {
                if snapshot.playing != remote.playing {
                    out.push(if snapshot.playing {
                        PlaybackEvent::Playing { position_ms: snapshot.position_ms }
                    } else {
                        PlaybackEvent::Paused { position_ms: snapshot.position_ms }
                    });
                } else if remote.expected_ms().abs_diff(snapshot.position_ms) > SEEK_TOLERANCE_MS {
                    out.push(PlaybackEvent::Seeked { position_ms: snapshot.position_ms });
                }
                remote.anchor(snapshot.playing, snapshot.position_ms);
            }
        } else if remote.settling() {
            // Still reporting the song before.
        } else if remote.near_end() {
            // It moved on by itself at the end of the song (autoplay, or a queue of its own).
            remote.ended_at = Some(Instant::now());
            remote.playing = false;
            out.push(PlaybackEvent::EndOfTrack);
        } else if snapshot.active_device_id != remote.device_id {
            lost = Some(if snapshot.active_device_id.is_empty() {
                format!("{} stopped playing", remote.name)
            } else {
                format!("Spotify moved playback off {}", remote.name)
            });
        } else {
            lost = Some(format!("Something else started playing on {}", remote.name));
        }
    }
    for event in out {
        events::publish(app, Source::Remote, event);
    }
    if let Some(reason) = lost {
        release(app, Some(reason));
    }
    if let Some((device_id, uri, position_ms)) = resend {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let _ = player_command(&app, &device_id, play_command(&uri, position_ms, false)).await;
        });
    }
}

/// Keeps time while playing on another device: when to get the next song
/// ready, and asking the device how it's doing when it has gone quiet.
fn spawn_ticker(app: AppHandle, generation: u64) {
    tauri::async_runtime::spawn(async move {
        let mut ticks = tokio::time::interval(TICK);
        loop {
            ticks.tick().await;
            let state = app.state::<AppState>();
            if state.connect.generation.load(Ordering::SeqCst) != generation || state.connect.output() != Output::Remote {
                return;
            }
            let mut out = None;
            let mut pause = None;
            {
                let Ok(mut guard) = state.connect.remote.lock() else {
                    return;
                };
                let Some(remote) = guard.as_mut() else {
                    return;
                };
                if let Some(ended_at) = remote.ended_at {
                    if !remote.paused_after_end && ended_at.elapsed() > STOP_AFTER_END {
                        remote.paused_after_end = true;
                        pause = Some(remote.device_id.clone());
                    }
                } else if remote.elsewhere && !remote.settling() && remote.near_end() {
                    // The song ran out while the device was settling after the last one sent.
                    remote.elsewhere = false;
                    remote.ended_at = Some(Instant::now());
                    remote.playing = false;
                    out = Some(PlaybackEvent::EndOfTrack);
                } else if remote.uri.is_some() && remote.playing && remote.duration_ms > 0 {
                    let expected = remote.expected_ms();
                    if expected > remote.duration_ms.saturating_add(OVERDUE_MS) {
                        // No word from the device that it moved on, but it will have.
                        remote.ended_at = Some(Instant::now());
                        remote.playing = false;
                        out = Some(PlaybackEvent::EndOfTrack);
                    } else if !remote.preloaded && expected.saturating_add(PRELOAD_BEFORE_MS) >= remote.duration_ms {
                        remote.preloaded = true;
                        out = Some(PlaybackEvent::PreloadNext);
                    }
                }
            }
            if let Some(event) = out {
                events::publish(&app, Source::Remote, event);
            }
            if let Some(device_id) = pause {
                let _ = player_command(&app, &device_id, json!({ "endpoint": "pause" })).await;
            }
        }
    });
}

/// The playback session and SpotiLarp's own Connect device, which commands
/// to other devices come from.
async fn own_device(app: &AppHandle) -> Result<(Session, String), String> {
    let state = app.state::<AppState>();
    let playback = state.playback.lock().await;
    let handle = playback
        .as_ref()
        .filter(|h| !h.session.is_invalid())
        .ok_or_else(|| "Playback isn't connected".to_string())?;
    let connect = handle.connect.as_ref().ok_or_else(|| {
        "Playing on other devices needs \"Show SpotiLarp in your Spotify apps\" on (Settings, Spotify Connect)".to_string()
    })?;
    Ok((handle.session.clone(), connect.device_id.clone()))
}

/// A player command (play, pause, seek...) to another device, as Spotify's own apps send them.
async fn player_command(app: &AppHandle, device_id: &str, mut command: serde_json::Value) -> Result<(), String> {
    if device_id == spotify_app::DEVICE_ID {
        return spotify_app::command(&command);
    }
    let (session, own_id) = own_device(app).await?;
    command["logging_params"] = json!({
        "command_id": format!("{:032x}", (now_ms() as u128).wrapping_mul(0x9e37_79b9_7f4a_7c15)),
        "command_initiated_time": now_ms(),
        "page_instance_ids": [],
        "interaction_ids": [],
    });
    send(
        &session,
        Method::POST,
        &format!("/connect-state/v1/player/command/from/{own_id}/to/{device_id}"),
        json!({ "command": command }),
    )
    .await
}

async fn send(session: &Session, method: Method, endpoint: &str, body: serde_json::Value) -> Result<(), String> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    session
        .spclient()
        .request_as_json(&method, endpoint, Some(headers), Some(&body.to_string()))
        .await
        .map(|_| ())
        .map_err(|e| format!("Spotify didn't pass that on to the device: {e}"))
}

#[cfg(test)]
mod tests {
    use super::{device_kind, device_name, SpotifyDeviceType};

    #[test]
    fn names_the_device() {
        assert_eq!(device_name("  "), "SpotiLarp");
        assert_eq!(device_name(" Desk "), "Desk");
        assert_eq!(device_name(&"x".repeat(100)).len(), 64);
    }

    #[test]
    fn names_device_kinds_as_the_web_api_does() {
        assert_eq!(device_kind(SpotifyDeviceType::COMPUTER), "computer");
        assert_eq!(device_kind(SpotifyDeviceType::CAST_VIDEO), "castvideo");
    }
}
