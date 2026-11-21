pub mod loopback;
pub mod oauth;
pub mod pkce;
pub mod token_store;

pub const REDIRECT_PORT: u16 = 8898;
pub const REDIRECT_PATH: &str = "/callback";

pub const AUTH_SCOPES: &str =
    "user-read-private user-library-read user-library-modify playlist-read-private \
     playlist-read-collaborative user-follow-read user-follow-modify \
     user-read-recently-played user-top-read playlist-modify-public playlist-modify-private";

/// Putting a cover on a playlist (the Recap's). Asked for when logging in,
/// but left out of `AUTH_SCOPES` so a login from before it was asked for
/// isn't told to log in again: the Recap's playlist does without a cover.
pub const COVER_SCOPE: &str = "ugc-image-upload";
