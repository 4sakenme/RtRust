//! Studio panels and popups: Avatar Settings (File > Avatar Settings,
//! Avatar tab > Settings), the Insert Object popup (an Explorer row's "+",
//! Window > Insert Object, Ctrl+I) and View > Screenshot.
//!
//! Insert Object lists classes the way Studio's does: grouped by Studio's
//! own ReflectionMetadata ClassCategory (read at run time from the Studio
//! install, like the other Studio files in CREDITS.md), searchable, with
//! each class's icon. The Explorer also orders siblings by its ExplorerOrder.

use std::collections::HashMap;
use std::sync::OnceLock;

use egui::{RichText, Sense, Vec2};
use rbx_dom_weak::types::{Enum, Ref, Variant};

use crate::app::Editor;
use crate::studio_ui::InsertPopup;
use rustudio_engine::theme;

#[derive(Default, Clone)]
pub struct ClassMeta {
    pub category: String,
    pub explorer_order: Option<i32>,
    /// ServiceVisibility (Always 0, Off 1, WithChildren 2), when set.
    pub service_visibility: Option<i32>,
}

/// Studio's ReflectionMetadata.xml, by class (empty when not found).
pub fn class_meta() -> &'static HashMap<String, ClassMeta> {
    static M: OnceLock<HashMap<String, ClassMeta>> = OnceLock::new();
    M.get_or_init(|| {
        let mut out = HashMap::new();
        let Some(dir) = rustudio_core::default_scripts::studio_dir() else {
            return out;
        };
        let Ok(text) = std::fs::read_to_string(dir.join("ReflectionMetadata.xml")) else {
            return out;
        };
        let field = |chunk: &str, name: &str| -> Option<String> {
            let key = format!("<string name=\"{name}\">");
            let i = chunk.find(&key)? + key.len();
            let j = chunk[i..].find("</string>")? + i;
            Some(chunk[i..j].to_string())
        };
        let field_tok = |chunk: &str, name: &str| -> Option<String> {
            let key = format!("<token name=\"{name}\">");
            let i = chunk.find(&key)? + key.len();
            let j = chunk[i..].find("</token>")? + i;
            Some(chunk[i..j].to_string())
        };
        for chunk in text.split("<Item class=\"ReflectionMetadataClass\"").skip(1) {
            let head = chunk.split("</Properties>").next().unwrap_or("");
            let Some(name) = field(head, "Name") else {
                continue;
            };
            out.insert(
                name,
                ClassMeta {
                    category: field(head, "ClassCategory").unwrap_or_default(),
                    explorer_order: field(head, "ExplorerOrder").and_then(|v| v.trim().parse().ok()),
                    service_visibility: field_tok(head, "ServiceVisibility").and_then(|v| v.trim().parse().ok()),
                },
            );
        }
        out
    })
}

/// Studio's Explorer order for a class (lower first; classes without one last).
pub fn explorer_order(class: &str) -> i32 {
    class_meta().get(class).and_then(|m| m.explorer_order).unwrap_or(i32::MAX)
}

/// Per-member ReflectionMetadata: (PropertyOrder, Browsable), by (class, member).
pub fn member_meta() -> &'static HashMap<(String, String), (Option<i32>, bool)> {
    static M: OnceLock<HashMap<(String, String), (Option<i32>, bool)>> = OnceLock::new();
    M.get_or_init(|| {
        let mut out = HashMap::new();
        let Some(dir) = rustudio_core::default_scripts::studio_dir() else {
            return out;
        };
        let Ok(text) = std::fs::read_to_string(dir.join("ReflectionMetadata.xml")) else {
            return out;
        };
        let field = |chunk: &str, tag: &str, name: &str| -> Option<String> {
            let key = format!("<{tag} name=\"{name}\">");
            let i = chunk.find(&key)? + key.len();
            let j = chunk[i..].find('<')? + i;
            Some(chunk[i..j].to_string())
        };
        for chunk in text.split("<Item class=\"ReflectionMetadataClass\"").skip(1) {
            let head = chunk.split("</Properties>").next().unwrap_or("");
            let Some(class) = field(head, "string", "Name") else {
                continue;
            };
            for mem in chunk.split("<Item class=\"ReflectionMetadataMember\"").skip(1) {
                let mh = mem.split("</Properties>").next().unwrap_or("");
                let Some(name) = field(mh, "string", "Name") else {
                    continue;
                };
                let order = field(mh, "string", "PropertyOrder").and_then(|v| v.trim().parse().ok());
                let browsable = field(mh, "bool", "Browsable")
                    .or_else(|| field(mh, "string", "Browsable"))
                    .is_none_or(|v| !v.trim().eq_ignore_ascii_case("false"));
                out.insert((class.clone(), name), (order, browsable));
            }
        }
        out
    })
}

