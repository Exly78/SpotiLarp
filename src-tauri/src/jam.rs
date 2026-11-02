//! Jams: listening together. One SpotiLarp hosts and the others join with its
//! code. Every app plays the songs itself (so everyone needs Premium); the
//! host's page tells the guests' pages what's playing and they follow along
//! (jam.svelte.js). This side only carries those messages: it connects the
//! apps to each other by public key, directly where the routers allow it and
//! through n0's public relays where they don't (iroh), and hands each message
//! to the page. Nothing runs until someone starts or joins a Jam.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use iroh::endpoint::{presets, Connection, ConnectionError, RecvStream, SendStream};
use iroh::{Endpoint, EndpointId, SecretKey};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::config;
use crate::discord::PresenceUpdate;
use crate::state::AppState;

pub const JAM_EVENT: &str = "jam-event";
const ALPN: &[u8] = b"spotilarp/jam/1";
/// The page a Discord "Listen along" button opens (docs/listen in the repo):
/// it hands the code after `#` to SpotiLarp as spotilarp://listen/<code>.
const LISTEN_PAGE: &str = "https://exly78.github.io/SpotiLarp/listen/";
/// What the page calls the host, on a guest's side.
const HOST_PEER: &str = "host";
/// Listeners besides the host.
const MAX_GUESTS: usize = 16;
/// Guests only ask for things, a few songs at a time; the host sends the whole queue.
const MAX_FROM_GUEST: usize = 64 * 1024;
const MAX_FROM_HOST: usize = 1024 * 1024;
/// More messages than this a second from one guest are dropped.
const GUEST_MESSAGES_PER_SECOND: u32 = 20;
const ONLINE_TIMEOUT: Duration = Duration::from_secs(15);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
/// How long a new connection gets to say hello.
const OPEN_TIMEOUT: Duration = Duration::from_secs(15);
const CLOSE_TIMEOUT: Duration = Duration::from_secs(3);

/// Why a connection was closed, sent with the close so the other side can say so.
const CLOSE_LEFT: u32 = 0;
const CLOSE_ENDED: u32 = 1;
const CLOSE_REMOVED: u32 = 2;
const CLOSE_FULL: u32 = 3;
const CLOSE_INVALID: u32 = 4;

#[derive(Default)]
pub struct JamState {
    session: tokio::sync::Mutex<Option<Session>>,
    /// Which session is current: the last events of one that's closing are dropped.
    generation: AtomicU64,
    /// The listen-along link while hosting, for the Discord status.
    listen_link: std::sync::Mutex<Option<String>>,
}

impl JamState {
    pub fn listen_link(&self) -> Option<String> {
        self.listen_link.lock().ok().and_then(|link| link.clone())
    }
}

#[derive(Serialize)]
pub struct Hosted {
    /// What guests join with.
    code: String,
    /// The same code as a web link, for the Discord button.
    link: String,
}

struct Session {
    endpoint: Endpoint,
    hosting: bool,
    peers: Peers,
}

type Peers = Arc<std::sync::Mutex<HashMap<String, Peer>>>;

