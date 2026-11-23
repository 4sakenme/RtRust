//! External & custom font management for RtRustMobile (Code & UI fonts).

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::rfd;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontCategory {
    Code,
    Ui,
}

#[derive(Clone, Debug)]
pub struct FontCatalogItem {
    pub id: &'static str,
    pub name: &'static str,
    pub category: FontCategory,
    pub url: &'static str,
    pub filename: &'static str,
    pub description: &'static str,
}

pub const CATALOG: &[FontCatalogItem] = &[
    FontCatalogItem {
        id: "monaspace_radon",
        name: "Monaspace Radon",
        category: FontCategory::Code,
        url: "https://github.com/githubnext/monaspace/raw/main/fonts/otf/MonaspaceRadon-Regular.otf",
        filename: "MonaspaceRadon-Regular.otf",
        description: "Handwriting-inspired expressive monospace for Luau code.",
    },
    FontCatalogItem {
        id: "monaspace_neon",
        name: "Monaspace Neon",
        category: FontCategory::Code,
        url: "https://github.com/githubnext/monaspace/raw/main/fonts/otf/MonaspaceNeon-Regular.otf",
        filename: "MonaspaceNeon-Regular.otf",
        description: "Crisp neo-grotesque monospace with clear punctuation.",
    },
    FontCatalogItem {
        id: "jetbrains_mono",
        name: "JetBrains Mono",
        category: FontCategory::Code,
        url: "https://github.com/JetBrains/JetBrainsMono/raw/master/fonts/ttf/JetBrainsMono-Regular.ttf",
        filename: "JetBrainsMono-Regular.ttf",
        description: "High-legibility IDE font with tall x-height.",
    },
    FontCatalogItem {
        id: "fira_code",
        name: "Fira Code",
        category: FontCategory::Code,
        url: "https://github.com/tonsky/FiraCode/raw/master/distr/ttf/FiraCode-Regular.ttf",
        filename: "FiraCode-Regular.ttf",
        description: "Clean programming monospace font.",
    },
    FontCatalogItem {
        id: "inter",
        name: "Inter",
        category: FontCategory::Ui,
        url: "https://github.com/google/fonts/raw/main/ofl/inter/Inter%5Bopsz%2Cwght%5D.ttf",
        filename: "Inter-Variable.ttf",
        description: "Modern UI font with full Vietnamese Unicode.",
    },
    FontCatalogItem {
        id: "be_vietnam_pro",
        name: "Be Vietnam Pro",
        category: FontCategory::Ui,
        url: "https://github.com/google/fonts/raw/main/ofl/bevietnampro/BeVietnamPro-Regular.ttf",
        filename: "BeVietnamPro-Regular.ttf",
        description: "Designed specifically for crisp Vietnamese UI.",
    },
    FontCatalogItem {
        id: "roboto",
        name: "Roboto",
        category: FontCategory::Ui,
        url: "https://github.com/googlefonts/roboto/raw/main/src/hinted/Roboto-Regular.ttf",
        filename: "Roboto-Regular.ttf",
        description: "Classic Android system UI font.",
    },
    FontCatalogItem {
        id: "fredoka",
        name: "Fredoka",
        category: FontCategory::Ui,
        url: "https://github.com/google/fonts/raw/main/ofl/fredoka/Fredoka%5Bwdth%2Cwght%5D.ttf",
        filename: "Fredoka-Variable.ttf",
        description: "Rounded playful Roblox-style UI font.",
    },
];

#[derive(Clone, Debug)]
pub struct CustomFontEntry {
    pub id: String,
    pub name: String,
    pub category: FontCategory,
    pub filename: String,
}

#[derive(Clone, Debug)]
pub enum FetchState {
    Downloading(f32),
    Failed(String),
}

struct FontState {
    initialized: bool,
    active_code: Option<String>,
    active_ui: Option<String>,
    custom_fonts: Vec<CustomFontEntry>,
    downloads: HashMap<String, FetchState>,
    last_message: Option<String>,
}

impl Default for FontState {
    fn default() -> Self {
        Self {
            initialized: false,
            active_code: None,
            active_ui: None,
            custom_fonts: Vec::new(),
            downloads: HashMap::new(),
            last_message: None,
        }
    }
}

static STATE: OnceLock<Mutex<FontState>> = OnceLock::new();

fn state() -> &'static Mutex<FontState> {
    STATE.get_or_init(|| Mutex::new(FontState::default()))
}

