//! Studio & Code Editor Wallpaper + Pastel Theme Customization + Luminous Border Trail FX.
//!
//! Allows the user to customize:
//! 1. **Studio / Start Page Background**: Curated RtRust Pastel / Dark Studio presets,
//!    a custom RGB Color Picker, or a local device image (`Choose Image from Device`)
//!    with adjustable dim overlay opacity.
//! 2. **Code Editor Background (`Code` tab)**: Independent custom background color
//!    or local device image behind the Luau code editor + dim overlay opacity slider
//!    (`0.00` – `0.92`) so syntax-highlighted code stays crisp and readable.
//! 3. **Animated Luminous Border Trail**: Draws an animated pastel-gradient highlight
//!    traveling smoothly around active popups, modals, and floating tool panels.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use egui::{Color32, CornerRadius, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, Vec2};
use serde::{Deserialize, Serialize};

use rustudio_engine::theme;

// ── Official RtRust Studio Brand Palette (Dark Obsidian + Rust Orange-Red + Electric Cyan) ──

/// Deep Obsidian Canvas: `#111318`
pub const OBSIDIAN_CANVAS: Color32 = Color32::from_rgb(17, 19, 24);
/// Elevated Card & Header Surface: `#191C24`
pub const OBSIDIAN_CARD: Color32 = Color32::from_rgb(25, 28, 36);
/// Crisp 1px Surface Border: `#2A2F3D`
pub const OBSIDIAN_BORDER: Color32 = Color32::from_rgb(42, 47, 61);
/// Hover / Active Card Surface: `#212530`
pub const OBSIDIAN_HOVER: Color32 = Color32::from_rgb(33, 37, 48);
/// Secondary Slate Action Pill Surface: `#222632`
pub const SLATE_PILL_BG: Color32 = Color32::from_rgb(34, 38, 50);
/// Secondary Slate Action Pill Border: `#323848`
pub const SLATE_PILL_BORDER: Color32 = Color32::from_rgb(50, 56, 72);

/// Primary Brand Accent (Rust Orange-Red): `#FF5A26`
pub const RUST_ORANGE: Color32 = Color32::from_rgb(255, 90, 38);
/// Hover Rust Orange: `#FF7244`
pub const RUST_ORANGE_HOVER: Color32 = Color32::from_rgb(255, 114, 68);
/// Pressed Rust Orange: `#E04515`
pub const RUST_ORANGE_PRESSED: Color32 = Color32::from_rgb(224, 69, 21);
/// Ember Red-Orange Secondary Accent: `#FF4B2B`
pub const EMBER_RED: Color32 = Color32::from_rgb(255, 75, 43);
/// Ember Warm Orange: `#FF7A18`
pub const EMBER_AMBER: Color32 = Color32::from_rgb(255, 122, 24);
/// Subtle Electric Cyan Highlight from App Logo: `#26C6DA`
pub const LOGO_CYAN: Color32 = Color32::from_rgb(38, 198, 218);

// Alias existing theme constants to the cohesive RtRust Orange-Red & Electric Cyan brand tokens
pub const PASTEL_LAVENDER: Color32 = RUST_ORANGE;
pub const PASTEL_MINT: Color32 = LOGO_CYAN;
pub const PASTEL_SKY: Color32 = LOGO_CYAN;
pub const PASTEL_PEACH: Color32 = RUST_ORANGE_HOVER;
pub const PASTEL_ROSE: Color32 = EMBER_RED;
pub const PASTEL_GOLD: Color32 = EMBER_AMBER;

pub const PASTEL_ACCENTS: &[Color32] = &[
    RUST_ORANGE,
    EMBER_AMBER,
    LOGO_CYAN,
    RUST_ORANGE_HOVER,
    EMBER_RED,
    RUST_ORANGE,
];

#[inline]
pub fn pastel_for_index(idx: usize) -> Color32 {
    PASTEL_ACCENTS[idx % PASTEL_ACCENTS.len()]
}

