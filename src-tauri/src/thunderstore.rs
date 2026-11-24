use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, SystemTime};

pub const COMMUNITY_PACKAGES: &str = "https://thunderstore.io/c/bonelab/api/v1/package/";
const EXPERIMENTAL_PACKAGE: &str = "https://thunderstore.io/api/experimental/package";
const CACHE_MAX_AGE: Duration = Duration::from_secs(60 * 30);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApiVersion {
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub version_number: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub download_url: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub date_created: String,
    #[serde(default)]
    pub website_url: String,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub file_size: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApiPackage {
    pub name: String,
    #[serde(default)]
    pub full_name: String,
    pub owner: String,
    #[serde(default)]
    pub package_url: String,
    #[serde(default)]
    pub donation_link: Option<String>,
    #[serde(default)]
    pub date_created: String,
    #[serde(default)]
    pub date_updated: String,
    #[serde(default)]
    pub rating_score: Option<i64>,
    #[serde(default)]
    pub is_deprecated: bool,
    #[serde(default)]
    pub categories: Vec<serde_json::Value>,
    #[serde(default)]
    pub versions: Vec<ApiVersion>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PackageSummary {
    pub namespace: String,
    pub name: String,
    pub full_name: String,
    pub description: String,
    pub icon: String,
    pub version: String,
    pub downloads: u64,
    pub rating: i64,
    pub updated: String,
    pub package_url: String,
    pub version_count: usize,
    pub deprecated: bool,
    pub website_url: String,
    pub size: u64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PackageVersion {
    pub version: String,
    pub downloads: u64,
    pub size: u64,
    pub created: String,
    pub description: String,
    pub download_url: String,
    pub dependencies: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PackageDetail {
    pub namespace: String,
    pub name: String,
    pub owner: String,
    pub description: String,
    pub icon: String,
    pub package_url: String,
    pub website_url: String,
    pub donation_link: String,
    pub downloads: u64,
    pub rating: i64,
    pub first_uploaded: String,
    pub updated: String,
    pub deprecated: bool,
    pub readme: String,
    pub changelog: String,
    pub versions: Vec<PackageVersion>,
}

pub fn latest_version(package: &ApiPackage) -> Option<&ApiVersion> {
    package
        .versions
        .iter()
        .max_by(|a, b| a.date_created.cmp(&b.date_created))
        .or_else(|| package.versions.first())
}

pub fn total_downloads(package: &ApiPackage) -> u64 {
    package.versions.iter().map(|v| v.downloads).sum()
}

pub fn summarize(package: &ApiPackage) -> PackageSummary {
    let latest = latest_version(package);
    let latest_website = latest.map(|v| v.website_url.clone()).unwrap_or_default();
    PackageSummary {
        namespace: package.owner.clone(),
        name: package.name.clone(),
        full_name: if package.full_name.is_empty() {
            format!("{}-{}", package.owner, package.name)
        } else {
            package.full_name.clone()
        },
        description: latest.map(|v| v.description.clone()).unwrap_or_default(),
        icon: latest.map(|v| v.icon.clone()).unwrap_or_default(),
        version: latest.map(|v| v.version_number.clone()).unwrap_or_default(),
        downloads: total_downloads(package),
        rating: package.rating_score.unwrap_or(0),
        updated: package.date_updated.clone(),
        package_url: if package.package_url.is_empty() {
            format!(
                "https://thunderstore.io/c/bonelab/p/{}/{}/",
                package.owner, package.name
            )
        } else {
            package.package_url.clone()
        },
        version_count: package.versions.len(),
        deprecated: package.is_deprecated,
        website_url: latest_website,
        size: latest.and_then(|v| v.file_size).unwrap_or(0),
    }
}

fn cache_fresh(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .map(|age| age < CACHE_MAX_AGE)
        .unwrap_or(false)
}

async fn fetch_community(http: &reqwest::Client) -> Result<Vec<ApiPackage>, String> {
    let response = http
        .get(COMMUNITY_PACKAGES)
        .send()
        .await
        .map_err(|e| format!("Could not reach Thunderstore: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Thunderstore responded with HTTP {}.",
            response.status()
        ));
    }
    let mut packages: Vec<ApiPackage> = response
        .json()
        .await
        .map_err(|e| format!("Unexpected Thunderstore response: {e}"))?;
    packages.retain(|p| !p.versions.is_empty());
    Ok(packages)
}

/// Returns the BONELAB package index, using an in-memory and on-disk cache.
pub async fn load(state: &AppState, force: bool) -> Result<Vec<ApiPackage>, String> {
    if !force {
        if let Some(packages) = state.packages.lock().unwrap().clone() {
            return Ok(packages);
        }
        if cache_fresh(&state.cache_path) {
            if let Ok(text) = std::fs::read_to_string(&state.cache_path) {
                if let Ok(packages) = serde_json::from_str::<Vec<ApiPackage>>(&text) {
                    *state.packages.lock().unwrap() = Some(packages.clone());
                    return Ok(packages);
                }
            }
        }
    }

    match fetch_community(&state.http).await {
        Ok(packages) => {
            *state.packages.lock().unwrap() = Some(packages.clone());
            if let Ok(text) = serde_json::to_string(&packages) {
                let _ = std::fs::write(&state.cache_path, text);
            }
            Ok(packages)
        }
        Err(error) => {
            if let Ok(text) = std::fs::read_to_string(&state.cache_path) {
                if let Ok(packages) = serde_json::from_str::<Vec<ApiPackage>>(&text) {
                    *state.packages.lock().unwrap() = Some(packages.clone());
                    return Ok(packages);
                }
            }
            Err(error)
        }
    }
}

pub async fn fetch_markdown(http: &reqwest::Client, url: &str) -> String {
    let Ok(response) = http.get(url).send().await else {
        return String::new();
    };
    if !response.status().is_success() {
        return String::new();
    }
    #[derive(Deserialize)]
    struct Markdown {
        #[serde(default)]
        markdown: Option<String>,
    }
    match response.json::<Markdown>().await {
        Ok(value) => value.markdown.unwrap_or_default(),
        Err(_) => String::new(),
    }
}

pub async fn detail(
    state: &AppState,
    namespace: &str,
    name: &str,
) -> Result<PackageDetail, String> {
    let packages = load(state, false).await?;
    let package = packages
        .iter()
        .find(|p| p.owner.eq_ignore_ascii_case(namespace) && p.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| format!("{namespace}/{name} was not found on Thunderstore."))?;
    let latest = latest_version(package).ok_or("This package has no versions.")?;

    let version = latest.version_number.clone();
    let readme_url = format!("{EXPERIMENTAL_PACKAGE}/{namespace}/{name}/{version}/readme/");
    let changelog_url = format!("{EXPERIMENTAL_PACKAGE}/{namespace}/{name}/{version}/changelog/");
    let (readme, changelog) = futures_join(
        fetch_markdown(&state.http, &readme_url),
        fetch_markdown(&state.http, &changelog_url),
    )
    .await;

    let mut versions: Vec<PackageVersion> = package
        .versions
        .iter()
        .map(|v| PackageVersion {
            version: v.version_number.clone(),
            downloads: v.downloads,
            size: v.file_size.unwrap_or(0),
            created: v.date_created.clone(),
            description: v.description.clone(),
            download_url: v.download_url.clone(),
            dependencies: v.dependencies.clone(),
        })
        .collect();
    versions.sort_by(|a, b| b.created.cmp(&a.created));

    Ok(PackageDetail {
        namespace: package.owner.clone(),
        name: package.name.clone(),
        owner: package.owner.clone(),
        description: latest.description.clone(),
        icon: latest.icon.clone(),
        package_url: if package.package_url.is_empty() {
            format!(
                "https://thunderstore.io/c/bonelab/p/{}/{}/",
                package.owner, package.name
            )
        } else {
            package.package_url.clone()
        },
        website_url: latest.website_url.clone(),
        donation_link: package.donation_link.clone().unwrap_or_default(),
        downloads: total_downloads(package),
        rating: package.rating_score.unwrap_or(0),
        first_uploaded: package.date_created.clone(),
        updated: package.date_updated.clone(),
        deprecated: package.is_deprecated,
        readme,
        changelog,
        versions,
    })
}

async fn futures_join<A, B>(a: A, b: B) -> (String, String)
where
    A: std::future::Future<Output = String>,
    B: std::future::Future<Output = String>,
{
    (a.await, b.await)
}

pub async fn download(http: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let response = http
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Download failed with HTTP {}.", response.status()));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;
    Ok(bytes.to_vec())
}

/// "Namespace-PackageName-1.2.3" -> ("Namespace", "PackageName", "1.2.3")
pub fn parse_dependency(dependency: &str) -> Option<(String, String, String)> {
    let mut parts = dependency.splitn(3, '-');
    let namespace = parts.next()?.to_string();
    let name = parts.next()?.to_string();
    let version = parts.next()?.to_string();
    if namespace.is_empty() || name.is_empty() || version.is_empty() {
        None
    } else {
        Some((namespace, name, version))
    }
}

