use crate::util;
use serde::Deserialize;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const LATEST_RELEASE: &str = "https://api.github.com/repos/LavaGang/MelonLoader/releases/latest";
const MARKER: &str = ".blmp-version";
const CACHE_TTL: Duration = Duration::from_secs(60 * 30);

type LatestCache = Option<(Instant, String, String)>;

fn cache() -> &'static Mutex<LatestCache> {
    static CACHE: OnceLock<Mutex<LatestCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

pub async fn latest(http: &reqwest::Client) -> Result<(String, String), String> {
    if let Some((fetched_at, tag, url)) = cache().lock().unwrap().clone() {
        if fetched_at.elapsed() < CACHE_TTL {
            return Ok((tag, url));
        }
    }

    let response = http
        .get(LATEST_RELEASE)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Could not reach GitHub: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "GitHub responded with HTTP {} while looking up MelonLoader.",
            response.status()
        ));
    }
    let release: Release = response
        .json()
        .await
        .map_err(|e| format!("Unexpected GitHub response: {e}"))?;
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == "MelonLoader.x64.zip")
        .ok_or("The latest MelonLoader release has no x64 archive.")?;
    let result = (release.tag_name, asset.browser_download_url.clone());
    *cache().lock().unwrap() = Some((Instant::now(), result.0.clone(), result.1.clone()));
    Ok(result)
}

pub fn installed_version(game_dir: &Path) -> Option<String> {
    if let Ok(text) = std::fs::read_to_string(game_dir.join("MelonLoader").join(MARKER)) {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    if game_dir.join("MelonLoader").join("MelonLoader.dll").exists() {
        return Some("unknown".into());
    }
    None
}

/// Installs the newest MelonLoader release into the game folder unless it is
/// already there. Returns the installed tag.
pub async fn ensure(
    http: &reqwest::Client,
    game_dir: &Path,
    force: bool,
) -> Result<String, String> {
    let (tag, url) = latest(http).await?;
    if !force {
        if let Some(current) = installed_version(game_dir) {
            if current == tag {
                return Ok(tag);
            }
        }
    }

    let response = http
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Could not download MelonLoader: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Could not download MelonLoader (HTTP {}).",
            response.status()
        ));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Could not download MelonLoader: {e}"))?;

    util::extract_zip_raw(&bytes, game_dir)?;
    let melon_dir = game_dir.join("MelonLoader");
    std::fs::create_dir_all(&melon_dir).map_err(|e| e.to_string())?;
    std::fs::write(melon_dir.join(MARKER), &tag).map_err(|e| e.to_string())?;
    if !game_dir.join("Mods").is_dir() {
        std::fs::create_dir_all(game_dir.join("Mods")).map_err(|e| e.to_string())?;
    }
    Ok(tag)
}