/// Curated background color presets for Studio / Start Page and Code Editor.
pub const STUDIO_BG_PRESETS: &[(&str, [u8; 3], Color32)] = &[
    ("RtRust Dark Orange (Default)", [17, 19, 24], RUST_ORANGE),
    ("Crimson Rust", [24, 16, 20], Color32::from_rgb(230, 57, 70)),
    ("Ember Obsidian", [22, 18, 17], EMBER_AMBER),
    ("Cyber Cyan Slate", [15, 21, 27], LOGO_CYAN),
    ("Midnight Charcoal", [20, 22, 29], RUST_ORANGE_HOVER),
    ("Deep OLED Pitch", [10, 11, 14], RUST_ORANGE),
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WallpaperConfig {
    pub studio_bg_rgb: [u8; 3],
    pub studio_image_path: Option<String>,
    pub studio_dim_opacity: f32,
    pub code_bg_rgb: [u8; 3],
    pub code_image_path: Option<String>,
    pub code_dim_opacity: f32,
    pub luminous_trails: bool,
}

impl Default for WallpaperConfig {
    fn default() -> Self {
        Self {
            studio_bg_rgb: [17, 19, 24], // #111318 Deep Obsidian Canvas
            studio_image_path: None,
            studio_dim_opacity: 0.45,
            code_bg_rgb: [17, 19, 24],   // #111318 Deep Obsidian Canvas
            code_image_path: None,
            code_dim_opacity: 0.62,
            luminous_trails: false,
        }
    }
}

struct RuntimeState {
    cfg: WallpaperConfig,
    loaded_from_disk: bool,
    studio_tex: Option<(String, TextureHandle)>,
    code_tex: Option<(String, TextureHandle)>,
    status_msg: Option<String>,
}

fn state() -> &'static Mutex<RuntimeState> {
    static S: OnceLock<Mutex<RuntimeState>> = OnceLock::new();
    S.get_or_init(|| {
        Mutex::new(RuntimeState {
            cfg: WallpaperConfig::default(),
            loaded_from_disk: false,
            studio_tex: None,
            code_tex: None,
            status_msg: None,
        })
    })
}

fn config_file_path() -> PathBuf {
    let dir = crate::platform::app_data_dir().join("wallpapers");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("wallpaper_config.json")
}

fn ensure_loaded(st: &mut RuntimeState) {
    if st.loaded_from_disk {
        return;
    }
    st.loaded_from_disk = true;
    let p = config_file_path();
    if let Ok(txt) = std::fs::read_to_string(&p) {
        if let Ok(c) = serde_json::from_str::<WallpaperConfig>(&txt) {
            st.cfg = c;
        }
    }
}

fn save_to_disk(cfg: &WallpaperConfig) {
    let p = config_file_path();
    if let Ok(txt) = serde_json::to_string_pretty(cfg) {
        let _ = std::fs::write(p, txt);
    }
}

pub fn config() -> WallpaperConfig {
    let mut g = state().lock().unwrap();
    ensure_loaded(&mut g);
    g.cfg.clone()
}

pub fn update_config(f: impl FnOnce(&mut WallpaperConfig)) {
    let mut g = state().lock().unwrap();
    ensure_loaded(&mut g);
    f(&mut g.cfg);
    save_to_disk(&g.cfg);
}

/// Called by `platform::poll_picked_wallpaper()` when the native iOS/Android/Desktop
/// image picker finishes writing a picked image file to `app_data_dir()/wallpapers/`.
pub fn notify_picked_wallpaper(target: &str, path: PathBuf) {
    let path_str = path.display().to_string();
    let mut g = state().lock().unwrap();
    ensure_loaded(&mut g);
    if target == "code" {
        g.cfg.code_image_path = Some(path_str);
        g.code_tex = None;
        g.status_msg = Some("✓ Updated Code Editor background wallpaper".to_string());
    } else {
        g.cfg.studio_image_path = Some(path_str);
        g.studio_tex = None;
        g.status_msg = Some("✓ Updated Studio background wallpaper".to_string());
    }
    save_to_disk(&g.cfg);
}