struct Peer {
    connection: Connection,
    outbox: UnboundedSender<Arc<[u8]>>,
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum JamEvent {
    Joined { peer: String },
    Message { peer: String, message: Value },
    /// `reason`: "left", "ended", "removed", "full" or "lost".
    Left { peer: String, reason: &'static str },
}

/// Starts hosting a Jam.
pub async fn host(app: &AppHandle) -> Result<Hosted, String> {
    let jam = &app.state::<AppState>().jam;
    let mut session = jam.session.lock().await;
    if session.is_some() {
        return Err("You're already in a Jam".into());
    }
    let endpoint = Endpoint::builder(presets::N0)
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .map_err(|e| format!("Couldn't start the Jam: {e}"))?;
    // Guests find the host through its relay, so it has to be reachable there first.
    if tokio::time::timeout(ONLINE_TIMEOUT, endpoint.online()).await.is_err() {
        endpoint.close().await;
        return Err("Couldn't start the Jam: no connection to the internet".into());
    }
    let generation = jam.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let peers = Peers::default();
    tauri::async_runtime::spawn(accept_guests(app.clone(), endpoint.clone(), peers.clone(), generation));
    let code = encode_code(&endpoint.id());
    let link = format!("{LISTEN_PAGE}#{code}");
    *session = Some(Session { endpoint, hosting: true, peers });
    drop(session);
    set_listen_link(app, Some(link.clone())).await;
    Ok(Hosted { code, link })
}

/// Puts the listen-along button on the Discord status, or takes it off.
async fn set_listen_link(app: &AppHandle, link: Option<String>) {
    let state = app.state::<AppState>();
    if let Ok(mut current) = state.jam.listen_link.lock() {
        current.clone_from(&link);
    }
    let discord = state.discord.lock().await;
    if let Some(discord) = discord.as_ref() {
        let _ = discord.send(PresenceUpdate::ListenLink(link));
    }
}

/// Joins the Jam a code is for.
pub async fn join(app: &AppHandle, code: &str) -> Result<(), String> {
    let host_id = decode_code(code).ok_or("That isn't a Jam code")?;
    let jam = &app.state::<AppState>().jam;
    let mut session = jam.session.lock().await;
    if session.is_some() {
        return Err("You're already in a Jam".into());
    }
    let endpoint = Endpoint::builder(presets::N0)
        .secret_key(guest_key())
        .bind()
        .await
        .map_err(|e| format!("Couldn't join the Jam: {e}"))?;
    let connected = async {
        let connection = endpoint.connect(host_id, ALPN).await.map_err(|e| e.to_string())?;
        let (send, recv) = connection.open_bi().await.map_err(|e| e.to_string())?;
        Ok::<_, String>((connection, send, recv))
    };
    let (connection, send, recv) = match tokio::time::timeout(CONNECT_TIMEOUT, connected).await {
        Ok(Ok(streams)) => streams,
        Ok(Err(e)) => {
            endpoint.close().await;
            return Err(format!("Couldn't join the Jam: {e}"));
        }
        Err(_) => {
            endpoint.close().await;
            return Err("Couldn't reach that Jam. Has it ended?".into());
        }
    };
    let generation = jam.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let peers = Peers::default();
    // Registered before returning, so the page's hello right after has somewhere to go.
    let outbox = register(&peers, HOST_PEER, &connection).ok_or("Couldn't join the Jam")?;
    tauri::async_runtime::spawn(serve(
        app.clone(),
        Link { connection, send, recv, outbox },
        HOST_PEER.to_string(),
        peers.clone(),
        generation,
        MAX_FROM_HOST,
    ));
    *session = Some(Session { endpoint, hosting: false, peers });
    Ok(())
}

/// A guest is the same every time, so a host who sent it away knows it if it
/// comes back. Hosts get a new key, and code, for each Jam.
fn guest_key() -> SecretKey {
    if let Some(bytes) = config::load_jam_key() {
        return SecretKey::from_bytes(&bytes);
    }
    let key = SecretKey::generate();
    let _ = config::save_jam_key(&key.to_bytes());
    key
}

/// Sends a message to one peer, or to all of them without one (a guest's one
/// peer is the host).
pub async fn send(app: &AppHandle, peer: Option<&str>, message: &Value) -> Result<(), String> {
    let mut frame = vec![0; 4];
    serde_json::to_writer(&mut frame, message).map_err(|e| e.to_string())?;
    let len = u32::try_from(frame.len() - 4).map_err(|_| "Message too big")?;
    frame[..4].copy_from_slice(&len.to_be_bytes());
    let frame: Arc<[u8]> = frame.into();

    let state = app.state::<AppState>();
    let session = state.jam.session.lock().await;
    let session = session.as_ref().ok_or("You're not in a Jam")?;
    let peers = session.peers.lock().map_err(|e| e.to_string())?;
    match peer {
        Some(peer) => {
            if let Some(peer) = peers.get(peer) {
                let _ = peer.outbox.send(frame);
            }
        }
        None => {
            for peer in peers.values() {
                let _ = peer.outbox.send(frame.clone());
            }
        }
    }
    Ok(())
}

/// The host sending a guest away.
pub async fn remove(app: &AppHandle, peer: &str) {
    let state = app.state::<AppState>();
    let session = state.jam.session.lock().await;
    let Some(session) = session.as_ref().filter(|session| session.hosting) else {
        return;
    };
    let removed = session.peers.lock().ok().and_then(|mut peers| peers.remove(peer));
    if let Some(removed) = removed {
        removed.connection.close(CLOSE_REMOVED.into(), b"removed");
    }
}

/// Leaves the Jam, or ends it for everyone when hosting.
pub async fn leave(app: &AppHandle) {
    let jam = &app.state::<AppState>().jam;
    let session = jam.session.lock().await.take();
    jam.generation.fetch_add(1, Ordering::SeqCst);
    let Some(session) = session else {
        return;
    };
    if session.hosting {
        set_listen_link(app, None).await;
    }
    let code = if session.hosting { CLOSE_ENDED } else { CLOSE_LEFT };
    if let Ok(mut peers) = session.peers.lock() {
        for (_, peer) in peers.drain() {
            peer.connection.close(code.into(), b"");
        }
    }
    // Lets the closes go out before the connections are dropped.
    let _ = tokio::time::timeout(CLOSE_TIMEOUT, session.endpoint.close()).await;
}

async fn accept_guests(app: AppHandle, endpoint: Endpoint, peers: Peers, generation: u64) {
    while let Some(incoming) = endpoint.accept().await {
        let app = app.clone();
        let peers = peers.clone();
        tauri::async_runtime::spawn(async move {
            let Ok(connection) = incoming.await else {
                return;
            };
            // A guest opens its stream by saying hello.
            let Ok(Ok((send, recv))) = tokio::time::timeout(OPEN_TIMEOUT, connection.accept_bi()).await else {
                connection.close(CLOSE_INVALID.into(), b"");
                return;
            };
            let peer = connection.remote_id().to_string();
            let Some(outbox) = register(&peers, &peer, &connection) else {
                connection.close(CLOSE_FULL.into(), b"full");
                return;
            };
            let link = Link { connection, send, recv, outbox };
            serve(app, link, peer, peers, generation, MAX_FROM_GUEST).await;
        });
    }
}

/// Adds a peer, unless the Jam is full.
fn register(peers: &Peers, peer: &str, connection: &Connection) -> Option<UnboundedReceiver<Arc<[u8]>>> {
    let mut peers = peers.lock().ok()?;
    // Someone back before their old connection timed out takes its place.
    if let Some(old) = peers.remove(peer) {
        old.connection.close(CLOSE_LEFT.into(), b"");
    } else if peers.len() >= MAX_GUESTS {
        return None;
    }
    let (outbox, frames) = mpsc::unbounded_channel();
    peers.insert(peer.to_string(), Peer { connection: connection.clone(), outbox });
    Some(frames)
}

struct Link {
    connection: Connection,
    send: SendStream,
    recv: RecvStream,
    outbox: UnboundedReceiver<Arc<[u8]>>,
}

/// Carries messages both ways until the connection ends.
async fn serve(app: AppHandle, link: Link, peer: String, peers: Peers, generation: u64, max_len: usize) {
    let Link { connection, mut send, mut recv, mut outbox } = link;
    let from_guest = peer != HOST_PEER;
    emit(&app, generation, JamEvent::Joined { peer: peer.clone() });

    let writing = async {
        while let Some(frame) = outbox.recv().await {
            if send.write_all(&frame).await.is_err() {
                break;
            }
        }
    };
    let reading = async {
        let mut window = (Instant::now(), 0u32);
        loop {
            match read_message(&mut recv, max_len).await {
                Ok(Some(message)) => {
                    if from_guest {
                        if window.0.elapsed() >= Duration::from_secs(1) {
                            window = (Instant::now(), 0);
                        }
                        window.1 += 1;
                        if window.1 > GUEST_MESSAGES_PER_SECOND {
                            continue;
                        }
                    }
                    emit(&app, generation, JamEvent::Message { peer: peer.clone(), message });
                }
                Ok(None) => break,
                Err(InvalidMessage) => {
                    connection.close(CLOSE_INVALID.into(), b"");
                    break;
                }
            }
        }
    };
    tokio::select! {
        _ = writing => {}
        _ = reading => {}
    }

    // A guest who was sent away, or came back on a new connection, has
    // already been dealt with: only a connection still on the list says it left.
    let listed = peers.lock().is_ok_and(|mut peers| {
        let listed = peers.get(&peer).is_some_and(|p| p.connection.stable_id() == connection.stable_id());
        if listed {
            peers.remove(&peer);
        }
        listed
    });
    let reason = match connection.close_reason() {
        Some(reason) => close_reason(&reason),
        None => {
            connection.close(CLOSE_LEFT.into(), b"");
            "lost"
        }
    };
    if listed {
        emit(&app, generation, JamEvent::Left { peer, reason });
    }
}

struct InvalidMessage;

/// The next message, `None` once the other side is done. Each one is a length
/// (4 bytes, big-endian) and that much JSON.
async fn read_message(recv: &mut RecvStream, max_len: usize) -> Result<Option<Value>, InvalidMessage> {
    let mut len = [0; 4];
    // The connection ending shows up here too; its reason is read afterwards.
    if recv.read_exact(&mut len).await.is_err() {
        return Ok(None);
    }
    let len = u32::from_be_bytes(len) as usize;
    if len > max_len {
        return Err(InvalidMessage);
    }
    let mut buf = vec![0; len];
    if recv.read_exact(&mut buf).await.is_err() {
        return Ok(None);
    }
    serde_json::from_slice(&buf).map(Some).map_err(|_| InvalidMessage)
}

fn close_reason(error: &ConnectionError) -> &'static str {
    match error {
        ConnectionError::ApplicationClosed(close) => match u32::try_from(close.error_code.into_inner()) {
            Ok(CLOSE_LEFT) => "left",
            Ok(CLOSE_ENDED) => "ended",
            Ok(CLOSE_REMOVED) => "removed",
            Ok(CLOSE_FULL) => "full",
            _ => "lost",
        },
        ConnectionError::LocallyClosed => "left",
        _ => "lost",
    }
}

fn emit(app: &AppHandle, generation: u64, event: JamEvent) {
    if app.state::<AppState>().jam.generation.load(Ordering::SeqCst) == generation {
        let _ = app.emit(JAM_EVENT, event);
    }
}

/// A host's public key in 43 characters.
fn encode_code(id: &EndpointId) -> String {
    URL_SAFE_NO_PAD.encode(id.as_bytes())
}

/// Takes a code as shared, with stray spaces or quotes, or iroh's own hex form.
fn decode_code(code: &str) -> Option<EndpointId> {
    let code = code.trim().trim_matches(|c: char| c == '"' || c == '\'' || c == '`');
    if let Ok(id) = code.parse::<EndpointId>() {
        return Some(id);
    }
    let bytes: [u8; 32] = URL_SAFE_NO_PAD.decode(code).ok()?.try_into().ok()?;
    EndpointId::from_bytes(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip() {
        let id = iroh::SecretKey::generate().public();
        let code = encode_code(&id);
        assert_eq!(code.len(), 43);
        assert_eq!(decode_code(&code), Some(id));
        assert_eq!(decode_code(&format!("  \"{code}\"\n")), Some(id));
        assert_eq!(decode_code(&id.to_string()), Some(id));
        assert_eq!(decode_code("not a code"), None);
        assert_eq!(decode_code(""), None);
    }

    /// Two endpoints finding each other from a code alone, through n0's
    /// servers. Needs the internet: `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn guest_reaches_host_by_code() {
        tauri::async_runtime::block_on(async {
            let host = Endpoint::builder(presets::N0).alpns(vec![ALPN.to_vec()]).bind().await.unwrap();
            tokio::time::timeout(ONLINE_TIMEOUT, host.online()).await.unwrap();
            let code = encode_code(&host.id());

            let accepting = tauri::async_runtime::spawn({
                let host = host.clone();
                async move {
                    let connection = host.accept().await.unwrap().await.unwrap();
                    let (_send, mut recv) = connection.accept_bi().await.unwrap();
                    let message = read_message(&mut recv, MAX_FROM_GUEST).await.ok().flatten();
                    (connection.remote_id(), message)
                }
            });

            let guest = Endpoint::bind(presets::N0).await.unwrap();
            let host_id = decode_code(&code).unwrap();
            let connection = tokio::time::timeout(CONNECT_TIMEOUT, guest.connect(host_id, ALPN)).await.unwrap().unwrap();
            let (mut send, _recv) = connection.open_bi().await.unwrap();
            let body = serde_json::to_vec(&serde_json::json!({ "type": "hello", "name": "Test" })).unwrap();
            send.write_all(&(body.len() as u32).to_be_bytes()).await.unwrap();
            send.write_all(&body).await.unwrap();

            let (guest_id, message) = accepting.await.unwrap();
            assert_eq!(guest_id, guest.id());
            assert_eq!(message, Some(serde_json::json!({ "type": "hello", "name": "Test" })));
            guest.close().await;
            host.close().await;
        });
    }
}
