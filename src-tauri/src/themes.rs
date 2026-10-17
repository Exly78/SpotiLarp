//! Themes, laid out the way Spicetify's are: a folder with a `color.ini` of
//! color schemes and, optionally, a `user.css` and the images and fonts it
//! uses. They come from SpotiLarp's own Themes folder, and from Spicetify's
//! when it's installed.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use percent_encoding::percent_decode_str;
use serde::Serialize;
use tauri::http::{header, Request, Response, StatusCode};

pub const THEME_SCHEME: &str = "themefile";
const COLOR_FILE: &str = "color.ini";
const CSS_FILE: &str = "user.css";
const MAX_CSS_BYTES: u64 = 2 * 1024 * 1024;
const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;
const MAX_IMPORT_BYTES: u64 = 200 * 1024 * 1024;
const FILE_TYPES: &[(&str, &str)] = &[
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("webp", "image/webp"),
    ("svg", "image/svg+xml"),
    ("ico", "image/x-icon"),
    ("woff", "font/woff"),
    ("woff2", "font/woff2"),
    ("ttf", "font/ttf"),
    ("otf", "font/otf"),
    ("css", "text/css"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// SpotiLarp's own Themes folder.
    Own,
    /// An installed Spicetify's.
    Spicetify,
}

impl Source {
    fn key(self) -> &'static str {
        match self {
            Self::Own => "own",
            Self::Spicetify => "spicetify",
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        match key {
            "own" => Some(Self::Own),
            "spicetify" => Some(Self::Spicetify),
            _ => None,
        }
    }
}

#[derive(Serialize)]
pub struct ThemeInfo {
    /// `own:<folder>` or `spicetify:<folder>`.
    pub id: String,
    pub name: String,
    pub source: Source,
    pub schemes: Vec<String>,
    pub has_css: bool,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ColorScheme {
    pub name: String,
    /// color.ini keys to six-digit hex colors, without the `#`.
    pub colors: BTreeMap<String, String>,
}

#[derive(Serialize)]
pub struct ThemeData {
    pub schemes: Vec<ColorScheme>,
    /// user.css, with its own files' addresses pointed at this theme's folder.
    pub css: Option<String>,
}

/// Where SpotiLarp's own themes live (next to its config).
pub fn own_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("SpotiLarp").join("Themes"))
}

/// Where Spicetify keeps themes: its user folder, or (on Windows) next to
/// the program.
fn spicetify_dirs() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    let candidates = [
        dirs::config_dir().map(|dir| dir.join("spicetify")),
        dirs::home_dir().map(|dir| dir.join(".config").join("spicetify")),
        dirs::data_local_dir().map(|dir| dir.join("spicetify")),
    ];
    for dir in candidates.into_iter().flatten() {
        let themes = dir.join("Themes");
        if !found.contains(&themes) {
            found.push(themes);
        }
    }
    found
}

fn roots() -> Vec<(Source, PathBuf)> {
    let mut roots: Vec<(Source, PathBuf)> = own_dir().map(|dir| (Source::Own, dir)).into_iter().collect();
    roots.extend(spicetify_dirs().into_iter().map(|dir| (Source::Spicetify, dir)));
    roots
}

fn is_theme(dir: &Path) -> bool {
    dir.join(COLOR_FILE).is_file() || dir.join(CSS_FILE).is_file()
}

/// A folder name that can't reach outside its Themes folder.
fn safe_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', ':'])
}

pub fn list() -> Vec<ThemeInfo> {
    let mut themes: Vec<ThemeInfo> = Vec::new();
    for (source, root) in roots() {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        let mut found: Vec<ThemeInfo> = entries
            .flatten()
            .filter(|entry| entry.path().is_dir() && is_theme(&entry.path()))
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !safe_name(&name) || name.starts_with('.') {
                    return None;
                }
                let dir = entry.path();
                let schemes = fs::read_to_string(dir.join(COLOR_FILE))
                    .map(|text| parse_color_ini(&text).into_iter().map(|scheme| scheme.name).collect())
                    .unwrap_or_default();
                Some(ThemeInfo {
                    id: format!("{}:{name}", source.key()),
                    name,
                    source,
                    schemes,
                    has_css: dir.join(CSS_FILE).is_file(),
                })
            })
            .collect();
        found.sort_by_key(|theme| theme.name.to_lowercase());
        for theme in found {
            // The same theme in two of Spicetify's folders shows once.
            if !themes.iter().any(|known| known.id == theme.id) {
                themes.push(theme);
            }
        }
    }
    themes
}

fn theme_dir(source: Source, name: &str) -> Option<PathBuf> {
    if !safe_name(name) {
        return None;
    }
    roots()
        .into_iter()
        .filter(|(root_source, _)| *root_source == source)
        .map(|(_, root)| root.join(name))
        .find(|dir| dir.is_dir() && is_theme(dir))
}

fn parse_id(id: &str) -> Option<(Source, &str)> {
    let (source, name) = id.split_once(':')?;
    Some((Source::from_key(source)?, name))
}