pub fn fonts_dir() -> PathBuf {
    let dir = rustudio_core::paths::data().join("rtrustmobile").join("fonts");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn config_path() -> PathBuf {
    fonts_dir().join("font_config.cfg")
}

fn load_config_into(st: &mut FontState) {
    let Ok(raw) = std::fs::read_to_string(config_path()) else {
        return;
    };
    st.custom_fonts.clear();
    for line in raw.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("code=") {
            st.active_code = (!v.is_empty()).then(|| v.to_owned());
        } else if let Some(v) = line.strip_prefix("ui=") {
            st.active_ui = (!v.is_empty()).then(|| v.to_owned());
        } else if let Some(v) = line.strip_prefix("custom=") {
            let mut parts = v.splitn(4, '|');
            if let (Some(id), Some(name), Some(cat), Some(file)) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            {
                let category = if cat == "code" {
                    FontCategory::Code
                } else {
                    FontCategory::Ui
                };
                st.custom_fonts.push(CustomFontEntry {
                    id: id.to_owned(),
                    name: name.to_owned(),
                    category,
                    filename: file.to_owned(),
                });
            }
        }
    }
}

fn save_config_from(st: &FontState) {
    let mut out = String::new();
    out.push_str("code=");
    if let Some(c) = &st.active_code {
        out.push_str(c);
    }
    out.push('\n');
    out.push_str("ui=");
    if let Some(u) = &st.active_ui {
        out.push_str(u);
    }
    out.push('\n');
    for c in &st.custom_fonts {
        let cat = if c.category == FontCategory::Code { "code" } else { "ui" };
        out.push_str(&format!("custom={}|{}|{}|{}\n", c.id, c.name, cat, c.filename));
    }
    let _ = std::fs::write(config_path(), out);
}

fn resolve_filename(st: &FontState, id: &str) -> Option<(String, FontCategory)> {
    if let Some(item) = CATALOG.iter().find(|i| i.id == id) {
        return Some((item.filename.to_owned(), item.category));
    }
    st.custom_fonts
        .iter()
        .find(|c| c.id == id)
        .map(|c| (c.filename.clone(), c.category))
}

pub fn is_cached(filename: &str) -> bool {
    let p = fonts_dir().join(filename);
    std::fs::metadata(&p).map(|m| m.len() > 1024).unwrap_or(false)
}

pub fn active_code_font_name() -> String {
    let Ok(st) = state().lock() else {
        return "Default".to_owned();
    };
    let Some(id) = st.active_code.as_deref() else {
        return "Default".to_owned();
    };
    if let Some(item) = CATALOG.iter().find(|i| i.id == id) {
        return item.name.to_owned();
    }
    st.custom_fonts
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "Default".to_owned())
}

pub fn available_code_fonts() -> Vec<(String, String, bool)> {
    let mut out = vec![("default".to_owned(), "Default (Built-in)".to_owned(), true)];
    let Ok(st) = state().lock() else {
        return out;
    };
    for item in CATALOG.iter().filter(|i| i.category == FontCategory::Code) {
        out.push((
            item.id.to_owned(),
            item.name.to_owned(),
            is_cached(item.filename),
        ));
    }
    for c in st.custom_fonts.iter().filter(|c| c.category == FontCategory::Code) {
        out.push((c.id.clone(), c.name.clone(), is_cached(&c.filename)));
    }
    out
}

pub fn init_on_startup(ctx: &egui::Context) {
    let should_apply = if let Ok(mut st) = state().lock() {
        if st.initialized {
            false
        } else {
            st.initialized = true;
            load_config_into(&mut st);
            true
        }
    } else {
        false
    };
    if should_apply {
        apply_fonts(ctx);
    }
}

pub fn set_active_font(id: Option<&str>, category: FontCategory, ctx: &egui::Context) {
    if let Ok(mut st) = state().lock() {
        match category {
            FontCategory::Code => st.active_code = id.map(ToOwned::to_owned),
            FontCategory::Ui => st.active_ui = id.map(ToOwned::to_owned),
        }
        save_config_from(&st);
    }
    apply_fonts(ctx);
}

