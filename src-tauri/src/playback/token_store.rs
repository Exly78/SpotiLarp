use keyring::Entry;

const SERVICE: &str = "SpotiLarp-connect-oauth";
const USERNAME: &str = "refresh_token";

pub fn save_refresh_token(token: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE, USERNAME).map_err(|e| e.to_string())?;
    entry.set_password(token).map_err(|e| e.to_string())
}

pub fn load_refresh_token() -> Option<String> {
    let entry = Entry::new(SERVICE, USERNAME).ok()?;
    entry.get_password().ok()
}
