use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tiny_http::{Request, Response, Server};
use url::Url;

const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);
const BIND_RETRIES: u32 = 20;
const BIND_RETRY_DELAY: Duration = Duration::from_millis(100);

pub const LOGIN_CANCELLED: &str = "Login cancelled";

pub struct CallbackResult {
    pub code: String,
    pub state: String,
}

struct Pending {
    port: u16,
    server: Arc<Server>,
    cancelled: Arc<AtomicBool>,
}

static PENDING: Mutex<Vec<Pending>> = Mutex::new(Vec::new());

pub struct Listener {
    port: u16,
    path: &'static str,
    server: Arc<Server>,
    cancelled: Arc<AtomicBool>,
}

pub async fn listen(port: u16, path: &'static str) -> Result<Listener, String> {
    cancel_where(|pending| pending == port);
    let server = Arc::new(bind(port).await?);
    let cancelled = Arc::new(AtomicBool::new(false));
    if let Ok(mut pending) = PENDING.lock() {
        pending.push(Pending {
            port,
            server: server.clone(),
            cancelled: cancelled.clone(),
        });
    }
    Ok(Listener {
        port,
        path,
        server,
        cancelled,
    })
}

pub fn cancel_all() {
    cancel_where(|_| true);
}

fn cancel_where(matches: impl Fn(u16) -> bool) {
    let Ok(mut pending) = PENDING.lock() else {
        return;
    };
    pending.retain(|login| {
        if !matches(login.port) {
            return true;
        }
        login.cancelled.store(true, Ordering::SeqCst);
        login.server.unblock();
        false
    });
}

async fn bind(port: u16) -> Result<Server, String> {
    let mut attempt = 0;
    loop {
        match Server::http(("127.0.0.1", port)) {
            Ok(server) => return Ok(server),
            Err(_) if attempt < BIND_RETRIES => {
                attempt += 1;
                tokio::time::sleep(BIND_RETRY_DELAY).await;
            }
            Err(e) => {
                return Err(format!(
                    "Couldn't listen for the Spotify login on port {port}, is another program using it? ({e})"
                ))
            }
        }
    }
}

impl Listener {
    pub fn wait(self) -> Result<CallbackResult, String> {
        let deadline = Instant::now() + LOGIN_TIMEOUT;

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let request = self
                .server
                .recv_timeout(remaining)
                .map_err(|e| format!("failed to receive OAuth redirect: {e}"))?;
            if self.cancelled.load(Ordering::SeqCst) {
                return Err(LOGIN_CANCELLED.to_string());
            }
            let request =
                request.ok_or_else(|| "Timed out waiting for the Spotify login, please try again".to_string())?;

            let full_url = format!("http://127.0.0.1:{}{}", self.port, request.url());
            let parsed = Url::parse(&full_url).map_err(|e| format!("failed to parse redirect URL: {e}"))?;

            if parsed.path() != self.path {
                let _ = request.respond(Response::empty(404));
                continue;
            }

            let mut code = None;
            let mut state = None;
            let mut error = None;
            for (key, value) in parsed.query_pairs() {
                match key.as_ref() {
                    "code" => code = Some(value.into_owned()),
                    "state" => state = Some(value.into_owned()),
                    "error" => error = Some(value.into_owned()),
                    _ => {}
                }
            }

            return match (code, state, error) {
                (_, _, Some(error)) => {
                    respond_html(request, "Login cancelled. You can close this tab and return to the app.");
                    Err(format!("Spotify login was not completed ({error})"))
                }
                (Some(code), Some(state), None) => {
                    respond_html(request, "Logged in. You can close this tab and return to the app.");
                    Ok(CallbackResult { code, state })
                }
                _ => {
                    respond_html(request, "Login failed. You can close this tab and try again in the app.");
                    Err("OAuth redirect missing code or state parameter".to_string())
                }
            };
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        if let Ok(mut pending) = PENDING.lock() {
            pending.retain(|login| !Arc::ptr_eq(&login.server, &self.server));
        }
    }
}

fn respond_html(request: Request, message: &str) {
    let body = format!("<html><body>{message}</body></html>");
    let header = "Content-Type: text/html".parse::<tiny_http::Header>().unwrap();
    let _ = request.respond(Response::from_string(body).with_header(header));
}
