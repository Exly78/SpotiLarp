use crate::secret_store;

const SERVICE: &str = "SpotiLarp-connect-oauth";
const USERNAME: &str = "refresh_token";

pub fn save_refresh_token(token: &str) -> Result<(), String> {
    secret_store::save(SERVICE, USERNAME, token)
}

pub fn load_refresh_token() -> Option<String> {
    secret_store::load(SERVICE, USERNAME).filter(|token| !token.is_empty())
}

pub fn clear_refresh_token() -> Result<(), String> {
    secret_store::clear(SERVICE, USERNAME)
}
