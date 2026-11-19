use crate::config::{SavedPlace, UserProfile};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const CLIENT_VERSION_URL: &str =
    "https://clientsettings.roblox.com/v2/client-version/WindowsStudio64";
const SETUP_CDN_BASE: &str = "https://setup.rbxcdn.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub version: String,
    pub client_version_upload: String,
    pub bootstrapper_version: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PackageItem {
    pub filename: String,
    pub checksum: String,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
}

#[derive(Deserialize)]
struct AuthUserResponse {
    id: u64,
    name: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Deserialize)]
struct GameDataItem {
    #[serde(rename = "rootPlace")]
    root_place: Option<RootPlaceItem>,
    name: String,
    description: Option<String>,
}

#[derive(Deserialize)]
struct RootPlaceItem {
    id: u64,
}

#[derive(Deserialize)]
struct GamesListResponse {
    data: Vec<GameDataItem>,
}

pub struct RobloxApi {
    client: reqwest::Client,
}

impl RobloxApi {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("RobloxStudio-LinuxLauncher/0.1")
            .build()
            .unwrap_or_default();
        Self { client }
    }

    /// Fetches the latest 64-bit Windows Studio version identifier.
    pub async fn fetch_latest_version(&self) -> Result<VersionInfo> {
        let resp = self
            .client
            .get(CLIENT_VERSION_URL)
            .send()
            .await
            .context("Failed to query Roblox client-version endpoint")?;

        let info: VersionInfo = resp
            .json()
            .await
            .context("Failed to parse version JSON response")?;

        Ok(info)
    }

    /// Fetches the manifest listing all archive components for a given version hash.
    pub async fn fetch_package_manifest(
        &self,
        version_upload: &str,
    ) -> Result<Vec<PackageItem>> {
        let manifest_url = format!("{}/{}-rbxPkgManifest.txt", SETUP_CDN_BASE, version_upload);
        let resp = self
            .client
            .get(&manifest_url)
            .send()
            .await
            .with_context(|| format!("Failed to download manifest from {}", manifest_url))?;

        let text = resp.text().await?;
        let mut packages = Vec::new();

        let mut lines = text.lines();
        if let Some(_header) = lines.next() {
            let mut iter = lines;
            while let Some(filename) = iter.next() {
                let filename = filename.trim();
                if filename.is_empty() {
                    continue;
                }
                let checksum = iter.next().unwrap_or("").trim().to_string();
                let compressed_size = iter.next().unwrap_or("0").trim().parse::<u64>().unwrap_or(0);
                let uncompressed_size = iter.next().unwrap_or("0").trim().parse::<u64>().unwrap_or(0);

                packages.push(PackageItem {
                    filename: filename.to_string(),
                    checksum,
                    compressed_size,
                    uncompressed_size,
                });
            }
        }

        Ok(packages)
    }

    /// Constructs the download URL for a specific package archive.
    pub fn package_download_url(version_upload: &str, filename: &str) -> String {
        format!("{}/{}-{}", SETUP_CDN_BASE, version_upload, filename)
    }

    /// Authenticates with Roblox using `.ROBLOSECURITY` cookie to retrieve user profile.
    pub async fn authenticate_with_cookie(&self, roblosecurity: &str) -> Result<UserProfile> {
        let resp = self
            .client
            .get("https://users.roblox.com/v1/users/authenticated")
            .header("Cookie", format!(".ROBLOSECURITY={}", roblosecurity.trim()))
            .send()
            .await
            .context("Failed to connect to Roblox authentication API")?;

        if !resp.status().is_success() {
            anyhow::bail!("Invalid or expired Roblox security cookie (status: {})", resp.status());
        }

        let user_data: AuthUserResponse = resp.json().await.context("Failed to parse auth user response")?;
        let avatar_url = self.fetch_user_avatar(user_data.id).await.ok();

        Ok(UserProfile {
            username: user_data.name,
            display_name: user_data.display_name,
            user_id: user_data.id,
            roblosecurity: roblosecurity.trim().to_string(),
            avatar_url,
        })
    }

    /// Fetches user's avatar headshot image URL.
    pub async fn fetch_user_avatar(&self, user_id: u64) -> Result<String> {
        let url = format!(
            "https://thumbnails.roblox.com/v1/users/avatar-headshot?userIds={}&size=150x150&format=Png&isCircular=true",
            user_id
        );
        let resp = self.client.get(&url).send().await?;
        #[derive(Deserialize)]
        struct AvatarItem {
            #[serde(rename = "imageUrl")]
            image_url: Option<String>,
        }
        #[derive(Deserialize)]
        struct AvatarResponse {
            data: Vec<AvatarItem>,
        }
        let data: AvatarResponse = resp.json().await?;
        data.data
            .into_iter()
            .find_map(|a| a.image_url)
            .context("Avatar image not found")
    }

    /// Initializes a Roblox Quick Login session (Returns 6-char code and privateKey).
    pub async fn create_quick_login(&self) -> Result<QuickLoginData> {
        #[derive(Deserialize)]
        struct CreateResponse {
            code: String,
            #[serde(rename = "privateKey")]
            private_key: String,
            #[serde(rename = "expirationTime")]
            expiration_time: String,
            #[serde(rename = "imagePath")]
            image_path: Option<String>,
        }

        let url = "https://apis.roblox.com/auth-token-service/v1/login/create";
        let mut resp = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            )
            .body("{}")
            .send()
            .await
            .context("Failed to create quick login challenge")?;

        if resp.status().as_u16() == 403 {
            if let Some(csrf) = resp.headers().get("x-csrf-token").and_then(|h| h.to_str().ok()) {
                resp = self
                    .client
                    .post(url)
                    .header("Content-Type", "application/json")
                    .header("x-csrf-token", csrf)
                    .header(
                        "User-Agent",
                        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
                    )
                    .body("{}")
                    .send()
                    .await
                    .context("Failed to send CSRF-retry quick login request")?;
            }
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Quick login API returned HTTP {}: {}", status, body_text);
        }

        let data: CreateResponse = resp.json().await?;
        let qr_url = data
            .image_path
            .map(|path| format!("https://apis.roblox.com/auth-token-service{}", path));

        Ok(QuickLoginData {
            code: data.code,
            private_key: data.private_key,
            expiration_time: data.expiration_time,
            qr_url,
        })
    }

    /// Checks the status of an ongoing Quick Login challenge.
    pub async fn poll_quick_login_status(&self, code: &str) -> Result<String> {
        #[derive(Serialize)]
        struct StatusPayload<'a> {
            code: &'a str,
        }
        #[derive(Deserialize)]
        struct StatusResponse {
            status: String,
        }

        let url = "https://apis.roblox.com/auth-token-service/v1/login/status";
        let mut resp = self
            .client
            .post(url)
            .json(&StatusPayload { code })
            .send()
            .await?;

        // Handle X-CSRF-Token challenge if required
        if resp.status().as_u16() == 403 {
            if let Some(csrf) = resp.headers().get("x-csrf-token").and_then(|h| h.to_str().ok()) {
                resp = self
                    .client
                    .post(url)
                    .header("x-csrf-token", csrf)
                    .json(&StatusPayload { code })
                    .send()
                    .await?;
            }
        }

        if !resp.status().is_success() {
            anyhow::bail!("Status check returned HTTP {}", resp.status());
        }

        let data: StatusResponse = resp.json().await?;
        Ok(data.status)
    }

    /// Redeems a Validated Quick Login challenge and extracts .ROBLOSECURITY from Set-Cookie.
    pub async fn redeem_quick_login(&self, code: &str, private_key: &str) -> Result<String> {
        #[derive(Serialize)]
        struct LoginPayload<'a> {
            ctype: &'a str,
            cvalue: &'a str,
            password: &'a str,
        }

        let url = "https://auth.roblox.com/v2/login";
        let payload = LoginPayload {
            ctype: "AuthToken",
            cvalue: code,
            password: private_key,
        };

        let mut resp = self.client.post(url).json(&payload).send().await?;

        // Handle X-CSRF-Token challenge
        if resp.status().as_u16() == 403 {
            if let Some(csrf) = resp.headers().get("x-csrf-token").and_then(|h| h.to_str().ok()) {
                resp = self
                    .client
                    .post(url)
                    .header("x-csrf-token", csrf)
                    .json(&payload)
                    .send()
                    .await?;
            }
        }

        if !resp.status().is_success() {
            anyhow::bail!("Quick login redeem failed with HTTP {}", resp.status());
        }

        // Search for .ROBLOSECURITY in Set-Cookie headers
        for val in resp.headers().get_all(reqwest::header::SET_COOKIE) {
            if let Ok(cookie_str) = val.to_str() {
                if let Some(pos) = cookie_str.find(".ROBLOSECURITY=") {
                    let part = &cookie_str[pos + 15..];
                    let end = part.find(';').unwrap_or(part.len());
                    let cookie_val = part[..end].trim().to_string();
                    return Ok(cookie_val);
                }
            }
        }

        anyhow::bail!("No .ROBLOSECURITY cookie found in authentication response headers")
    }

    /// Obtains a single-use rbx-authentication-ticket for Roblox Studio to log in without prompting.
    pub async fn get_auth_ticket(&self, roblosecurity: &str) -> Result<String> {
        let url = "https://auth.roblox.com/v1/authentication-ticket/";
        let mut resp = self
            .client
            .post(url)
            .header("Cookie", format!(".ROBLOSECURITY={}", roblosecurity.trim()))
            .header("Referer", "https://www.roblox.com")
            .header("Origin", "https://www.roblox.com")
            .header("Content-Type", "application/json")
            .body("{}")
            .send()
            .await?;

        if resp.status().as_u16() == 403 {
            if let Some(csrf) = resp.headers().get("x-csrf-token").and_then(|h| h.to_str().ok()) {
                resp = self
                    .client
                    .post(url)
                    .header("Cookie", format!(".ROBLOSECURITY={}", roblosecurity.trim()))
                    .header("x-csrf-token", csrf)
                    .header("Referer", "https://www.roblox.com")
                    .header("Origin", "https://www.roblox.com")
                    .header("Content-Type", "application/json")
                    .body("{}")
                    .send()
                    .await?;
            }
        }

        if let Some(ticket) = resp
            .headers()
            .get("rbx-authentication-ticket")
            .and_then(|h| h.to_str().ok())
        {
            return Ok(ticket.to_string());
        }

        anyhow::bail!("Failed to retrieve rbx-authentication-ticket (HTTP {})", resp.status())
    }

    /// Fetches user's created places / games.
    pub async fn fetch_user_places(&self, user_id: u64, roblosecurity: &str) -> Result<Vec<SavedPlace>> {
        let url = format!("https://games.roblox.com/v2/users/{}/games?sortOrder=Desc&limit=15", user_id);
        let resp = self
            .client
            .get(&url)
            .header("Cookie", format!(".ROBLOSECURITY={}", roblosecurity.trim()))
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Failed to fetch user games list");
        }

        let list: GamesListResponse = resp.json().await?;
        let places = list
            .data
            .into_iter()
            .filter_map(|g| {
                let place_id = g.root_place?.id;
                Some(SavedPlace {
                    name: g.name,
                    place_id,
                    description: g.description.unwrap_or_else(|| "Roblox Experience".to_string()),
                })
            })
            .collect();

        Ok(places)
    }

    /// Fetches trending Roblox experiences with thumbnail URLs.
    pub async fn fetch_trending_experiences(&self) -> Result<Vec<TrendingExperience>> {
        let places = vec![
            ("Blox Fruits", 2753915549u64, "Trending Anime Action RPG", "450K+ Playing"),
            ("Brookhaven RP", 4924922222u64, "Top Roleplay Community Hub", "380K+ Playing"),
            ("DOORS 👁️", 6516141723u64, "Atmospheric Horror & Puzzle", "45K+ Playing"),
            ("Rivals FPS", 17625359962u64, "High-Octane Competitive FPS", "80K+ Playing"),
            ("Blade Ball", 13772394625u64, "Fast-Paced Deflection Arena", "60K+ Playing"),
            ("Pet Simulator 99", 8737899170u64, "Economy & Creature Collecting", "75K+ Playing"),
        ];

        let place_ids_csv = places
            .iter()
            .map(|(_, id, _, _)| id.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let thumb_url = format!(
            "https://thumbnails.roblox.com/v1/places/gameicons?placeIds={}&size=512x512&format=Png&isCircular=false",
            place_ids_csv
        );

        let mut thumb_map = std::collections::HashMap::new();
        if let Ok(resp) = self.client.get(&thumb_url).send().await {
            #[derive(Deserialize)]
            struct ThumbItem {
                #[serde(rename = "targetId")]
                target_id: u64,
                #[serde(rename = "imageUrl")]
                image_url: Option<String>,
            }
            #[derive(Deserialize)]
            struct ThumbResponse {
                data: Vec<ThumbItem>,
            }
            if let Ok(data) = resp.json::<ThumbResponse>().await {
                for item in data.data {
                    if let Some(url) = item.image_url {
                        thumb_map.insert(item.target_id, url);
                    }
                }
            }
        }

        let result = places
            .into_iter()
            .map(|(name, id, desc, playing)| {
                let thumb = thumb_map
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| "https://tr.rbxcdn.com/180DAY-cc821ae6198f8ae795ba7dfdf003b879/768/432/Image/Webp/noFilter".to_string());
                TrendingExperience {
                    name: name.to_string(),
                    place_id: id,
                    description: desc.to_string(),
                    thumbnail_url: thumb,
                    playing: playing.to_string(),
                }
            })
            .collect();

        Ok(result)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingExperience {
    pub name: String,
    pub place_id: u64,
    pub description: String,
    pub thumbnail_url: String,
    pub playing: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickLoginData {
    pub code: String,
    pub private_key: String,
    pub expiration_time: String,
    pub qr_url: Option<String>,
}