pub fn read(id: &str) -> Result<ThemeData, String> {
    let (source, name) = parse_id(id).ok_or("That isn't a theme")?;
    let dir = theme_dir(source, name).ok_or_else(|| format!("The theme \"{name}\" isn't there any more"))?;
    let schemes = fs::read_to_string(dir.join(COLOR_FILE))
        .map(|text| parse_color_ini(&text))
        .unwrap_or_default();
    let css_path = dir.join(CSS_FILE);
    let css = match fs::metadata(&css_path) {
        Ok(meta) if meta.len() <= MAX_CSS_BYTES => fs::read_to_string(&css_path)
            .ok()
            .map(|css| rewrite_urls(&css, &file_url_base(source, name))),
        _ => None,
    };
    Ok(ThemeData { schemes, css })
}

/// Reads Spicetify's color.ini: `[Scheme]` sections of `key = hex` lines,
/// with `;` or `#` comments.
pub fn parse_color_ini(text: &str) -> Vec<ColorScheme> {
    let mut schemes: Vec<ColorScheme> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|rest| rest.split(']').next()) {
            schemes.push(ColorScheme {
                name: name.trim().to_string(),
                colors: BTreeMap::new(),
            });
            continue;
        }
        let (Some(scheme), Some((key, value))) = (schemes.last_mut(), line.split_once('=')) else {
            continue;
        };
        // A comment can follow the value.
        let value = value.split(';').next().unwrap_or("").trim();
        if let Some(hex) = normalize_hex(value) {
            scheme.colors.insert(key.trim().to_ascii_lowercase(), hex);
        }
    }
    schemes.retain(|scheme| !scheme.name.is_empty() && !scheme.colors.is_empty());
    schemes
}

/// "fff", "#FFFFFF" or "ffffffff" as "ffffff"; `None` for anything else
/// (Spicetify's `${xrdb:...}` values, say).
fn normalize_hex(value: &str) -> Option<String> {
    let hex = value.trim_start_matches('#').split_whitespace().next()?;
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let hex = hex.to_ascii_lowercase();
    match hex.len() {
        3 => Some(hex.chars().flat_map(|c| [c, c]).collect()),
        6 => Some(hex),
        8 => Some(hex[..6].to_string()),
        _ => None,
    }
}

fn file_url_base(source: Source, name: &str) -> String {
    let base = if cfg!(windows) {
        format!("http://{THEME_SCHEME}.localhost/")
    } else {
        format!("{THEME_SCHEME}://localhost/")
    };
    match url::Url::parse(&base) {
        Ok(mut url) => {
            if let Ok(mut segments) = url.path_segments_mut() {
                segments.pop_if_empty().push(source.key()).push(name).push("");
            }
            url.to_string()
        }
        Err(_) => base,
    }
}

/// Points `url(...)` references to the theme's own files (images, fonts) at
/// the theme folder; web addresses and `data:` stay as they are.
fn rewrite_urls(css: &str, base: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = find_ascii_ci(rest, "url(") {
        let (before, after) = rest.split_at(start + 4);
        out.push_str(before);
        let Some(end) = after.find(')') else {
            rest = after;
            break;
        };
        let inner = after[..end].trim();
        let path = inner.trim_matches(|c| c == '"' || c == '\'');
        let relative = !path.is_empty()
            && !path.contains(':')
            && !path.starts_with('/')
            && !path.starts_with('#')
            && !path.starts_with("var(");
        if relative {
            let path = path.trim_start_matches("./");
            out.push('"');
            out.push_str(base);
            out.push_str(&encode_path(path));
            out.push('"');
        } else {
            out.push_str(&after[..end]);
        }
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

fn find_ascii_ci(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

fn encode_path(path: &str) -> String {
    let mut url = match url::Url::parse("http://x/") {
        Ok(url) => url,
        Err(_) => return path.to_string(),
    };
    if let Ok(mut segments) = url.path_segments_mut() {
        segments.clear().extend(path.split('/'));
    }
    url.path().trim_start_matches('/').to_string()
}

/// The theme file a request asks for: `/<source>/<theme>/<path>`, only
/// inside that theme's folder.
fn requested_file(path: &str) -> Option<(PathBuf, &'static str)> {
    let mut parts = path.trim_start_matches('/').splitn(3, '/');
    let source = Source::from_key(parts.next()?)?;
    let name = percent_decode_str(parts.next()?).decode_utf8().ok()?;
    let relative = percent_decode_str(parts.next()?).decode_utf8().ok()?;
    let dir = theme_dir(source, &name)?;
    let relative = Path::new(relative.as_ref());
    if relative.components().any(|part| !matches!(part, Component::Normal(_))) {
        return None;
    }
    let extension = relative.extension()?.to_str()?.to_ascii_lowercase();
    let content_type = FILE_TYPES.iter().find(|(ext, _)| *ext == extension)?.1;
    let file = dir.join(relative).canonicalize().ok()?;
    file.starts_with(dir.canonicalize().ok()?).then_some((file, content_type))
}

pub fn serve(request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let found = requested_file(request.uri().path()).and_then(|(file, content_type)| {
        let meta = fs::metadata(&file).ok()?;
        if meta.len() > MAX_FILE_BYTES {
            return None;
        }
        Some((content_type, fs::read(file).ok()?))
    });
    let response = match found {
        Some((content_type, bytes)) => Response::builder()
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CACHE_CONTROL, "no-cache")
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .body(bytes),
        None => Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()),
    };
    response.unwrap_or_else(|_| Response::new(Vec::new()))
}

/// Copies a theme folder, or every theme folder inside one (a downloaded
/// copy of spicetify-themes, say), into SpotiLarp's Themes folder. Returns
/// the names of the themes copied.
pub fn import(from: &Path) -> Result<Vec<String>, String> {
    let own = own_dir().ok_or("Couldn't find SpotiLarp's settings folder")?;
    if from.starts_with(&own) {
        return Err("That folder is already in SpotiLarp's themes".to_string());
    }
    let themes: Vec<PathBuf> = if is_theme(from) {
        vec![from.to_path_buf()]
    } else {
        fs::read_dir(from)
            .map_err(|e| e.to_string())?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_dir()
                    && is_theme(path)
                    && path.file_name().is_some_and(|name| !name.to_string_lossy().starts_with('.'))
            })
            .collect()
    };
    if themes.is_empty() {
        return Err("No themes there: a theme is a folder with a color.ini or user.css".to_string());
    }
    let mut budget = MAX_IMPORT_BYTES;
    let mut imported = Vec::new();
    for theme in themes {
        let Some(name) = theme.file_name().map(|name| name.to_string_lossy().into_owned()) else {
            continue;
        };
        if !safe_name(&name) {
            continue;
        }
        copy_dir(&theme, &own.join(&name), &mut budget)?;
        imported.push(name);
    }
    Ok(imported)
}