pub fn apply_fonts(ctx: &egui::Context) {
    let (code_file, ui_file) = if let Ok(st) = state().lock() {
        let cf = st
            .active_code
            .as_deref()
            .and_then(|id| resolve_filename(&st, id))
            .map(|(f, _)| f);
        let uf = st
            .active_ui
            .as_deref()
            .and_then(|id| resolve_filename(&st, id))
            .map(|(f, _)| f);
        (cf, uf)
    } else {
        (None, None)
    };

    let mut fonts = egui::FontDefinitions::default();
    let bundled_key = "rtrust_vietnamese_fallback".to_owned();
    fonts.font_data.insert(
        bundled_key.clone(),
        std::sync::Arc::new(egui::FontData::from_static(crate::platform::BUNDLED_VIETNAMESE_FONT)),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push(bundled_key.clone());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push(bundled_key);
    let dir = fonts_dir();

    if let Some(ref file) = ui_file {
        let path = dir.join(file);
        if let Ok(bytes) = std::fs::read(&path) {
            let key = "rtrust_custom_ui".to_owned();
            fonts
                .font_data
                .insert(key.clone(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, key);
            sync_to_content_fonts(&path, file);
        }
    }

    if let Some(ref file) = code_file {
        let path = dir.join(file);
        if let Ok(bytes) = std::fs::read(&path) {
            let key = "rtrust_custom_code".to_owned();
            fonts
                .font_data
                .insert(key.clone(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, key);
            sync_to_content_fonts(&path, file);
        }
    }

    ctx.set_fonts(fonts);
    // Keep ti icons registered in proportional/monospace fallback.
    rustudio_engine::ti::install(ctx);
}

fn sync_to_content_fonts(src: &Path, filename: &str) {
    let app_content_fonts = rustudio_core::paths::data()
        .join("rtrustmobile")
        .join("content")
        .join("fonts");
    let _ = std::fs::create_dir_all(&app_content_fonts);
    let _ = std::fs::copy(src, app_content_fonts.join(filename));
    let local_content_fonts = PathBuf::from("content").join("fonts");
    if local_content_fonts.exists() {
        let _ = std::fs::copy(src, local_content_fonts.join(filename));
    }
}

pub fn start_fetch(item: &'static FontCatalogItem, ctx: egui::Context) {
    if let Ok(mut st) = state().lock() {
        if matches!(st.downloads.get(item.id), Some(FetchState::Downloading(_))) {
            return;
        }
        st.downloads
            .insert(item.id.to_owned(), FetchState::Downloading(0.05));
    }
    std::thread::spawn(move || {
        let res = download_font(item.url, &fonts_dir().join(item.filename), |p| {
            if let Ok(mut st) = state().lock() {
                st.downloads
                    .insert(item.id.to_owned(), FetchState::Downloading(p));
            }
            ctx.request_repaint();
        });
        if let Ok(mut st) = state().lock() {
            match res {
                Ok(()) => {
                    st.downloads.remove(item.id);
                    st.last_message = Some(format!("Downloaded {}", item.name));
                }
                Err(e) => {
                    st.downloads
                        .insert(item.id.to_owned(), FetchState::Failed(e));
                }
            }
        }
        ctx.request_repaint();
    });
}

fn download_font(
    url: &str,
    dest: &Path,
    mut on_progress: impl FnMut(f32),
) -> Result<(), String> {
    let resp = rustudio_account::android_net::default_agent()
        .get(url)
        .call()
        .map_err(|e| format!("Network error: {e}"))?;
    let total_len: u64 = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut reader = resp.into_body().into_reader();
    let mut buf = Vec::new();
    let mut chunk = [0u8; 16384];
    loop {
        let n = reader
            .read(&mut chunk)
            .map_err(|e| format!("Read error: {e}"))?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if total_len > 0 {
            on_progress((buf.len() as f32 / total_len as f32).clamp(0.05, 0.99));
        }
    }
    if buf.len() < 1024 {
        return Err("Downloaded file is too small".to_owned());
    }
    let tmp = dest.with_extension("tmp");
    std::fs::write(&tmp, &buf).map_err(|e| format!("Write error: {e}"))?;
    std::fs::rename(&tmp, dest).map_err(|e| format!("Rename error: {e}"))?;
    Ok(())
}

pub fn pick_and_import_local_font(category: FontCategory, ctx: egui::Context) {
    std::thread::spawn(move || {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Font (.ttf, .otf)", &["ttf", "otf"])
            .pick_file()
        else {
            return;
        };
        let Some(filename) = path.file_name().and_then(|s| s.to_str()).map(ToOwned::to_owned) else {
            return;
        };
        let dest = fonts_dir().join(&filename);
        if std::fs::copy(&path, &dest).is_err() {
            return;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Custom Font")
            .to_owned();
        let id = format!(
            "custom_{}_{}",
            if category == FontCategory::Code { "code" } else { "ui" },
            stem.to_lowercase().replace(' ', "_")
        );
        if let Ok(mut st) = state().lock() {
            if !st.custom_fonts.iter().any(|c| c.id == id) {
                st.custom_fonts.push(CustomFontEntry {
                    id: id.clone(),
                    name: stem.clone(),
                    category,
                    filename,
                });
            }
            match category {
                FontCategory::Code => st.active_code = Some(id),
                FontCategory::Ui => st.active_ui = Some(id),
            }
            st.last_message = Some(format!("Imported {stem}"));
            save_config_from(&st);
        }
        apply_fonts(&ctx);
        ctx.request_repaint();
    });
}

pub fn settings_ui(ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    ui.add_space(6.0);
    ui.heading("Fonts & Typography");
    ui.label(
        egui::RichText::new("Download code/UI fonts once and cache offline, or import local .ttf/.otf files.")
            .size(12.0)
            .color(egui::Color32::from_gray(170)),
    );
    ui.add_space(8.0);

    // Live Preview
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(28, 30, 36))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("Live Preview")
                    .size(11.0)
                    .color(egui::Color32::from_gray(160)),
            );
            ui.label(
                egui::RichText::new("local function test() -- Tiếng Việt OK")
                    .font(egui::FontId::monospace(14.0))
                    .color(egui::Color32::from_rgb(130, 215, 255)),
            );
            ui.label(
                egui::RichText::new("UI Label: Xin chào RtRustMobile — Tiếng Việt sắc nét 100%")
                    .font(egui::FontId::proportional(13.5))
                    .color(egui::Color32::from_gray(235)),
            );
        });

    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        if ui.button("+ Import Code Font (.ttf/.otf)").clicked() {
            pick_and_import_local_font(FontCategory::Code, ctx.clone());
        }
        if ui.button("+ Import UI Font (.ttf/.otf)").clicked() {
            pick_and_import_local_font(FontCategory::Ui, ctx.clone());
        }
        if ui.button("Reset Defaults").clicked() {
            set_active_font(None, FontCategory::Code, &ctx);
            set_active_font(None, FontCategory::Ui, &ctx);
        }
    });

    let (active_code, active_ui, custom_fonts, downloads, last_msg) = if let Ok(st) = state().lock() {
        (
            st.active_code.clone(),
            st.active_ui.clone(),
            st.custom_fonts.clone(),
            st.downloads.clone(),
            st.last_message.clone(),
        )
    } else {
        return;
    };

    if let Some(msg) = last_msg {
        ui.add_space(4.0);
        ui.label(egui::RichText::new(msg).size(11.5).color(egui::Color32::from_rgb(120, 220, 145)));
    }

    for (heading, cat, active_id) in [
        ("Code Fonts (Script Editor)", FontCategory::Code, active_code.as_deref()),
        ("UI & Vietnamese Fonts", FontCategory::Ui, active_ui.as_deref()),
    ] {
        ui.add_space(10.0);
        ui.label(egui::RichText::new(heading).strong().size(13.5));
        ui.separator();

        for item in CATALOG.iter().filter(|i| i.category == cat) {
            let cached = is_cached(item.filename);
            let is_active = active_id == Some(item.id);
            let dl = downloads.get(item.id).cloned();

            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(item.name).strong().size(13.0));
                            ui.label(
                                egui::RichText::new(item.description)
                                    .size(11.0)
                                    .color(egui::Color32::from_gray(165)),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if is_active {
                                ui.label(
                                    egui::RichText::new("✓ Active")
                                        .strong()
                                        .color(egui::Color32::from_rgb(110, 220, 130)),
                                );
                            } else if cached {
                                if ui.button("Apply").clicked() {
                                    set_active_font(Some(item.id), cat, &ctx);
                                }
                            } else if let Some(FetchState::Downloading(p)) = dl {
                                ui.add(
                                    egui::ProgressBar::new(p)
                                        .desired_width(110.0)
                                        .text(format!("{}%", (p * 100.0) as u32)),
                                );
                            } else {
                                if ui.button("⬇ Fetch").clicked() {
                                    start_fetch(item, ctx.clone());
                                }
                                if let Some(FetchState::Failed(err)) = dl {
                                    ui.label(
                                        egui::RichText::new(err)
                                            .size(10.5)
                                            .color(egui::Color32::from_rgb(255, 115, 115)),
                                    );
                                }
                            }
                        });
                    });
                });
        }

        for custom in custom_fonts.iter().filter(|c| c.category == cat) {
            let is_active = active_id == Some(custom.id.as_str());
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{} (Local)", custom.name)).strong().size(13.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if is_active {
                                ui.label(
                                    egui::RichText::new("✓ Active")
                                        .strong()
                                        .color(egui::Color32::from_rgb(110, 220, 130)),
                                );
                            } else if ui.button("Apply").clicked() {
                                set_active_font(Some(&custom.id), cat, &ctx);
                            }
                        });
                    });
                });
        }
    }
}
