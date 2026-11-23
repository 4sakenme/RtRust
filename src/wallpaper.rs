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

// ── Curated RtRust Pastel Palette ──────────────────────────────────────────

pub const PASTEL_LAVENDER: Color32 = Color32::from_rgb(184, 181, 255); // #B8B5FF
pub const PASTEL_MINT: Color32 = Color32::from_rgb(168, 230, 207);     // #A8E6CF
pub const PASTEL_SKY: Color32 = Color32::from_rgb(160, 210, 235);      // #A0D2EB
pub const PASTEL_PEACH: Color32 = Color32::from_rgb(255, 170, 165);    // #FFAAA5
pub const PASTEL_ROSE: Color32 = Color32::from_rgb(245, 180, 215);     // #F5B4D7
pub const PASTEL_GOLD: Color32 = Color32::from_rgb(255, 223, 160);     // #FFDFA0

pub const PASTEL_ACCENTS: &[Color32] = &[
    PASTEL_LAVENDER,
    PASTEL_MINT,
    PASTEL_SKY,
    PASTEL_PEACH,
    PASTEL_ROSE,
    PASTEL_GOLD,
];

#[inline]
pub fn pastel_for_index(idx: usize) -> Color32 {
    PASTEL_ACCENTS[idx % PASTEL_ACCENTS.len()]
}

/// Curated background color presets for Studio / Start Page and Code Editor.
pub const STUDIO_BG_PRESETS: &[(&str, [u8; 3], Color32)] = &[
    ("Midnight Slate (Default)", [20, 22, 29], PASTEL_LAVENDER),
    ("Pastel Lavender Mist", [28, 26, 42], PASTEL_LAVENDER),
    ("Mint Sage Night", [20, 32, 30], PASTEL_MINT),
    ("Sky Nebula", [20, 30, 42], PASTEL_SKY),
    ("Sunset Peach Dusk", [36, 25, 31], PASTEL_PEACH),
    ("Deep OLED Pitch", [11, 12, 16], PASTEL_SKY),
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
            studio_bg_rgb: [20, 22, 29],
            studio_image_path: None,
            studio_dim_opacity: 0.45,
            code_bg_rgb: [22, 24, 31],
            code_image_path: None,
            code_dim_opacity: 0.62,
            luminous_trails: true,
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

/// Draws a smooth animated luminous border trail orbiting `rect` for popovers, modals, and floating panels.
pub fn draw_luminous_border_trail(
    ctx: &egui::Context,
    painter: &egui::Painter,
    rect: Rect,
    rounding: f32,
    accent: Color32,
) {
    let enabled = {
        let mut g = state().lock().unwrap();
        ensure_loaded(&mut g);
        g.cfg.luminous_trails
    };
    let cr = CornerRadius::same(rounding.round().clamp(0.0, 255.0) as u8);
    // Crisp base border
    painter.rect_stroke(
        rect,
        cr,
        Stroke::new(1.0, accent.gamma_multiply(0.32)),
        egui::StrokeKind::Inside,
    );
    if !enabled {
        return;
    }
    let t = (ctx.input(|i| i.time) as f32) * 0.55;
    let perimeterPoints = rect_perimeter_points(rect.shrink(0.5), 48);
    if perimeterPoints.len() < 8 {
        return;
    }
    let n = perimeterPoints.len();
    let head = ((t.fract()) * (n as f32)) as usize;
    let trail_len = 14usize;
    for step in 0..trail_len {
        let i0 = (head + n - step) % n;
        let i1 = (head + n - step - 1) % n;
        let fade = 1.0 - (step as f32 / trail_len as f32);
        let col = accent.gamma_multiply(fade * fade * 0.95);
        painter.line_segment(
            [perimeterPoints[i1], perimeterPoints[i0]],
            Stroke::new(1.8 + fade * 0.7, col),
        );
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(33));
}

fn rect_perimeter_points(r: Rect, count: usize) -> Vec<Pos2> {
    let w = r.width().max(1.0);
    let h = r.height().max(1.0);
    let perim = 2.0 * (w + h);
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let mut d = (i as f32 / count as f32) * perim;
        let p = if d < w {
            Pos2::new(r.min.x + d, r.min.y)
        } else {
            d -= w;
            if d < h {
                Pos2::new(r.max.x, r.min.y + d)
            } else {
                d -= h;
                if d < w {
                    Pos2::new(r.max.x - d, r.max.y)
                } else {
                    d -= w;
                    Pos2::new(r.min.x, r.max.y - d)
                }
            }
        };
        out.push(p);
    }
    out
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