fn load_color_image_scaled(path: &str) -> Option<egui::ColorImage> {
    let bytes = std::fs::read(path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    // Cap to 1600px max dimension so mobile VRAM stays light and fast.
    let img = if img.width() > 1600 || img.height() > 1600 {
        img.thumbnail(1600, 1600)
    } else {
        img
    };
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

/// Paints the Studio / Start Page background (solid color + optional cover wallpaper image + dim overlay).
pub fn paint_studio_background(ctx: &egui::Context, painter: &egui::Painter, rect: Rect) {
    crate::platform::poll_picked_wallpaper();
    let mut g = state().lock().unwrap();
    ensure_loaded(&mut g);
    let bg_col = Color32::from_rgb(
        g.cfg.studio_bg_rgb[0],
        g.cfg.studio_bg_rgb[1],
        g.cfg.studio_bg_rgb[2],
    );
    painter.rect_filled(rect, 0.0, bg_col);

    if let Some(path) = g.cfg.studio_image_path.clone() {
        let needs_load = g.studio_tex.as_ref().is_none_or(|(p, _)| p != &path);
        if needs_load {
            if let Some(ci) = load_color_image_scaled(&path) {
                let tex = ctx.load_texture("studio_bg_wallpaper", ci, egui::TextureOptions::LINEAR);
                g.studio_tex = Some((path.clone(), tex));
            } else {
                g.cfg.studio_image_path = None;
            }
        }
        if let Some((_, tex)) = &g.studio_tex {
            paint_cover_image(painter, rect, tex);
            let alpha = (g.cfg.studio_dim_opacity.clamp(0.0, 0.95) * 255.0).round() as u8;
            if alpha > 0 {
                painter.rect_filled(
                    rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(
                        g.cfg.studio_bg_rgb[0],
                        g.cfg.studio_bg_rgb[1],
                        g.cfg.studio_bg_rgb[2],
                        alpha,
                    ),
                );
            }
        }
    }
}

/// Paints the Code Editor background (`Code` tab) with custom color, optional device image,
/// and dim overlay opacity so Luau syntax highlighting remains crisp.
pub fn paint_code_background(ctx: &egui::Context, painter: &egui::Painter, rect: Rect) {
    crate::platform::poll_picked_wallpaper();
    let mut g = state().lock().unwrap();
    ensure_loaded(&mut g);
    let bg_col = Color32::from_rgb(
        g.cfg.code_bg_rgb[0],
        g.cfg.code_bg_rgb[1],
        g.cfg.code_bg_rgb[2],
    );
    painter.rect_filled(rect, 0.0, bg_col);

    if let Some(path) = g.cfg.code_image_path.clone() {
        let needs_load = g.code_tex.as_ref().is_none_or(|(p, _)| p != &path);
        if needs_load {
            if let Some(ci) = load_color_image_scaled(&path) {
                let tex = ctx.load_texture("code_bg_wallpaper", ci, egui::TextureOptions::LINEAR);
                g.code_tex = Some((path.clone(), tex));
            } else {
                g.cfg.code_image_path = None;
            }
        }
        if let Some((_, tex)) = &g.code_tex {
            paint_cover_image(painter, rect, tex);
            let alpha = (g.cfg.code_dim_opacity.clamp(0.0, 0.95) * 255.0).round() as u8;
            if alpha > 0 {
                painter.rect_filled(
                    rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(
                        g.cfg.code_bg_rgb[0],
                        g.cfg.code_bg_rgb[1],
                        g.cfg.code_bg_rgb[2],
                        alpha,
                    ),
                );
            }
        }
    }
}

fn paint_cover_image(painter: &egui::Painter, rect: Rect, tex: &TextureHandle) {
    let sz = tex.size_vec2();
    if sz.x <= 0.0 || sz.y <= 0.0 || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let img_aspect = sz.x / sz.y;
    let rect_aspect = rect.width() / rect.height();
    let uv = if rect_aspect > img_aspect {
        let vis_h = img_aspect / rect_aspect;
        let y0 = (1.0 - vis_h) * 0.5;
        Rect::from_min_max(Pos2::new(0.0, y0), Pos2::new(1.0, y0 + vis_h))
    } else {
        let vis_w = rect_aspect / img_aspect;
        let x0 = (1.0 - vis_w) * 0.5;
        Rect::from_min_max(Pos2::new(x0, 0.0), Pos2::new(x0 + vis_w, 1.0))
    };
    painter.image(tex.id(), rect, uv, Color32::WHITE);
}

/// Draws a clean, static 1px glass-edge rim border (`rgba(255, 255, 255, 0.14)`)
/// and subtle top specular highlight on popovers, modals, and floating panels
/// (without any rotating/traveling neon trail).
pub fn draw_luminous_border_trail(
    _ctx: &egui::Context,
    painter: &egui::Painter,
    rect: Rect,
    rounding: f32,
    _accent: Color32,
) {
    let cr = CornerRadius::same(rounding.round().clamp(0.0, 255.0) as u8);
    // Soft outer ambient drop rim
    painter.rect_stroke(
        rect.expand(1.0),
        cr,
        Stroke::new(1.5, Color32::from_black_alpha(75)),
        egui::StrokeKind::Outside,
    );
    // Crisp 1px subtle glass rim border: rgba(255, 255, 255, 0.14) (~36 alpha)
    painter.rect_stroke(
        rect,
        cr,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 36)),
        egui::StrokeKind::Inside,
    );
}

