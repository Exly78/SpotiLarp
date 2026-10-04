use librespot_core::session::Session;
use serde_json::{json, Value};

use super::client::SpotifyClient;
use super::library;
use super::models::{Album, ArtistDetails, ArtistPage, ExternalLink, Page, PlaylistCard, PopularTrack, TopCity, Track};
use super::pathfinder::{self, count, grouped_releases, id_from_uri, images, list, text};
use super::search;

const ALBUMS_PAGE_SIZE: u32 = 50;
const DISCOGRAPHY_PAGE_SIZE: u64 = 50;
const MAX_DISCOGRAPHY_PAGES: u64 = 20;

pub async fn get_artist(client: &mut SpotifyClient, id: &str) -> Result<ArtistDetails, String> {
    let path = format!("/artists/{id}");
    client.get_json(&path, &[]).await
}

pub async fn get_albums(client: &mut SpotifyClient, id: &str) -> Result<Vec<Album>, String> {
    let path = format!("/artists/{id}/albums");
    let page: Page<Album> = client
        .get_json(&path, &[("include_groups", "album,single"), ("limit", "50")])
        .await?;
    Ok(page.items)
}

pub async fn popular_tracks(client: &mut SpotifyClient, id: &str, name: &str) -> Result<Vec<Track>, String> {
    let query = format!("artist:\"{}\"", name.replace('"', ""));
    let tracks = search::search_tracks(client, &query).await?;
    Ok(tracks
        .into_iter()
        .filter(|track| track.artists.iter().any(|artist| artist.id == id))
        .collect())
}

pub async fn is_following(client: &mut SpotifyClient, id: &str) -> Result<bool, String> {
    library::is_saved(client, &format!("spotify:artist:{id}")).await
}

pub async fn follow(client: &mut SpotifyClient, id: &str) -> Result<(), String> {
    library::save(client, &format!("spotify:artist:{id}")).await
}

pub async fn unfollow(client: &mut SpotifyClient, id: &str) -> Result<(), String> {
    library::remove(client, &format!("spotify:artist:{id}")).await
}

/// The whole page, as Spotify's own apps load it.
pub async fn page(session: &Session, id: &str) -> Result<ArtistPage, String> {
    let data = pathfinder::query(
        session,
        &pathfinder::ARTIST_OVERVIEW,
        json!({ "uri": format!("spotify:artist:{id}"), "locale": "", "preReleaseV2": false }),
    )
    .await?;
    let artist = data
        .get("artistUnion")
        .filter(|artist| artist["__typename"] == "Artist")
        .ok_or_else(|| "Spotify doesn't have a page for this artist".to_string())?;

    let mut header_images = images(artist, "/headerImage/data");
    if header_images.is_empty() {
        header_images = images(artist, "/visuals/headerImage");
    }
    let verified = artist.pointer("/onPlatformReputationTrait/verification/isVerified") == Some(&Value::Bool(true))
        || artist.pointer("/profile/verified") == Some(&Value::Bool(true));

    Ok(ArtistPage {
        id: id.to_string(),
        name: text(artist, "/profile/name"),
        images: images(artist, "/visuals/avatarImage"),
        header_images,
        color: artist
            .pointer("/visuals/avatarImage/extractedColors/colorRaw/hex")
            .and_then(Value::as_str)
            .map(str::to_string),
        verified,
        monthly_listeners: count(artist, "/stats/monthlyListeners"),
        followers: count(artist, "/stats/followers"),
        world_rank: count(artist, "/stats/worldRank").filter(|rank| *rank > 0),
        genres: Vec::new(),
        biography: artist
            .pointer("/profile/biography/text")
            .and_then(Value::as_str)
            .filter(|bio| !bio.trim().is_empty())
            .map(str::to_string),
        gallery: list(artist, "/visuals/gallery/items")
            .iter()
            .map(|item| images(item, ""))
            .filter(|sources| !sources.is_empty())
            .collect(),
        top_cities: list(artist, "/stats/topCities/items")
            .iter()
            .map(|city| TopCity {
                city: text(city, "/city"),
                country: text(city, "/country"),
                listeners: count(city, "/numberOfListeners").unwrap_or(0),
            })
            .collect(),
        external_links: list(artist, "/profile/externalLinks/items")
            .iter()
            .map(|link| ExternalLink {
                name: text(link, "/name"),
                url: text(link, "/url"),
            })
            .filter(|link| link.url.starts_with("https://") || link.url.starts_with("http://"))
            .collect(),
        following: artist.get("saved").and_then(Value::as_bool),
        top_tracks: list(artist, "/discography/topTracks/items")
            .iter()
            .filter_map(|item| {
                let track = item.get("track")?;
                Some(PopularTrack {
                    track: pathfinder::track(track)?,
                    playcount: count(track, "/playcount"),
                    explicit: pathfinder::is_explicit(track),
                })
            })
            .collect(),
        latest_release: artist.pointer("/discography/latest").and_then(pathfinder::album),
        popular_releases: list(artist, "/discography/popularReleasesAlbums/items")
            .iter()
            .filter_map(pathfinder::album)
            .collect(),
        albums: grouped_releases(artist, "/discography/albums"),
        singles: grouped_releases(artist, "/discography/singles"),
        compilations: grouped_releases(artist, "/discography/compilations"),
        album_count: release_count(artist, "albums"),
        single_count: release_count(artist, "singles"),
        compilation_count: release_count(artist, "compilations"),
        related_artists: list(artist, "/relatedContent/relatedArtists/items")
            .iter()
            .filter_map(related_artist)
            .collect(),
        appears_on: grouped_releases(artist, "/relatedContent/appearsOn"),
        featuring: playlist_cards(artist, "/relatedContent/featuringV2/items"),
        discovered_on: playlist_cards(artist, "/relatedContent/discoveredOnV2/items"),
        playlists: playlist_cards(artist, "/profile/playlistsV2/items"),
    })
}

