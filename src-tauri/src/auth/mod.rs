pub mod loopback;
pub mod pkce;
pub mod token_store;

pub const REDIRECT_PORT: u16 = 8898;
pub const REDIRECT_URI: &str = "http://127.0.0.1:8898/callback";

pub const AUTH_SCOPES: &str =
    "user-read-private user-library-read user-library-modify playlist-read-private \
     playlist-read-collaborative user-follow-read user-follow-modify";