/// Ease-out-back spring curve (`t` in `0.0..=1.0`) for iOS/macOS-style popovers
/// (`scale: 0.92 -> 1.00`, `8pt` directional slide, and opacity fade).
#[inline]
pub fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let c1 = 1.45;
    let c3 = c1 + 1.0;
    let tm1 = t - 1.0;
    (1.0 + c3 * tm1 * tm1 * tm1 + c1 * tm1 * tm1).clamp(0.0, 1.08)
}

/// Staggered item reveal factor (`0.0..=1.0`) for item `index` given overall menu progress `menu_t` (`0.0..=1.0`),
/// producing a snappy ~12ms per-item sequential cascade.
#[inline]
pub fn staggered_item_alpha(menu_t: f32, index: usize) -> f32 {
    let delay = (index as f32) * 0.065;
    ((menu_t - delay) / (1.0 - delay.min(0.65))).clamp(0.0, 1.0)
}

/// Interactive settings page inside `Studio Settings -> Background & Wallpapers`
/// (and also accessible as a standalone modal from the Start Page gear button).
pub fn settings_ui(ui: &mut egui::Ui) {
    crate::platform::poll_picked_wallpaper();
    let (mut cfg, status_msg) = {
        let mut g = state().lock().unwrap();
        ensure_loaded(&mut g);
        (g.cfg.clone(), g.status_msg.clone())
    };
    let mut changed = false;

    egui::ScrollArea::vertical()
        .id_salt("wallpaper_settings_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 10.0);
            ui.label(
                RichText::new("🎨 Studio & Code Background Customization")
                    .size(16.0)
                    .strong()
                    .color(PASTEL_LAVENDER),
            );
            ui.label(
                RichText::new(
                    "Personalize the Start Page, Studio backdrop, and Luau Code Editor with curated RtRust Pastel presets, custom colors, or photos directly from your device.",
                )
                .size(12.0)
                .color(theme::TEXT_DIM),
            );

            if let Some(msg) = status_msg {
                ui.label(RichText::new(msg).size(12.0).color(PASTEL_MINT).strong());
            }

            // ── Section 1: Studio & Start Page Background ──
            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(26, 28, 36))
                .stroke(Stroke::new(1.0, PASTEL_LAVENDER.gamma_multiply(0.35)))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("1. Studio & Start Page Background")
                            .size(14.0)
                            .strong()
                            .color(PASTEL_LAVENDER),
                    );
                    ui.label(
                        RichText::new("Curated Pastel / Dark Studio Presets:")
                            .size(12.0)
                            .color(theme::TEXT_DIM),
                    );
                    ui.horizontal_wrapped(|ui| {
                        for (name, rgb, accent) in STUDIO_BG_PRESETS {
                            let active = cfg.studio_bg_rgb == *rgb && cfg.studio_image_path.is_none();
                            let btn = egui::Button::new(
                                RichText::new(*name)
                                    .size(12.0)
                                    .color(if active { Color32::BLACK } else { Color32::WHITE }),
                            )
                            .fill(if active {
                                *accent
                            } else {
                                Color32::from_rgb(rgb[0], rgb[1], rgb[2])
                            })
                            .stroke(Stroke::new(1.0, *accent))
                            .corner_radius(CornerRadius::same(14));
                            if ui.add(btn).clicked() {
                                cfg.studio_bg_rgb = *rgb;
                                changed = true;
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Custom Studio Color:");
                        if ui.color_edit_button_srgb(&mut cfg.studio_bg_rgb).changed() {
                            changed = true;
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("🖼 Choose Image from Device…")
                                        .color(Color32::BLACK)
                                        .strong(),
                                )
                                .fill(PASTEL_LAVENDER)
                                .corner_radius(CornerRadius::same(8)),
                            )
                            .clicked()
                        {
                            crate::platform::pick_wallpaper_image("studio");
                        }
                        if cfg.studio_image_path.is_some()
                            && ui
                                .add(
                                    egui::Button::new(RichText::new("✕ Clear Image").color(PASTEL_PEACH))
                                        .corner_radius(CornerRadius::same(8)),
                                )
                                .clicked()
                        {
                            cfg.studio_image_path = None;
                            changed = true;
                        }
                    });

                    if let Some(p) = &cfg.studio_image_path {
                        let fname = std::path::Path::new(p)
                            .file_name()
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_else(|| p.clone());
                        ui.label(
                            RichText::new(format!("Active Image: {fname}"))
                                .size(11.5)
                                .color(PASTEL_MINT),
                        );
                        if ui
                            .add(
                                egui::Slider::new(&mut cfg.studio_dim_opacity, 0.0..=0.90)
                                    .text("Background Dim / Overlay Opacity"),
                            )
                            .changed()
                        {
                            changed = true;
                        }
                    }
                });

            // ── Section 2: Code Editor Background (Code Tab) ──
            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(26, 28, 36))
                .stroke(Stroke::new(1.0, PASTEL_MINT.gamma_multiply(0.35)))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("2. Code Editor Background (Code Tab)")
                            .size(14.0)
                            .strong()
                            .color(PASTEL_MINT),
                    );
                    ui.horizontal_wrapped(|ui| {
                        for (name, rgb, accent) in STUDIO_BG_PRESETS {
                            let active = cfg.code_bg_rgb == *rgb && cfg.code_image_path.is_none();
                            let btn = egui::Button::new(
                                RichText::new(*name)
                                    .size(12.0)
                                    .color(if active { Color32::BLACK } else { Color32::WHITE }),
                            )
                            .fill(if active {
                                *accent
                            } else {
                                Color32::from_rgb(rgb[0], rgb[1], rgb[2])
                            })
                            .stroke(Stroke::new(1.0, *accent))
                            .corner_radius(CornerRadius::same(14));
                            if ui.add(btn).clicked() {
                                cfg.code_bg_rgb = *rgb;
                                changed = true;
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Custom Editor Color:");
                        if ui.color_edit_button_srgb(&mut cfg.code_bg_rgb).changed() {
                            changed = true;
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("🖼 Choose Image from Device…")
                                        .color(Color32::BLACK)
                                        .strong(),
                                )
                                .fill(PASTEL_MINT)
                                .corner_radius(CornerRadius::same(8)),
                            )
                            .clicked()
                        {
                            crate::platform::pick_wallpaper_image("code");
                        }
                        if cfg.code_image_path.is_some()
                            && ui
                                .add(
                                    egui::Button::new(RichText::new("✕ Clear Image").color(PASTEL_PEACH))
                                        .corner_radius(CornerRadius::same(8)),
                                )
                                .clicked()
                        {
                            cfg.code_image_path = None;
                            changed = true;
                        }
                    });

                    if ui
                        .add(
                            egui::Slider::new(&mut cfg.code_dim_opacity, 0.0..=0.90)
                                .text("Code Editor Dim / Overlay Opacity"),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    if let Some(p) = &cfg.code_image_path {
                        let fname = std::path::Path::new(p)
                            .file_name()
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_else(|| p.clone());
                        ui.label(
                            RichText::new(format!("Active Code Wallpaper: {fname}"))
                                .size(11.5)
                                .color(PASTEL_MINT),
                        );
                    }
                });

            // ── Section 3: UI Animations & Luminous Border Trail ──
            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(26, 28, 36))
                .stroke(Stroke::new(1.0, PASTEL_SKY.gamma_multiply(0.35)))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("3. Visual Polish & Trail Animations")
                            .size(14.0)
                            .strong()
                            .color(PASTEL_SKY),
                    );
                    if ui
                        .checkbox(
                            &mut cfg.luminous_trails,
                            "Enable animated luminous border trails on popups, dropdowns & tool windows",
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });
        });

    if changed {
        let mut g = state().lock().unwrap();
        if g.cfg.studio_image_path != cfg.studio_image_path {
            g.studio_tex = None;
        }
        if g.cfg.code_image_path != cfg.code_image_path {
            g.code_tex = None;
        }
        g.cfg = cfg;
        save_to_disk(&g.cfg);
    }
}