fn copy_dir(from: &Path, to: &Path, budget: &mut u64) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(from).map_err(|e| e.to_string())?.flatten() {
        let name = entry.file_name();
        // Leaves out .git and the like.
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let target = to.join(&name);
        if kind.is_dir() {
            copy_dir(&entry.path(), &target, budget)?;
        } else if kind.is_file() {
            let size = entry.metadata().map(|meta| meta.len()).unwrap_or(u64::MAX);
            if size > MAX_FILE_BYTES {
                continue;
            }
            if size > *budget {
                return Err("Those themes are too big to copy (over 200 MB)".to_string());
            }
            *budget -= size;
            fs::copy(entry.path(), &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_spicetify_color_schemes() {
        let text = "; a comment\n[Wealthy]\n; Green on grey\ntext = 8bc34a\nmain               = 202020 ; dark\nbutton = #FFF\nbad = ${xrdb:color1}\n\n[Empty]\n[Cherry]\nText = D98BA1\n";
        let schemes = parse_color_ini(text);
        assert_eq!(schemes.len(), 2);
        assert_eq!(schemes[0].name, "Wealthy");
        assert_eq!(schemes[0].colors.get("text").map(String::as_str), Some("8bc34a"));
        assert_eq!(schemes[0].colors.get("main").map(String::as_str), Some("202020"));
        assert_eq!(schemes[0].colors.get("button").map(String::as_str), Some("ffffff"));
        assert!(!schemes[0].colors.contains_key("bad"));
        assert_eq!(schemes[1].name, "Cherry");
        assert_eq!(schemes[1].colors.get("text").map(String::as_str), Some("d98ba1"));
    }

    #[test]
    fn points_theme_files_at_the_theme_folder() {
        let base = "http://themefile.localhost/own/My%20Theme/";
        let css = r#"a { background: url("images/bg.png"); } b { src: url(./fonts/a b.woff2) } c { background: URL('https://x.y/z.png') } d { mask: url(data:image/png;base64,AA==) } e { fill: url(#grad) }"#;
        let out = rewrite_urls(css, base);
        assert!(out.contains(r#"url("http://themefile.localhost/own/My%20Theme/images/bg.png")"#));
        assert!(out.contains(r#"url("http://themefile.localhost/own/My%20Theme/fonts/a%20b.woff2")"#));
        assert!(out.contains("URL('https://x.y/z.png')"));
        assert!(out.contains("url(data:image/png;base64,AA==)"));
        assert!(out.contains("url(#grad)"));
    }

    #[test]
    fn theme_urls_name_the_source_and_folder() {
        let base = file_url_base(Source::Spicetify, "Sleek");
        assert!(base.ends_with("/spicetify/Sleek/"));
        assert_eq!(parse_id("spicetify:Sleek"), Some((Source::Spicetify, "Sleek")));
        assert_eq!(parse_id("elsewhere:Sleek"), None);
    }

    #[test]
    fn refuses_paths_out_of_a_theme() {
        assert!(!safe_name(".."));
        assert!(!safe_name("a/b"));
        assert!(!safe_name("C:"));
        assert!(requested_file("/own/../../config.json/x.png").is_none());
        assert!(requested_file("/own/Theme/../../secret.png").is_none());
    }
}