/// What the Web API still offers: name, photo, genres and releases, with
/// "Popular" approximated by a search since the top tracks endpoint is gone.
pub async fn basic_page(client: &mut SpotifyClient, id: &str) -> Result<ArtistPage, String> {
    let artist = get_artist(client, id).await?;
    let releases = get_albums(client, id).await.unwrap_or_default();
    let top_tracks = popular_tracks(client, id, &artist.name).await.unwrap_or_default();
    let following = is_following(client, id).await.ok();
    let (singles, albums): (Vec<Album>, Vec<Album>) = releases
        .into_iter()
        .partition(|release| release.album_type.as_deref() == Some("single"));
    Ok(ArtistPage {
        id: artist.id,
        name: artist.name,
        images: artist.images,
        followers: artist.followers.map(|followers| u64::from(followers.total)),
        genres: artist.genres,
        following,
        top_tracks: top_tracks
            .into_iter()
            .map(|track| PopularTrack {
                track,
                playcount: None,
                explicit: false,
            })
            .collect(),
        album_count: albums.len() as u32,
        single_count: singles.len() as u32,
        albums,
        singles,
        ..Default::default()
    })
}

/// Every release in one group ("album", "single", "compilation"), or all of them
/// for anything else, newest first.
pub async fn discography(session: &Session, id: &str, group: &str) -> Result<Vec<Album>, String> {
    let (operation, field) = match group {
        "album" => (&pathfinder::ARTIST_DISCOGRAPHY_ALBUMS, "albums"),
        "single" => (&pathfinder::ARTIST_DISCOGRAPHY_SINGLES, "singles"),
        "compilation" => (&pathfinder::ARTIST_DISCOGRAPHY_COMPILATIONS, "compilations"),
        _ => (&pathfinder::ARTIST_DISCOGRAPHY_ALL, "all"),
    };
    let uri = format!("spotify:artist:{id}");
    let pointer = format!("/artistUnion/discography/{field}");
    let mut releases = Vec::new();
    for page in 0..MAX_DISCOGRAPHY_PAGES {
        let variables = json!({
            "uri": uri,
            "offset": page * DISCOGRAPHY_PAGE_SIZE,
            "limit": DISCOGRAPHY_PAGE_SIZE,
            "order": "DATE_DESC",
        });
        let data = pathfinder::query(session, operation, variables).await?;
        let found = grouped_releases(&data, &pointer);
        let fetched = found.len() as u64;
        releases.extend(found);
        let total = count(&data, &format!("{pointer}/totalCount")).unwrap_or(0);
        if fetched < DISCOGRAPHY_PAGE_SIZE || releases.len() as u64 >= total {
            break;
        }
    }
    Ok(releases)
}

pub async fn basic_discography(client: &mut SpotifyClient, id: &str, group: &str) -> Result<Vec<Album>, String> {
    let groups = match group {
        "album" | "single" | "compilation" => group,
        _ => "album,single,compilation",
    };
    let path = format!("/artists/{id}/albums?include_groups={groups}");
    client.get_all_pages(&path, ALBUMS_PAGE_SIZE).await
}

fn release_count(artist: &Value, group: &str) -> u32 {
    count(artist, &format!("/discography/{group}/totalCount")).unwrap_or(0) as u32
}

fn related_artist(item: &Value) -> Option<ArtistDetails> {
    let artist = item.get("data").unwrap_or(item);
    let uri = artist.get("uri")?.as_str()?;
    Some(ArtistDetails {
        id: id_from_uri(uri),
        name: text(artist, "/profile/name"),
        genres: Vec::new(),
        images: images(artist, "/visuals/avatarImage"),
        followers: None,
    })
}

fn playlist_cards(artist: &Value, pointer: &str) -> Vec<PlaylistCard> {
    list(artist, pointer)
        .iter()
        .filter_map(|item| {
            let playlist = item.get("data").unwrap_or(item);
            if playlist.get("__typename").and_then(Value::as_str).is_some_and(|kind| kind != "Playlist") {
                return None;
            }
            let uri = playlist.get("uri")?.as_str()?;
            Some(PlaylistCard {
                id: id_from_uri(uri),
                name: text(playlist, "/name"),
                description: text(playlist, "/description"),
                images: images(playlist, "/images/items/0"),
                owner: playlist
                    .pointer("/ownerV2/data/name")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            })
        })
        .collect()
}