/// Renders a standalone floating modal for Wallpaper & Theme settings when opened from the Start Page gear icon.
pub fn modal_ui(ctx: &egui::Context) {
    let open_id = egui::Id::new("wallpaper_modal_open");
    let mut open = ctx.data(|d| d.get_temp::<bool>(open_id).unwrap_or(false));
    if !open {
        return;
    }
    let screen = ctx.content_rect();
    let w = (screen.width() - 24.0).clamp(280.0, 540.0);
    let h = (screen.height() - 60.0).clamp(280.0, 560.0);
    let mut win_open = true;
    let shown = egui::Window::new("🎨 Studio Settings — Themes & Wallpapers")
        .id(egui::Id::new("studio_wallpaper_modal_win"))
        .open(&mut win_open)
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(w, h))
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(
            egui::Frame::window(&ctx.global_style())
                .fill(Color32::from_rgba_unmultiplied(20, 22, 29, 246))
                .stroke(Stroke::new(1.0, PASTEL_LAVENDER.gamma_multiply(0.5)))
                .corner_radius(CornerRadius::same(16))
                .inner_margin(egui::Margin::same(14)),
        )
        .show(ctx, |ui| {
            settings_ui(ui);
        });
    if let Some(r) = shown {
        draw_luminous_border_trail(ctx, &ctx.layer_painter(r.response.layer_id), r.response.rect, 16.0, PASTEL_LAVENDER);
    }
    if !win_open {
        open = false;
    }
    ctx.data_mut(|d| d.insert_temp(open_id, open));
}

pub fn toggle_modal(ctx: &egui::Context) {
    let open_id = egui::Id::new("wallpaper_modal_open");
    let cur = ctx.data(|d| d.get_temp::<bool>(open_id).unwrap_or(false));
    ctx.data_mut(|d| d.insert_temp(open_id, !cur));
}
