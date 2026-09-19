use tiny_http::{Response, Server};
use url::Url;

pub struct CallbackResult {
    pub code: String,
    pub state: String,
}

pub fn await_callback(port: u16) -> Result<CallbackResult, String> {
    let server = Server::http(("127.0.0.1", port))
        .map_err(|e| format!("failed to bind loopback listener on port {port}: {e}"))?;

    let request = server
        .recv()
        .map_err(|e| format!("failed to receive OAuth redirect: {e}"))?;

    let full_url = format!("http://127.0.0.1:{port}{}", request.url());
    let parsed = Url::parse(&full_url).map_err(|e| format!("failed to parse redirect URL: {e}"))?;

    let mut code = None;
    let mut state = None;
    for (key, value) in parsed.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            _ => {}
        }
    }

    let body = "<html><body>Logged in. You can close this tab and return to the app.</body></html>";
    let header = "Content-Type: text/html".parse::<tiny_http::Header>().unwrap();
    let _ = request.respond(Response::from_string(body).with_header(header));

    match (code, state) {
        (Some(code), Some(state)) => Ok(CallbackResult { code, state }),
        _ => Err("OAuth redirect missing code or state parameter".to_string()),
    }
}