/// StarterPlayer.GameSettingsAvatar (Enum.GameAvatarType): R6 0, R15 1, PlayerChoice 2.
const AVATAR_TYPES: [(&str, u32, &str); 3] = [
    ("R15", 1, "Characters use the R15 body (15 parts)."),
    ("R6", 0, "Characters use the classic R6 body (6 parts)."),
    ("Player Choice", 2, "Each player's own avatar decides (rustudio: your avatar when signed in, else R15)."),
];

impl Editor {
    fn starter_player(&self) -> Option<Ref> {
        let root = self.game.doc.dom.root_ref();
        self.game.doc.find_child(root, "StarterPlayer")
    }

    /// The place's avatar type (StarterPlayer.GameSettingsAvatar), Studio's default R15.
    pub fn avatar_type(&self) -> u32 {
        self.starter_player()
            .and_then(|sp| rustudio_render::scene::enum_u32(self.game.doc.prop(sp, "GameSettingsAvatar")))
            .unwrap_or(1)
    }

    /// Sets the place's avatar type (undoable, saved with the place).
    /// Studio's setter (, on
    /// StarterPlayer, category "Character", value at with
    /// the component at) returns at once when the value
    /// is unchanged: no change is recorded, so no undo step either.
    pub fn set_avatar_type(&mut self, v: u32) {
        if v > 2 || self.avatar_type() == v {
            return;
        }
        if let Some(sp) = self.starter_player() {
            let edit = vec![(sp, rbx_dom_weak::ustr("GameSettingsAvatar"), Variant::Enum(Enum::from_u32(v)))];
            self.game.doc.set_props("Avatar Type", edit, None);
        }
    }

