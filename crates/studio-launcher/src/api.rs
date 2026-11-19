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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioUpdateStatus {
    pub has_update: bool,
    pub latest_version: String,
    pub current_version: Option<String>,
    pub launcher_update: Option<String>,
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

    /// Checks if a newer Studio version exists compared to the local versions directory.
    pub async fn check_studio_update(
        &self,
        versions_dir: &std::path::Path,
    ) -> Result<StudioUpdateStatus> {
        let latest = self.fetch_latest_version().await?;
        let latest_upload = latest.client_version_upload;

        let version_path = versions_dir.join(&latest_upload);
        let exe_path = version_path.join("RobloxStudioBeta.exe");
        let fonts_path = version_path.join("content/fonts");

        let mut current_version = None;
        if let Ok(entries) = std::fs::read_dir(versions_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() && p.join("RobloxStudioBeta.exe").exists() {
                    if let Some(name) = p.file_name() {
                        current_version = Some(name.to_string_lossy().to_string());
                        break;
                    }
                }
            }
        }

        let has_update = !(exe_path.exists() && fonts_path.exists());

        Ok(StudioUpdateStatus {
            has_update,
            latest_version: latest_upload,
            current_version,
            launcher_update: None,
        })
    }

    /// Checks GitHub releases to see if a newer launcher version is available.
    pub async fn check_github_release(&self) -> Result<Option<String>> {
        let url = "https://api.github.com/repos/4sakenme/RtRust/releases/latest";
        let resp = self
            .client
            .get(url)
            .header("User-Agent", "RtRust-Launcher")
            .send()
            .await;

        if let Ok(res) = resp {
            if res.status().is_success() {
                #[derive(Deserialize)]
                struct GhRelease {
                    tag_name: String,
                }
                if let Ok(gh) = res.json::<GhRelease>().await {
                    let current = format!("v{}", env!("CARGO_PKG_VERSION"));
                    if gh.tag_name != current && !gh.tag_name.is_empty() {
                        return Ok(Some(gh.tag_name));
                    }
                }
            }
        }
        Ok(None)
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

    /// Fetches user's created places / games across Personal, Groups, and Shared.
    pub async fn fetch_user_places(&self, user_id: u64, roblosecurity: &str) -> Result<Vec<SavedPlace>> {
        self.fetch_all_user_experiences(user_id, roblosecurity).await
    }

    /// Fetches full list of user experiences across Personal, Groups, and Shared (Team Create).
    pub async fn fetch_all_user_experiences(
        &self,
        user_id: u64,
        roblosecurity: &str,
    ) -> Result<Vec<SavedPlace>> {
        let cookie_header = format!(".ROBLOSECURITY={}", roblosecurity.trim());
        let mut places: Vec<SavedPlace> = Vec::new();
        let mut universe_ids: Vec<u64> = Vec::new();

        // 1. Personal Universes: develop.roblox.com/v1/user/universes
        let personal_url = "https://develop.roblox.com/v1/user/universes?limit=50&sortOrder=Desc";
        if let Ok(resp) = self
            .client
            .get(personal_url)
            .header("Cookie", &cookie_header)
            .send()
            .await
        {
            #[derive(Deserialize)]
            struct UnivList {
                #[serde(default)]
                data: Vec<UnivItem>,
            }
            #[derive(Deserialize)]
            struct UnivItem {
                id: u64,
                name: String,
                description: Option<String>,
                #[serde(rename = "rootPlaceId")]
                root_place_id: Option<u64>,
            }

            if let Ok(list) = resp.json::<UnivList>().await {
                for u in list.data {
                    if let Some(root_id) = u.root_place_id {
                        universe_ids.push(u.id);
                        places.push(SavedPlace {
                            name: u.name,
                            place_id: root_id,
                            description: u.description.unwrap_or_else(|| "Personal Experience".to_string()),
                            universe_id: Some(u.id),
                            thumbnail_url: None,
                            team_create_enabled: None,
                            category: "personal".to_string(),
                            local_path: None,
                        });
                    }
                }
            }
        }

        // 2. Groups Universes: groups.roblox.com/v2/users/{user_id}/groups/roles
        let groups_url = format!("https://groups.roblox.com/v2/users/{}/groups/roles", user_id);
        if let Ok(resp) = self
            .client
            .get(&groups_url)
            .header("Cookie", &cookie_header)
            .send()
            .await
        {
            #[derive(Deserialize)]
            struct GrpList {
                #[serde(default)]
                data: Vec<GrpRoleItem>,
            }
            #[derive(Deserialize)]
            struct GrpRoleItem {
                group: GrpInfo,
            }
            #[derive(Deserialize)]
            struct GrpInfo {
                id: u64,
                name: String,
            }

            if let Ok(groups_data) = resp.json::<GrpList>().await {
                for grp_item in groups_data.data.into_iter().take(8) {
                    let grp = grp_item.group;
                    let grp_univ_url = format!(
                        "https://develop.roblox.com/v1/groups/{}/universes?limit=50&sortOrder=Desc",
                        grp.id
                    );
                    if let Ok(g_resp) = self
                        .client
                        .get(&grp_univ_url)
                        .header("Cookie", &cookie_header)
                        .send()
                        .await
                    {
                        #[derive(Deserialize)]
                        struct GrpUnivList {
                            #[serde(default)]
                            data: Vec<GrpUnivItem>,
                        }
                        #[derive(Deserialize)]
                        struct GrpUnivItem {
                            id: u64,
                            name: String,
                            #[serde(rename = "rootPlaceId")]
                            root_place_id: Option<u64>,
                        }

                        if let Ok(list) = g_resp.json::<GrpUnivList>().await {
                            for u in list.data {
                                if let Some(root_id) = u.root_place_id {
                                    universe_ids.push(u.id);
                                    places.push(SavedPlace {
                                        name: u.name,
                                        place_id: root_id,
                                        description: format!("Group: {}", grp.name),
                                        universe_id: Some(u.id),
                                        thumbnail_url: None,
                                        team_create_enabled: None,
                                        category: "group".to_string(),
                                        local_path: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Shared With Me / Team Create: apis.roblox.com/universes/v1/search
        let shared_url = format!(
            "https://apis.roblox.com/universes/v1/search?creatorType=Team&creatorTargetId={}&isArchived=false&sortParam=LastUpdated&sortOrder=Desc&pageSize=50&surface=StudioStartPage",
            user_id
        );
        if let Ok(resp) = self
            .client
            .get(&shared_url)
            .header("Cookie", &cookie_header)
            .send()
            .await
        {
            #[derive(Deserialize)]
            struct SharedData {
                #[serde(default)]
                data: Vec<SharedItem>,
            }
            #[derive(Deserialize)]
            struct SharedItem {
                id: u64,
                name: String,
                #[serde(rename = "rootPlaceId")]
                root_place_id: Option<u64>,
                #[serde(rename = "creatorName")]
                creator_name: Option<String>,
                #[serde(rename = "isTeamCreateEnabled")]
                is_team_create_enabled: Option<bool>,
            }
            if let Ok(shared_list) = resp.json::<SharedData>().await {
                for s in shared_list.data {
                    if let Some(root_id) = s.root_place_id {
                        universe_ids.push(s.id);
                        places.push(SavedPlace {
                            name: s.name,
                            place_id: root_id,
                            description: format!("Shared by: {}", s.creator_name.unwrap_or_else(|| "Collaborator".to_string())),
                            universe_id: Some(s.id),
                            thumbnail_url: None,
                            team_create_enabled: s.is_team_create_enabled.or(Some(true)),
                            category: "shared".to_string(),
                            local_path: None,
                        });
                    }
                }
            }
        }

        // 4. Batch Query Team Create Status
        if !universe_ids.is_empty() {
            let mut ids_param = String::new();
            for id in universe_ids.iter().take(50) {
                if !ids_param.is_empty() {
                    ids_param.push('&');
                }
                ids_param.push_str(&format!("ids={}", id));
            }
            let tc_url = format!("https://develop.roblox.com/v1/universes/multiget/teamcreate?{}", ids_param);
            if let Ok(tc_resp) = self
                .client
                .get(&tc_url)
                .header("Cookie", &cookie_header)
                .send()
                .await
            {
                #[derive(Deserialize)]
                struct TeamCreateMulti {
                    #[serde(default)]
                    data: Vec<TeamCreateMultiItem>,
                }
                #[derive(Deserialize)]
                struct TeamCreateMultiItem {
                    id: u64,
                    #[serde(rename = "isEnabled")]
                    is_enabled: bool,
                }

                if let Ok(tc_data) = tc_resp.json::<TeamCreateMulti>().await {
                    let tc_map: std::collections::HashMap<u64, bool> = tc_data
                        .data
                        .into_iter()
                        .map(|item| (item.id, item.is_enabled))
                        .collect();
                    for p in &mut places {
                        if let Some(uid) = p.universe_id {
                            if let Some(&enabled) = tc_map.get(&uid) {
                                p.team_create_enabled = Some(enabled);
                            }
                        }
                    }
                }
            }
        }

        // 5. Batch Query 512x512 High-Res Icons and cache to ~/.cache/rtrust/thumbnails/
        if !universe_ids.is_empty() {
            let uids_csv = universe_ids
                .iter()
                .take(50)
                .map(|u| u.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let icons_url = format!(
                "https://thumbnails.roblox.com/v1/games/icons?universeIds={}&returnPolicy=PlaceHolder&size=512x512&format=Png&isCircular=false",
                uids_csv
            );
            if let Ok(icon_resp) = self.client.get(&icons_url).send().await {
                #[derive(Deserialize)]
                struct IconResponse {
                    #[serde(default)]
                    data: Vec<IconItem>,
                }
                #[derive(Deserialize)]
                struct IconItem {
                    #[serde(rename = "targetId")]
                    target_id: u64,
                    #[serde(rename = "imageUrl")]
                    image_url: Option<String>,
                }

                if let Ok(icon_data) = icon_resp.json::<IconResponse>().await {
                    let cache_dir = dirs::cache_dir()
                        .map(|c| c.join("rtrust/thumbnails"))
                        .unwrap_or_else(|| std::path::PathBuf::from("/tmp/rtrust_thumbnails"));
                    let _ = std::fs::create_dir_all(&cache_dir);

                    let icon_map: std::collections::HashMap<u64, String> = icon_data
                        .data
                        .into_iter()
                        .filter_map(|item| item.image_url.map(|url| (item.target_id, url)))
                        .collect();

                    for p in &mut places {
                        if let Some(uid) = p.universe_id {
                            if let Some(url) = icon_map.get(&uid) {
                                p.thumbnail_url = Some(url.clone());
                            }
                        }
                    }
                }
            }
        }

        // Default template fallback if empty
        if places.is_empty() {
            places = crate::config::default_saved_places();
        }

        Ok(places)
    }

    /// Fetches all sub-places belonging to a given universe.
    pub async fn fetch_universe_subplaces(
        &self,
        universe_id: u64,
        roblosecurity: &str,
    ) -> Result<Vec<SavedPlace>> {
        let cookie_header = format!(".ROBLOSECURITY={}", roblosecurity.trim());
        let url = format!(
            "https://develop.roblox.com/v1/universes/{}/places?limit=50",
            universe_id
        );
        let resp = self
            .client
            .get(&url)
            .header("Cookie", &cookie_header)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct PlacesResponse {
            #[serde(default)]
            data: Vec<SubPlaceItem>,
        }
        #[derive(Deserialize)]
        struct SubPlaceItem {
            id: u64,
            name: String,
            description: Option<String>,
        }

        let mut subplaces = Vec::new();
        if let Ok(list) = resp.json::<PlacesResponse>().await {
            for item in list.data {
                subplaces.push(SavedPlace {
                    name: item.name,
                    place_id: item.id,
                    description: item.description.unwrap_or_else(|| "Universe Sub-Place".to_string()),
                    universe_id: Some(universe_id),
                    thumbnail_url: None,
                    team_create_enabled: None,
                    category: "personal".to_string(),
                    local_path: None,
                });
            }
        }

        Ok(subplaces)
    }

    /// Downloads a place file snapshot (.rbxl) to ~/.local/share/rtrust/backups/{placeId}_{timestamp}.rbxl
    pub async fn download_place_snapshot(
        &self,
        place_id: u64,
        roblosecurity: &str,
    ) -> Result<std::path::PathBuf> {
        let url = format!("https://assetdelivery.roblox.com/v1/asset/?id={}", place_id);
        let resp = self
            .client
            .get(&url)
            .header("Cookie", format!(".ROBLOSECURITY={}", roblosecurity.trim()))
            .header("User-Agent", "Roblox/WinInet")
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("AssetDelivery failed with HTTP {}", resp.status());
        }

        let bytes = resp.bytes().await?;
        let backup_dir = dirs::data_local_dir()
            .map(|d| d.join("rtrust/backups"))
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp/rtrust_backups"));
        std::fs::create_dir_all(&backup_dir)?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let backup_path = backup_dir.join(format!("{}_{}.rbxl", place_id, timestamp));
        std::fs::write(&backup_path, bytes)?;

        tracing::info!("Saved place snapshot to: {:?}", backup_path);
        Ok(backup_path)
    }

    /// Fetches trending Roblox experiences with thumbnail URLs.
    #[allow(dead_code)]
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