    /// The Avatar Settings panel (Studio: File > Avatar Settings).
    pub fn avatar_settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.label(RichText::new("Avatar").strong());
        ui.label(
            RichText::new("Saved with the place (StarterPlayer.GameSettingsAvatar), so Play uses it here and in Roblox.")
                .color(theme::TEXT_DIM)
                .small(),
        );
        ui.add_space(6.0);
        let cur = self.avatar_type();
        let has_sp = self.starter_player().is_some();
        ui.add_enabled_ui(has_sp && self.game.play.is_none(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Avatar Type");
                for (label, v, tip) in AVATAR_TYPES {
                    let r = ui.radio(cur == v, label).on_hover_text(tip);
                    self.ui_rects.insert(avatar_type_key(v), r.rect);
                    if r.clicked() && cur != v {
                        self.set_avatar_type(v);
                    }
                }
            });
        });
        if !has_sp {
            ui.label(RichText::new("This place has no StarterPlayer.").color(theme::WARN));
        }
        let (rig, source) = self.game.rig_choice();
        let name = if rig == rustudio_engine::rig::RigType::R15 { "R15" } else { "R6" };
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!("Next Play: {name} (from the {})", source.label()))
                .color(theme::TEXT_DIM)
                .small(),
        );
    }

    /// Opens Insert Object for `parent` at `at` (Studio's popup).
    pub fn open_insert_popup(&mut self, parent: Ref, at: egui::Pos2) {
        self.ui.insert_popup = Some(InsertPopup {
            parent,
            at,
            query: String::new(),
            opened_frame: self.game.frame_no,
        });
    }

    /// The Insert Object popup: a search field, then the classes that can
    /// be made, by Studio's category; Enter or a click inserts into the parent.
    pub fn insert_popup_ui(&mut self, ctx: &egui::Context) {
        let Some(mut pop) = self.ui.insert_popup.take() else {
            return;
        };
        let mut chosen: Option<String> = None;
        let mut close = false;
        let screen = ctx.content_rect();
        let mobile = rustudio_core::mobile::is_drawer_mode() || screen.width() < 700.0;
        let pop_w = if mobile { (screen.width() - 32.0).clamp(260.0, 380.0) } else { 280.0 };
        let row_h = if mobile { 38.0 } else { 26.0 };
        let area_builder = egui::Area::new(egui::Id::new("insert_object_popup"))
            .order(egui::Order::Foreground);
        let area_builder = if mobile {
            area_builder.anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        } else {
            area_builder.fixed_pos(pop.at)
        };
        let area = area_builder.show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(theme::PANEL_DARK)
                    .stroke(egui::Stroke::new(1.0, theme::DIVIDER))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                    ui.set_width(pop_w);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Insert Object").strong().size(16.0).color(egui::Color32::WHITE));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(RichText::new("CANCEL").color(egui::Color32::from_rgb(130, 175, 255))).clicked() {
                                close = true;
                            }
                        });
                    });
                    ui.add_space(6.0);
                    let r = ui.add_sized(
                        [pop_w, if mobile { 34.0 } else { 26.0 }],
                        egui::TextEdit::singleline(&mut pop.query)
                            .hint_text("Search objects"),
                    );
                    self.ui_rects.insert("insert.search", r.rect);
                    if !mobile && pop.opened_frame + 1 >= self.game.frame_no {
                        r.request_focus();
                    }
                    ui.add_space(4.0);
                    let q = pop.query.to_lowercase();
                    let meta = class_meta();
                    let mut classes: Vec<&str> = rustudio_core::doc::db()
                        .classes
                        .keys()
                        .copied()
                        .filter(|c| rustudio_core::doc::is_creatable(c) && c.to_lowercase().contains(&q))
                        .collect();
                    let cat = |c: &str| {
                        meta.get(c)
                            .map(|m| m.category.clone())
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| "Other".into())
                    };
                    if q.is_empty() {
                        classes.sort_by_key(|c| (cat(c), c.to_string()));
                    } else {
                        classes.sort_by_key(|c| (!c.to_lowercase().starts_with(&q), c.len(), c.to_string()));
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        chosen = classes.first().map(|s| s.to_string());
                    }
                    let max_h = if mobile { (screen.height() * 0.56).max(240.0) } else { 400.0 };
                    egui::ScrollArea::vertical().max_height(max_h).show(ui, |ui| {
                        let mut last = String::new();
                        for c in classes.iter().take(400) {
                            if q.is_empty() {
                                let k = cat(c);
                                if k != last {
                                    ui.add_space(4.0);
                                    ui.label(RichText::new(&k).small().color(theme::TEXT_DIM));
                                    last = k;
                                }
                            }
                            let (r, resp) = ui.allocate_exact_size(Vec2::new(pop_w - 8.0, row_h), Sense::click());
                            if resp.hovered() {
                                ui.painter().rect_filled(r, 4.0, theme::HOVER_BG);
                            }
                            let icon_sz = if mobile { 20.0 } else { 16.0 };
                            let icon = egui::Rect::from_center_size(
                                egui::pos2(r.min.x + 14.0, r.center().y),
                                Vec2::splat(icon_sz),
                            );
                            crate::icons::paint_class_icon(ui.painter(), icon, c);
                            let col = if resp.hovered() { egui::Color32::WHITE } else { theme::TEXT };
                            ui.painter().text(
                                egui::pos2(r.min.x + 32.0, r.center().y),
                                egui::Align2::LEFT_CENTER,
                                *c,
                                egui::FontId::proportional(if mobile { 15.0 } else { 13.0 }),
                                col,
                            );
                            ui.painter().line_segment(
                                [egui::pos2(r.min.x, r.max.y), egui::pos2(r.max.x, r.max.y)],
                                egui::Stroke::new(0.5, theme::DIVIDER.gamma_multiply(0.5)),
                            );
                            if resp.clicked() {
                                chosen = Some(c.to_string());
                            }
                        }
                    });
                });
            });
        let outside = ctx.input(|i| i.pointer.any_pressed())
            && !ctx.pointer_interact_pos().is_some_and(|p| area.response.rect.contains(p))
            && pop.opened_frame + 1 < self.game.frame_no;
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || outside {
            close = true;
        }
        if let Some(c) = chosen {
            self.insert_class(&c, pop.parent);
        } else if !close {
            self.ui.insert_popup = Some(pop);
        }
    }

    /// View > Screenshot: the window as a PNG in ~/Pictures.
    pub fn user_screenshot_tick(&mut self, ctx: &egui::Context) {
        if self.pending_user_shot.is_none() {
            return;
        }
        let shot = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let (Some(image), Some(path)) = (shot, self.pending_user_shot.clone()) {
            self.pending_user_shot = None;
            match crate::app::save_png(&image, &path) {
                Ok(()) => self.info(format!("Screenshot saved to {}", path.display())),
                Err(e) => self.warn(format!("Screenshot failed: {e}")),
            }
        }
    }
}

pub fn avatar_type_key(v: u32) -> &'static str {
    match v {
        0 => "avatar.R6",
        1 => "avatar.R15",
        _ => "avatar.PlayerChoice",
    }
}

#[cfg(test)]
mod tests {
    /// Studio's metadata, when the install is there: services order like
    /// its Explorer (Workspace, Players, Lighting… before the rest).
    #[test]
    fn explorer_order_from_studio() {
        if super::class_meta().is_empty() {
            return;
        }
        let o = super::explorer_order;
        assert!(o("Workspace") < o("Players") && o("Players") < o("Lighting"));
        assert!(o("Camera") < o("Part"));
        assert_eq!(super::class_meta()["ClickDetector"].category, "3D Interfaces");
    }
}
