//! Script editor tabs: Luau syntax highlighting, line numbers and a live
//! compile check that underlines the line of the first syntax error.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, RichText, Stroke};
use rbx_dom_weak::types::{Ref, Variant};

use crate::app::Editor;
use crate::icons;
use rustudio_engine::runtime;
use rustudio_engine::theme;

const KEYWORDS: &[&str] = &[
    "and", "break", "continue", "do", "else", "elseif", "end", "export", "false", "for",
    "function", "if", "in", "local", "nil", "not", "or", "repeat", "return", "then", "true",
    "type", "until", "while",
];

const GLOBALS: &[&str] = &[
    "game",
    "workspace",
    "Workspace",
    "script",
    "Instance",
    "Vector3",
    "Vector2",
    "CFrame",
    "Color3",
    "BrickColor",
    "UDim",
    "UDim2",
    "Enum",
    "task",
    "print",
    "warn",
    "error",
    "require",
    "typeof",
    "type",
    "math",
    "string",
    "table",
    "pcall",
    "xpcall",
    "pairs",
    "ipairs",
    "next",
    "select",
    "tostring",
    "tonumber",
    "setmetatable",
    "getmetatable",
    "assert",
    "wait",
    "spawn",
    "delay",
    "tick",
    "time",
    "coroutine",
    "buffer",
    "bit32",
    "utf8",
    "os",
    "debug",
    "self",
    "shared",
    "_G",
    "rawget",
    "rawset",
    "rawequal",
    "unpack",
];


#[derive(Debug, Clone, Copy, PartialEq)]
enum Tok {
    Plain,
    Keyword,
    Global,
    Str,
    Num,
    Comment,
    Op,
}

/// Splits Luau source into coloured spans (byte ranges).
fn tokenize(src: &str) -> Vec<(std::ops::Range<usize>, Tok)> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    // `[[ ... ]]` / `[==[ ... ]==]` long brackets; returns the end index.
    let long_bracket = |at: usize| -> Option<usize> {
        if b.get(at) != Some(&b'[') {
            return None;
        }
        let mut j = at + 1;
        let mut level = 0;
        while b.get(j) == Some(&b'=') {
            level += 1;
            j += 1;
        }
        if b.get(j) != Some(&b'[') {
            return None;
        }
        let close: String = format!("]{}]", "=".repeat(level));
        Some(
            src[j + 1..]
                .find(&close)
                .map(|p| j + 1 + p + close.len())
                .unwrap_or(src.len()),
        )
    };
    while i < b.len() {
        let c = b[i];
        let start = i;
        if c == b'-' && b.get(i + 1) == Some(&b'-') {
            i = long_bracket(i + 2)
                .unwrap_or_else(|| src[i..].find('\n').map(|p| i + p).unwrap_or(src.len()));
            out.push((start..i, Tok::Comment));
        } else if c == b'"' || c == b'\'' || c == b'`' {
            i += 1;
            while i < b.len() && b[i] != c && b[i] != b'\n' {
                i += if b[i] == b'\\' { 2 } else { 1 };
            }
            i = (i + 1).min(b.len());
            out.push((start..i, Tok::Str));
        } else if let Some(end) = (c == b'[').then(|| long_bracket(i)).flatten() {
            i = end;
            out.push((start..i, Tok::Str));
        } else if c.is_ascii_digit()
            || (c == b'.' && b.get(i + 1).is_some_and(|d| d.is_ascii_digit()))
        {
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'.' || b[i] == b'_') {
                i += 1;
            }
            out.push((start..i, Tok::Num));
        } else if c.is_ascii_alphabetic() || c == b'_' {
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            let word = &src[start..i];
            let after_dot = start > 0 && b[start - 1] == b'.' || start > 0 && b[start - 1] == b':';
            let t = if KEYWORDS.contains(&word) {
                Tok::Keyword
            } else if GLOBALS.contains(&word) && !after_dot {
                Tok::Global
            } else {
                Tok::Plain
            };
            out.push((start..i, t));
        } else if c.is_ascii_punctuation() {
            i += 1;
            out.push((start..i, Tok::Op));
        } else {
            // Whitespace and anything else (including multi-byte UTF-8).
            i += 1;
            while i < b.len() && !src.is_char_boundary(i) {
                i += 1;
            }
            out.push((start..i, Tok::Plain));
        }
    }
    out
}

/// Byte range of a 1-based line.
fn line_range(src: &str, line: usize) -> Option<std::ops::Range<usize>> {
    let mut start = 0;
    for (n, l) in src.split('\n').enumerate() {
        if n + 1 == line {
            return Some(start..start + l.len());
        }
        start += l.len() + 1;
    }
    None
}

/// A highlighted layout job; the error line gets a red wavy-looking underline.
pub fn highlight(src: &str, error_line: Option<usize>, font: FontId) -> LayoutJob {
    let mut job = LayoutJob::default();
    let err = error_line.and_then(|l| line_range(src, l));
    // Studio Settings > Script Editor > Theme.
    let sc = crate::studio_settings::script_colors();
    for (range, tok) in tokenize(src) {
        let color = match tok {
            Tok::Plain => sc.text,
            Tok::Keyword => sc.keyword,
            Tok::Global => sc.builtin,
            Tok::Str => sc.string,
            Tok::Num => sc.number,
            Tok::Comment => sc.comment,
            Tok::Op => sc.operator,
        };
        // Split the token where it crosses into / out of the error line.
        let mut cuts = vec![range.start, range.end];
        if let Some(e) = &err {
            for p in [e.start, e.end] {
                if p > range.start && p < range.end {
                    cuts.push(p);
                }
            }
        }
        cuts.sort_unstable();
        for w in cuts.windows(2) {
            let (a, z) = (w[0], w[1]);
            if a == z {
                continue;
            }
            let on_err = err
                .as_ref()
                .is_some_and(|e| a >= e.start && z <= e.end && e.start != e.end);
            let fmt = TextFormat {
                font_id: font.clone(),
                color,
                italics: tok == Tok::Comment,
                underline: if on_err {
                    Stroke::new(1.5, theme::ERROR)
                } else {
                    Stroke::NONE
                },
                background: if on_err {
                    Color32::from_rgba_unmultiplied(240, 96, 86, 28)
                } else {
                    Color32::TRANSPARENT
                },
                ..Default::default()
            };
            job.append(&src[a..z], 0.0, fmt);
        }
    }
    job
}

pub struct ScriptBuf {
    pub text: String,
    pub error: Option<(usize, String)>,
    checked: Option<String>,
    edited: Instant,
    /// Plugins' ScriptEditorService analysis results (line, message) and
    /// autocomplete items (label, text) for the text they were made for.
    plugin_diags: Vec<(i64, String)>,
    plugin_checked: Option<String>,
    completions: Vec<(String, String)>,
}

#[derive(Default)]
pub struct ScriptTabs {
    pub bufs: HashMap<Ref, ScriptBuf>,
    /// Lines (1-based) to put the cursor on and scroll to once the tab
    /// draws (the profiler's "Open script at line N").
    pub goto: HashMap<Ref, usize>,
}

impl Editor {
    pub fn script_title(&self, r: Ref) -> String {
        match self.game.doc.dom.get_by_ref(r) {
            Some(i) => i.name.clone(),
            None => "Script".to_string(),
        }
    }

    pub fn script_tab_ui(&mut self, ui: &mut egui::Ui, r: Ref) {
        self.live_draft_toolbar(ui,r);
        let Some(inst) = self.game.doc.dom.get_by_ref(r) else {
            ui.add_space(12.0);
            ui.label(RichText::new("This script is not in the DataModel being shown (it may belong to the edit copy while playing).").color(theme::TEXT_DIM));
            return;
        };
        let class = inst.class.to_string();
        let source = match self.game.doc.prop(r, "Source") {
            Some(Variant::String(s)) => s,
            _ => String::new(),
        };
        let full_name = self.full_name(r);
        let follow_live_source=self.live_script_has_draft(r)==Some(false);
        // A tab on screen is an open ScriptEditorService document.
        if !crate::plugin_host::script_docs::is_open(r) {
            crate::plugin_host::script_docs::open(r, None, true);
        }
        let editor_id = egui::Id::new(("script_text", r));
        let pre_cursor_idx = egui::TextEdit::load_state(ui.ctx(), editor_id)
            .and_then(|st| st.cursor.char_range())
            .filter(|cr| cr.primary.index.0 == cr.secondary.index.0)
            .map(|cr| cr.primary.index.0);
        let dom_suggestions = {
            let current_text = self
                .scripts
                .bufs
                .get(&r)
                .map(|b| b.text.as_str())
                .unwrap_or(source.as_str());
            if let Some(idx) = pre_cursor_idx {
                let before: String = current_text.chars().take(idx).collect();
                mobile_luau_suggestions(&self.game.doc.dom, r, &before)
            } else {
                Vec::new()
            }
        };
        let buf = self.scripts.bufs.entry(r).or_insert_with(|| ScriptBuf {
            text: source.clone(),
            error: None,
            checked: None,
            edited: Instant::now(),
            plugin_diags: Vec::new(),
            plugin_checked: None,
            completions: Vec::new(),
        });
        let editor_id = egui::Id::new(("script_text", r));
        let goto = self.scripts.goto.get(&r).copied();
        let focused = ui.ctx().memory(|m| m.has_focus(editor_id));
        // Undo/redo or a script change elsewhere: follow the document.
        if (!focused || follow_live_source) && buf.text != source {
            buf.text = source.clone();
            buf.checked = None;
        }
        // Compile check once typing pauses.
        if buf.checked.as_deref() != Some(buf.text.as_str())
            && buf.edited.elapsed() > Duration::from_millis(250)
        {
            buf.error = runtime::check_syntax(&buf.text, &full_name);
            buf.checked = Some(buf.text.clone());
        }
        // Plugins' script analysis callbacks (ScriptEditorService), once
        // typing pauses too.
        let analyze = buf.plugin_checked.as_deref() != Some(buf.text.as_str())
            && buf.edited.elapsed() > Duration::from_millis(250);
        if analyze {
            buf.plugin_checked = Some(buf.text.clone());
            let diags = self
                .plugins
                .with_edit(&mut self.game.doc, |rt| {
                    let lua = &rt.lua;
                    if crate::plugin_host::api::script_editor::has_callbacks(lua).1 {
                        crate::plugin_host::api::script_editor::analyze(lua, r).unwrap_or_default()
                    } else {
                        Vec::new()
                    }
                })
                .unwrap_or_default();
            if let Some(b) = self.scripts.bufs.get_mut(&r) {
                b.plugin_diags = diags;
            }
        }
        let Some(buf) = self.scripts.bufs.get_mut(&r) else { return };

        let mut jump_to_line: Option<usize> = None;
        let mut toggle_comment = false;
        let mut find_next: Option<String> = None;
        let find_open_id = egui::Id::new("mobile_script_find_open");
        let find_query_id = egui::Id::new("mobile_script_find_query");
        let font_override_id = egui::Id::new("mobile_script_font_px");

        let mut find_open = ui.ctx().data(|d| d.get_temp::<bool>(find_open_id).unwrap_or(false));
        let mut find_query = ui.ctx().data(|d| d.get_temp::<String>(find_query_id).unwrap_or_default());

        ui.horizontal_wrapped(|ui| {
            icons::class_icon(ui, &class, 16.0);
            ui.label(RichText::new(&full_name).strong());
            ui.label(
                RichText::new(format!("{} lines", buf.text.lines().count().max(1)))
                    .small()
                    .color(theme::TEXT_DIM),
            );
            if buf.error.is_none() {
                let color = Color32::from_rgb(130, 200, 120);
                ui.label(rustudio_engine::ti::text("check", 12.0).color(color));
                ui.label(RichText::new("OK").small().color(color));
            }
            let base_font = crate::studio_settings::font_px("Studio", "Font");
            let is_portrait = ui.ctx().content_rect().width() < ui.ctx().content_rect().height();
            let narrow = ui.ctx().content_rect().width() < 540.0;
            let cur_px = ui.ctx().data(|d| {
                d.get_temp::<f32>(font_override_id)
                    .unwrap_or(if is_portrait { base_font.max(14.5) } else { base_font })
            });
            let wrap_id = egui::Id::new("mobile_script_wrap_override");
            let cur_wrap = ui.ctx().data(|d| d.get_temp::<bool>(wrap_id))
                .unwrap_or_else(|| crate::studio_settings::flag("Studio", "Text Wrapping", false));
            let tools_id = egui::Id::new("mobile_script_tools_open");
            let mut tools_open = ui.ctx().data(|d| d.get_temp::<bool>(tools_id).unwrap_or(false));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if narrow {
                    let arrow = if tools_open { "▲ Tools" } else { "▼ Tools" };
                    if ui.selectable_label(tools_open, arrow).clicked() {
                        tools_open = !tools_open;
                        ui.ctx().data_mut(|d| d.insert_temp(tools_id, tools_open));
                    }
                    if ui.selectable_label(find_open, "🔍 Find").clicked() {
                        find_open = !find_open;
                        ui.ctx().data_mut(|d| d.insert_temp(find_open_id, find_open));
                    }
                    if ui.small_button("--").on_hover_text("Toggle Comment dòng hiện tại").clicked() {
                        toggle_comment = true;
                    }
                    return;
                }
                let ctx_menu = ui.ctx().clone();
                ui.menu_button("Font ▼", |ui| {
                    for (id, name, cached) in crate::font_manager::available_code_fonts() {
                        if id == "default" {
                            if ui.button(name).clicked() {
                                crate::font_manager::set_active_font(None, crate::font_manager::FontCategory::Code, &ctx_menu);
                                ui.close();
                            }
                        } else if cached {
                            if ui.button(format!("✓ {name}")).clicked() {
                                crate::font_manager::set_active_font(Some(&id), crate::font_manager::FontCategory::Code, &ctx_menu);
                                ui.close();
                            }
                        } else if let Some(item) = crate::font_manager::CATALOG.iter().find(|c| c.id == id) {
                            if ui.button(format!("⬇ {name}")).clicked() {
                                crate::font_manager::start_fetch(item, ctx_menu.clone());
                            }
                        }
                    }
                });
                if ui.selectable_label(cur_wrap, "Wrap").on_hover_text("Tự động xuống dòng / Cuộn ngang").clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp(wrap_id, !cur_wrap));
                }
                if ui.small_button("A+").on_hover_text("Tăng cỡ chữ").clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp(font_override_id, (cur_px + 1.5).min(24.0)));
                }
                if ui.small_button("A-").on_hover_text("Giảm cỡ chữ").clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp(font_override_id, (cur_px - 1.5).max(10.0)));
                }
                if ui.selectable_label(find_open, "🔍 Find").clicked() {
                    find_open = !find_open;
                    ui.ctx().data_mut(|d| d.insert_temp(find_open_id, find_open));
                }
                if ui.small_button("--").on_hover_text("Toggle Comment dòng hiện tại").clicked() {
                    toggle_comment = true;
                }
                let clip_id = egui::Id::new("mobile_clipboard_ring");
                let clip_ring = ui.ctx().data(|d| d.get_temp::<Vec<String>>(clip_id).unwrap_or_default());
                ui.menu_button("📋 History", |ui| {
                    if clip_ring.is_empty() {
                        ui.label(RichText::new("Chưa có lịch sử copy").small().color(theme::TEXT_DIM));
                    } else {
                        for item in &clip_ring {
                            let preview: String = item.lines().next().unwrap_or("").chars().take(32).collect();
                            if ui.button(preview).clicked() {
                                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("mobile_luau_snippet"), item.clone()));
                                ui.close();
                            }
                        }
                    }
                });
                let outline_items = scan_function_outline(&buf.text);
                ui.menu_button("ƒ Outline", |ui| {
                    if outline_items.is_empty() {
                        ui.label(RichText::new("Không tìm thấy function / section").small().color(theme::TEXT_DIM));
                    } else {
                        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                            for (line_no, label) in &outline_items {
                                if ui.button(format!("L{line_no}: {label}")).clicked() {
                                    jump_to_line = Some(*line_no);
                                    ui.close();
                                }
                            }
                        });
                    }
                });
            });
        });

        let narrow = ui.ctx().content_rect().width() < 540.0;
        let tools_open = ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("mobile_script_tools_open")).unwrap_or(false));
        if narrow && tools_open {
            let base_font = crate::studio_settings::font_px("Studio", "Font");
            let is_portrait = ui.ctx().content_rect().width() < ui.ctx().content_rect().height();
            let cur_px = ui.ctx().data(|d| {
                d.get_temp::<f32>(font_override_id)
                    .unwrap_or(if is_portrait { base_font.max(14.5) } else { base_font })
            });
            let wrap_id = egui::Id::new("mobile_script_wrap_override");
            let cur_wrap = ui.ctx().data(|d| d.get_temp::<bool>(wrap_id))
                .unwrap_or_else(|| crate::studio_settings::flag("Studio", "Text Wrapping", false));
            ui.horizontal_wrapped(|ui| {
                let outline_items = scan_function_outline(&buf.text);
                ui.menu_button("ƒ Outline", |ui| {
                    if outline_items.is_empty() {
                        ui.label(RichText::new("Không tìm thấy function / section").small().color(theme::TEXT_DIM));
                    } else {
                        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                            for (line_no, label) in &outline_items {
                                if ui.button(format!("L{line_no}: {label}")).clicked() {
                                    jump_to_line = Some(*line_no);
                                    ui.close();
                                }
                            }
                        });
                    }
                });
                let clip_id = egui::Id::new("mobile_clipboard_ring");
                let clip_ring = ui.ctx().data(|d| d.get_temp::<Vec<String>>(clip_id).unwrap_or_default());
                ui.menu_button("📋 History", |ui| {
                    if clip_ring.is_empty() {
                        ui.label(RichText::new("Chưa có lịch sử copy").small().color(theme::TEXT_DIM));
                    } else {
                        for item in &clip_ring {
                            let preview: String = item.lines().next().unwrap_or("").chars().take(32).collect();
                            if ui.button(preview).clicked() {
                                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("mobile_luau_snippet"), item.clone()));
                                ui.close();
                            }
                        }
                    }
                });
                if ui.small_button("A-").clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp(font_override_id, (cur_px - 1.5).max(10.0)));
                }
                if ui.small_button("A+").clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp(font_override_id, (cur_px + 1.5).min(24.0)));
                }
                if ui.selectable_label(cur_wrap, "Wrap").clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp(wrap_id, !cur_wrap));
                }
                let ctx_menu = ui.ctx().clone();
                ui.menu_button("Font ▼", |ui| {
                    for (id, name, cached) in crate::font_manager::available_code_fonts() {
                        if id == "default" {
                            if ui.button(name).clicked() {
                                crate::font_manager::set_active_font(None, crate::font_manager::FontCategory::Code, &ctx_menu);
                                ui.close();
                            }
                        } else if cached {
                            if ui.button(format!("✓ {name}")).clicked() {
                                crate::font_manager::set_active_font(Some(&id), crate::font_manager::FontCategory::Code, &ctx_menu);
                                ui.close();
                            }
                        } else if let Some(item) = crate::font_manager::CATALOG.iter().find(|c| c.id == id) {
                            if ui.button(format!("⬇ {name}")).clicked() {
                                crate::font_manager::start_fetch(item, ctx_menu.clone());
                            }
                        }
                    }
                });
            });
        }

        // Clickable Jump-to-Error banner when Luau syntax check finds an error
        if let Some((err_line, err_msg)) = &buf.error {
            let banner = egui::Frame::new()
                .fill(Color32::from_rgb(95, 25, 28))
                .inner_margin(egui::Margin::symmetric(8, 5))
                .corner_radius(4.0);
            let resp = banner
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("⚠ Line {err_line}: {err_msg}"))
                                .color(Color32::from_rgb(255, 210, 210))
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("Tap to Jump ↵").small().color(Color32::WHITE));
                        });
                    });
                })
                .response
                .interact(egui::Sense::click());
            if resp.clicked() {
                jump_to_line = Some(*err_line);
            }
        }

        // Compact Find Bar
        if find_open {
            ui.horizontal(|ui| {
                ui.label("Find:");
                let f_resp = ui.add(egui::TextEdit::singleline(&mut find_query).desired_width(140.0));
                if f_resp.changed() {
                    ui.ctx().data_mut(|d| d.insert_temp(find_query_id, find_query.clone()));
                }
                if (ui.button("Next").clicked() || (f_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                    && !find_query.is_empty()
                {
                    find_next = Some(find_query.clone());
                }
                if ui.small_button("✕").clicked() {
                    find_open = false;
                    ui.ctx().data_mut(|d| d.insert_temp(find_open_id, false));
                }
            });
        }

        for (line, msg) in &buf.plugin_diags {
            ui.label(RichText::new(format!("⚠ line {line}: {msg}")).small().color(theme::ERROR));
        }

        // Context-aware + Plugin autocomplete chips
        let mut accept: Option<String> = None;
        let combined_suggestions: Vec<(String, String)> = if !dom_suggestions.is_empty() {
            dom_suggestions
        } else {
            buf.completions.clone()
        };
        if !combined_suggestions.is_empty() {
            egui::ScrollArea::horizontal().id_salt(("ac_scroll", r)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (idx, (label, insert)) in combined_suggestions.iter().take(12).enumerate() {
                        let btn = if idx == 0 {
                            egui::Button::new(
                                RichText::new(format!("⇥ {label}"))
                                    .monospace()
                                    .size(13.0)
                                    .color(Color32::from_rgb(140, 225, 150)),
                            )
                            .fill(Color32::from_rgb(32, 52, 40))
                        } else {
                            egui::Button::new(RichText::new(label).monospace().size(13.0))
                        };
                        if ui.add(btn).clicked() {
                            accept = Some(insert.clone());
                            ui.ctx().memory_mut(|m| m.request_focus(editor_id));
                        }
                    }
                });
            });
        }
        // Quick mobile action bar for newline (↵), tab/ghost accept (⇥), and cursor movement
        // always visible on iOS/Android even when keyboard height is 0.
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            egui::ScrollArea::horizontal().id_salt(("mobile_top_quick_bar", r)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    if ui
                        .button(
                            RichText::new("↵ Enter")
                                .monospace()
                                .size(13.0)
                                .strong()
                                .color(Color32::from_rgb(130, 200, 255)),
                        )
                        .on_hover_text("Xuống dòng (Enter)")
                        .clicked()
                    {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("mobile_luau_newline"), true);
                        });
                        ui.ctx().memory_mut(|m| m.request_focus(editor_id));
                    }
                    if let Some((lbl, ins)) = combined_suggestions.first() {
                        if ui
                            .button(
                                RichText::new(format!("⇥ Chèn: {lbl}"))
                                    .monospace()
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::from_rgb(130, 220, 145)),
                            )
                            .clicked()
                        {
                            accept = Some(ins.clone());
                            ui.ctx().memory_mut(|m| m.request_focus(editor_id));
                        }
                    }
                    for (label, snip) in [
                        ("Tab", "    "),
                        ("local", "local "),
                        ("function", "function "),
                        ("if", "if "),
                        ("then", "then"),
                        ("end", "end"),
                        ("()", "()"),
                        ("\"", "\""),
                        ("=", " = "),
                        (":", ":"),
                        (".", "."),
                    ] {
                        if ui.button(RichText::new(label).monospace().size(12.5)).clicked() {
                            ui.ctx().data_mut(|d| {
                                d.insert_temp(egui::Id::new("mobile_luau_snippet"), snip.to_string());
                            });
                            ui.ctx().memory_mut(|m| m.request_focus(editor_id));
                        }
                    }
                });
            });
        }
        ui.separator();

        // Studio Settings > Script Editor > Font (size), Background Color,
        // Text Wrapping.
        let base_font = crate::studio_settings::font_px("Studio", "Font");
        let is_portrait = ui.ctx().content_rect().width() < ui.ctx().content_rect().height();
        let default_px = if is_portrait { base_font.max(14.5) } else { base_font };
        let font_px = ui.ctx().data(|d| d.get_temp::<f32>(font_override_id).unwrap_or(default_px));
        let font = FontId::monospace(font_px);
        let row_h = ui.fonts_mut(|f| f.row_height(&font));
        let wrap_id = egui::Id::new("mobile_script_wrap_override");
        let wrap = ui.ctx().data(|d| d.get_temp::<bool>(wrap_id))
            .unwrap_or_else(|| crate::studio_settings::flag("Studio", "Text Wrapping", false));
        let bg = crate::studio_settings::script_colors().background;
        ui.painter().rect_filled(ui.available_rect_before_wrap(), 0.0, bg);
        let error_line = buf.error.as_ref().map(|e| e.0);
        let mut changed = false;
        let prev_text = buf.text.clone();
        let mut scroll_area = egui::ScrollArea::both()
            .auto_shrink([false, false])
            .id_salt(("script_scroll", r));
        if let Some(target_line) = jump_to_line {
            let y = ((target_line.saturating_sub(2)) as f32) * row_h;
            scroll_area = scroll_area.vertical_scroll_offset(y.max(0.0));
        }
        scroll_area.show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    // Line numbers + Code Folding markers (▾ / ▸).
                    let fold_id = egui::Id::new(("mobile_code_folds", r));
                    let mut folded = ui.ctx().data(|d| d.get_temp::<std::collections::HashSet<usize>>(fold_id).unwrap_or_default());
                    let foldable = scan_foldable_lines(&buf.text);
                    let lines = buf.text.split('\n').count().max(1);
                    let digits = lines.to_string().len().max(2);
                    let mut gutter = LayoutJob::default();
                    for n in 1..=lines {
                        let c = if Some(n) == error_line {
                            theme::ERROR
                        } else {
                            theme::TEXT_DIM
                        };
                        let fold_sym = if foldable.contains_key(&n) {
                            if folded.contains(&n) { "▸" } else { "▾" }
                        } else {
                            " "
                        };
                        let nl = if n == lines { "" } else { "\n" };
                        gutter.append(
                            &format!("{fold_sym}{n:>digits$}{nl}"),
                            0.0,
                            TextFormat {
                                font_id: font.clone(),
                                color: c,
                                ..Default::default()
                            },
                        );
                    }
                    let galley = ui.fonts_mut(|f| f.layout_job(gutter));
                    let (rect, gutter_resp) = ui.allocate_exact_size(
                        egui::vec2(galley.size().x + 12.0, galley.size().y.max(row_h)),
                        egui::Sense::click(),
                    );
                    ui.painter().rect_filled(rect, 0.0, theme::PANEL_DARK);
                    ui.painter()
                        .galley(rect.min + egui::vec2(4.0, 2.0), galley, theme::TEXT_DIM);
                    if gutter_resp.clicked() {
                        if let Some(pos) = gutter_resp.interact_pointer_pos() {
                            let clicked_line = (((pos.y - rect.min.y - 2.0) / row_h).floor().max(0.0) as usize) + 1;
                            if foldable.contains_key(&clicked_line) {
                                if !folded.remove(&clicked_line) {
                                    folded.insert(clicked_line);
                                }
                                ui.ctx().data_mut(|d| d.insert_temp(fold_id, folded.clone()));
                            }
                        }
                    }

                    let mut layouter = |ui: &egui::Ui,
                                        text: &dyn egui::TextBuffer,
                                        wrap_w: f32|
                     -> Arc<egui::Galley> {
                        let mut job = highlight(text.as_str(), None, font.clone());
                        job.wrap.max_width = if wrap { wrap_w } else { f32::INFINITY };
                        ui.fonts_mut(|f| f.layout_job(job))
                    };
                    let resp = ui.add(
                        egui::TextEdit::multiline(&mut buf.text)
                            .id(editor_id)
                            .code_editor()
                            .frame(egui::Frame::NONE)
                            .desired_width(if wrap { ui.available_width().max(120.0) } else { 0.0 })
                            .desired_rows(lines.max(24))
                            .lock_focus(true)
                            .layouter(&mut layouter),
                    );
                    if resp.changed() {
                        changed = true;
                        buf.edited = Instant::now();
                    }
                    #[cfg(target_os = "android")]
                    if resp.clicked() {
                        crate::android_ime::force_show_keyboard();
                    }
                    // VS Code-style Error Lens: show error inline next to the offending line.
                    if let Some((err_line, err_msg)) = &buf.error {
                        let line_idx = err_line.saturating_sub(1);
                        let line_str = buf.text.lines().nth(line_idx).unwrap_or("");
                        let line_w = ui.fonts_mut(|f| {
                            f.layout_no_wrap(line_str.to_owned(), font.clone(), Color32::WHITE).size().x
                        });
                        let y = resp.rect.top() + (line_idx as f32) * row_h;
                        let pos = egui::pos2(resp.rect.left() + line_w + 14.0, y);
                        ui.painter().text(
                            pos,
                            egui::Align2::LEFT_TOP,
                            format!("⚠ {err_msg}"),
                            font.clone(),
                            Color32::from_rgba_unmultiplied(245, 105, 105, 200),
                        );
                    }
                    // Roblox Studio-style dim ghost autocomplete suffix at cursor.
                    // Tapping the ghost text or pressing Tab / Right Arrow at EOL accepts it!
                    if resp.has_focus() || pre_cursor_idx.is_some() {
                        if let Some((_, top_insert)) = combined_suggestions.first() {
                            if let Some(st) = egui::TextEdit::load_state(ui.ctx(), editor_id) {
                                if let Some(cr) = st.cursor.char_range() {
                                    if cr.primary.index == cr.secondary.index {
                                        let ci = cr.primary.index.0;
                                        let before: String = buf.text.chars().take(ci).collect();
                                        let word_start = before
                                            .rfind(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                                            .map(|i| i + 1)
                                            .unwrap_or(0);
                                        let typed = &before[word_start..];
                                        let typed_lower = typed.to_ascii_lowercase();
                                        let top_lower = top_insert.to_ascii_lowercase();
                                        let suffix_opt: Option<&str> = if !typed.is_empty()
                                            && top_lower.starts_with(&typed_lower)
                                            && top_insert.len() > typed.len()
                                        {
                                            Some(&top_insert[typed.len()..])
                                        } else if typed.is_empty()
                                            && (before.ends_with('.') || before.ends_with(':'))
                                        {
                                            Some(top_insert.as_str())
                                        } else {
                                            None
                                        };
                                        if let Some(suffix) = suffix_opt {
                                            let suffix_first_line = suffix.lines().next().unwrap_or(suffix);
                                            let (line_1, col_1) = crate::plugin_host::script_docs::pos_of(&buf.text, ci);
                                            let line_str = buf.text.lines().nth(line_1.saturating_sub(1) as usize).unwrap_or("");
                                            let prefix_on_line: String = line_str.chars().take(col_1.saturating_sub(1) as usize).collect();
                                            let x_off = ui.fonts_mut(|f| {
                                                f.layout_no_wrap(prefix_on_line, font.clone(), Color32::WHITE).size().x
                                            });
                                            let ghost_galley = ui.fonts_mut(|f| {
                                                f.layout_no_wrap(
                                                    format!("{suffix_first_line} ⇥"),
                                                    font.clone(),
                                                    Color32::from_rgba_unmultiplied(165, 175, 195, 145),
                                                )
                                            });
                                            let y = resp.rect.top() + (line_1.saturating_sub(1) as f32) * row_h;
                                            let ghost_pos = egui::pos2(resp.rect.left() + x_off + 2.0, y);
                                            let ghost_rect = egui::Rect::from_min_size(
                                                ghost_pos,
                                                egui::vec2(ghost_galley.size().x.max(48.0), row_h.max(24.0)),
                                            );
                                            ui.painter().rect_filled(
                                                ghost_rect.expand2(egui::vec2(3.0, 1.0)),
                                                3.0,
                                                Color32::from_rgba_unmultiplied(60, 70, 90, 55),
                                            );
                                            ui.painter().galley(
                                                ghost_pos,
                                                ghost_galley,
                                                Color32::from_rgba_unmultiplied(165, 175, 195, 145),
                                            );
                                            let tapped_ghost = ui.input(|i| {
                                                i.pointer.any_released()
                                                    && i.pointer
                                                        .interact_pos()
                                                        .is_some_and(|p| ghost_rect.expand(6.0).contains(p))
                                            });
                                            let tab_pressed = ui.input(|i| i.key_pressed(egui::Key::Tab));
                                            if tapped_ghost || tab_pressed {
                                                accept = Some(top_insert.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if let Some(line) = goto {
                        let y = resp.rect.top() + line.saturating_sub(1) as f32 * row_h;
                        ui.scroll_to_rect(egui::Rect::from_min_size(egui::pos2(resp.rect.left(), y), egui::vec2(1.0, row_h)), Some(egui::Align::Center));
                    }
                });
            });
        // The cursor / selection as the plugins' ScriptDocument sees it
        // (Studio's 1-based line, character), and selections they set.
        let mut state = egui::TextEdit::load_state(ui.ctx(), editor_id);
        let mut word_end: Option<usize> = None;
        if let Some(st) = state.as_mut() {
            if let Some(cr) = st.cursor.char_range() {
                let (a, b) = (cr.primary.index.0, cr.secondary.index.0);
                let pa = crate::plugin_host::script_docs::pos_of(&buf.text, a);
                let pb = crate::plugin_host::script_docs::pos_of(&buf.text, b);
                crate::plugin_host::script_docs::set_selection(r, (pa.0, pa.1, pb.0, pb.1), true);
                if a == b {
                    word_end = Some(a);
                }
            }
        }
        if let (Some(line), Some(mut st)) = (goto, state.clone()) {
            // (the profiler's jump: the cursor at the line's start, focused)
            let at = crate::plugin_host::script_docs::index_of(&buf.text, line as i64, 1);
            st.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(at))));
            st.store(ui.ctx(), editor_id);
            ui.ctx().memory_mut(|m| m.request_focus(editor_id));
            self.scripts.goto.remove(&r);
        }
        if let Some((sel, mut st)) = crate::plugin_host::script_docs::take_selection_request(r).zip(state) {
            let a = crate::plugin_host::script_docs::index_of(&buf.text, sel.0, sel.1);
            let b = crate::plugin_host::script_docs::index_of(&buf.text, sel.2, sel.3);
            st.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(b), egui::text::CCursor::new(a))));
            st.store(ui.ctx(), editor_id);
        }
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            let kb_h = crate::platform::keyboard_height_points(ui.ctx());
            if kb_h > 0.0 {
                let screen = ui.ctx().content_rect();
                let bar_h = 40.0;
                let y = (screen.bottom() - kb_h - bar_h).max(40.0);
                egui::Area::new(egui::Id::new("mobile_kb_accessory_bar"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(egui::pos2(0.0, y))
                    .show(ui.ctx(), |ui| {
                        egui::Frame::new()
                            .fill(theme::PANEL_DARK)
                            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(55, 60, 72)))
                            .inner_margin(egui::Margin::symmetric(6, 5))
                            .show(ui, |ui| {
                                ui.set_min_width(screen.width());
                                ui.set_max_width(screen.width());
                                egui::ScrollArea::horizontal().id_salt("kb_bar_scroll").show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(5.0, 0.0);
                                        if let Some((lbl, ins)) = combined_suggestions.first() {
                                            if ui.button(RichText::new(format!("⇥ {lbl}")).monospace().color(Color32::from_rgb(130, 210, 140))).clicked() {
                                                accept = Some(ins.clone());
                                                ui.ctx().memory_mut(|m| m.request_focus(editor_id));
                                            }
                                        }
                                        for (label, dir) in [("◀", "left"), ("▶", "right"), ("▲", "up"), ("▼", "down")] {
                                            if ui.button(RichText::new(label).monospace().size(13.5).strong()).clicked() {
                                                ui.ctx().data_mut(|d| {
                                                    d.insert_temp(egui::Id::new("mobile_cursor_move"), dir.to_string());
                                                });
                                                ui.ctx().memory_mut(|m| m.request_focus(editor_id));
                                            }
                                        }
                                        for (label, snip) in [
                                            ("Tab", "    "),
                                            ("local", "local "),
                                            ("function", "function "),
                                            ("end", "end"),
                                            ("if", "if "),
                                            ("then", "then"),
                                            ("else", "else"),
                                            ("for", "for "),
                                            ("return", "return "),
                                            ("(", "("),
                                            (")", ")"),
                                            ("\"", "\""),
                                            ("=", " = "),
                                            ("==", " == "),
                                            ("~=", " ~= "),
                                            (":", ":"),
                                            (".", "."),
                                            ("..", " .. "),
                                            ("{}", "{}"),
                                            ("[]", "[]"),
                                            ("--", "-- "),
                                        ] {
                                            if ui.button(RichText::new(label).monospace().size(13.0)).clicked() {
                                                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("mobile_luau_snippet"), snip.to_string()));
                                                ui.ctx().memory_mut(|m| m.request_focus(editor_id));
                                            }
                                        }
                                        if ui.button("✓").clicked() {
                                            ui.ctx().memory_mut(|m| m.surrender_focus(editor_id));
                                        }
                                    });
                                });
                            });
                    });
            }
        }
        if let Some(insert) = accept {
            let end = word_end
                .or(pre_cursor_idx)
                .unwrap_or_else(|| buf.text.chars().count());
            let chars: Vec<char> = buf.text.chars().collect();
            let end = end.min(chars.len());
            // Strip any leading `\t` inserted by a simultaneous Tab key press
            let mut actual_end = end;
            let mut start = end;
            if start > 0 && chars[start - 1] == '\t' {
                start -= 1;
                actual_end = end;
            }
            while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
                start -= 1;
            }
            let ins_chars: Vec<char> = insert.chars().collect();
            let new_cursor = start + ins_chars.len();
            buf.text = chars[..start]
                .iter()
                .chain(ins_chars.iter())
                .chain(chars[actual_end..].iter())
                .collect();
            if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), editor_id) {
                // Place cursor inside `()` or `("")` if the completion ends with empty parens/quotes
                let offset_back = if insert.ends_with("(\"\")") {
                    2
                } else if insert.ends_with("()") {
                    1
                } else {
                    0
                };
                let c = egui::text::CCursor::new(new_cursor.saturating_sub(offset_back));
                st.cursor.set_char_range(Some(egui::text::CCursorRange::one(c)));
                st.store(ui.ctx(), editor_id);
            }
            ui.ctx().memory_mut(|m| m.request_focus(editor_id));
            buf.completions.clear();
            buf.edited = Instant::now();
            changed = true;
        }
        if ui
            .ctx()
            .data_mut(|d| d.remove_temp::<bool>(egui::Id::new("mobile_luau_newline")))
            .unwrap_or(false)
        {
            let insert_at = word_end
                .or(pre_cursor_idx)
                .unwrap_or_else(|| buf.text.chars().count());
            let mut chars: Vec<char> = buf.text.chars().collect();
            let pos = insert_at.min(chars.len());
            chars.insert(pos, '\n');
            buf.text = chars.into_iter().collect();
            let new_pos = pos + 1;
            if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), editor_id) {
                let c = egui::text::CCursor::new(new_pos);
                st.cursor.set_char_range(Some(egui::text::CCursorRange::one(c)));
                st.store(ui.ctx(), editor_id);
            }
            let _ = apply_smart_indent(&mut buf.text, new_pos, ui.ctx(), editor_id);
            ui.ctx().memory_mut(|m| m.request_focus(editor_id));
            buf.edited = Instant::now();
            changed = true;
        }
        if let Some(snippet) = ui.ctx().data_mut(|d| d.remove_temp::<String>(egui::Id::new("mobile_luau_snippet"))) {
            let insert_at = word_end.unwrap_or_else(|| buf.text.chars().count());
            let mut chars: Vec<char> = buf.text.chars().collect();
            let pos = insert_at.min(chars.len());
            let snippet_chars: Vec<char> = snippet.chars().collect();
            let new_pos = pos + snippet_chars.len();
            chars.splice(pos..pos, snippet_chars);
            buf.text = chars.into_iter().collect();
            if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), editor_id) {
                let c = egui::text::CCursor::new(new_pos);
                st.cursor.set_char_range(Some(egui::text::CCursorRange::one(c)));
                st.store(ui.ctx(), editor_id);
            }
            ui.ctx().memory_mut(|m| m.request_focus(editor_id));
            buf.edited = Instant::now();
            changed = true;
        }
        if let Some(dir) = ui.ctx().data_mut(|d| d.remove_temp::<String>(egui::Id::new("mobile_cursor_move"))) {
            move_cursor_dir(&buf.text, &dir, ui.ctx(), editor_id);
            ui.ctx().memory_mut(|m| m.request_focus(editor_id));
        }
        if let Some(line) = jump_to_line {
            let idx = crate::plugin_host::script_docs::index_of(&buf.text, line as i64, 1);
            if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), editor_id) {
                st.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(idx))));
                st.store(ui.ctx(), editor_id);
            }
            ui.ctx().memory_mut(|m| m.request_focus(editor_id));
        }
        if toggle_comment {
            let cursor_idx = word_end.unwrap_or(0);
            if let Some(new_idx) = toggle_comment_at_cursor(&mut buf.text, cursor_idx) {
                if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), editor_id) {
                    st.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(new_idx))));
                    st.store(ui.ctx(), editor_id);
                }
                ui.ctx().memory_mut(|m| m.request_focus(editor_id));
                buf.edited = Instant::now();
                changed = true;
            }
        }
        if let Some(query) = find_next {
            let start_char = word_end.map(|i| i + 1).unwrap_or(0);
            let start_byte = buf.text.char_indices().nth(start_char).map(|(b, _)| b).unwrap_or(0);
            let found_byte = buf.text[start_byte..]
                .find(&query)
                .map(|b| start_byte + b)
                .or_else(|| buf.text.find(&query));
            if let Some(b_idx) = found_byte {
                let c_idx = buf.text[..b_idx].chars().count();
                let end_idx = c_idx + query.chars().count();
                if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), editor_id) {
                    st.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(c_idx),
                        egui::text::CCursor::new(end_idx),
                    )));
                    st.store(ui.ctx(), editor_id);
                }
                ui.ctx().memory_mut(|m| m.request_focus(editor_id));
            }
        }
        if ui.ctx().data_mut(|d| d.remove_temp::<bool>(egui::Id::new("mobile_select_word"))).unwrap_or(false) {
            select_word_at_cursor(&buf.text, ui.ctx(), editor_id);
            ui.ctx().memory_mut(|m| m.request_focus(editor_id));
        }
        if ui.ctx().data_mut(|d| d.remove_temp::<bool>(egui::Id::new("mobile_dup_line"))).unwrap_or(false) {
            if duplicate_line_at_cursor(&mut buf.text, ui.ctx(), editor_id) {
                buf.edited = Instant::now();
                changed = true;
            }
            ui.ctx().memory_mut(|m| m.request_focus(editor_id));
        }
        // Record copied text into 5-item Clipboard Ring
        if ui.ctx().input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy | egui::Event::Cut))) {
            if let Some(st) = egui::TextEdit::load_state(ui.ctx(), editor_id) {
                if let Some(cr) = st.cursor.char_range() {
                    let (a, b) = (cr.primary.index.0, cr.secondary.index.0);
                    let (lo, hi) = (a.min(b), a.max(b));
                    if lo < hi {
                        let copied: String = buf.text.chars().skip(lo).take(hi - lo).collect();
                        if !copied.trim().is_empty() {
                            let clip_id = egui::Id::new("mobile_clipboard_ring");
                            ui.ctx().data_mut(|d| {
                                let mut ring = d.get_temp::<Vec<String>>(clip_id).unwrap_or_default();
                                ring.retain(|x| x != &copied);
                                ring.insert(0, copied);
                                ring.truncate(5);
                                d.insert_temp(clip_id, ring);
                            });
                        }
                    }
                }
            }
        }
        // Auto-Close Pairs (), {}, [], "", '' and Smart Auto-Indent & Auto-End
        if changed && buf.text.len() == prev_text.len() + 1 {
            if let Some(cursor) = egui::TextEdit::load_state(ui.ctx(), editor_id)
                .and_then(|st| st.cursor.char_range())
                .map(|cr| cr.primary.index.0)
            {
                if apply_auto_close_pair(&mut buf.text, cursor, ui.ctx(), editor_id) {
                    buf.edited = Instant::now();
                } else if apply_smart_indent(&mut buf.text, cursor, ui.ctx(), editor_id) {
                    buf.edited = Instant::now();
                }
            }
        }
        if changed {
            let text = buf.text.clone();
            // ScriptEditorService's TextDocumentDidChange for the person's typing.
            if let Some(c) = crate::plugin_host::script_docs::diff(&source, &text) {
                crate::plugin_host::script_docs::changed(r, vec![c]);
            }
            // Plugins' autocomplete for the word being typed.
            let pos = word_end.map(|i| crate::plugin_host::script_docs::pos_of(&text, i));
            self.game.doc.set_props(
                "Edit script",
                vec![(r, rbx_dom_weak::ustr("Source"), Variant::String(text.clone()))],
                Some(format!("source:{r:?}")),
            );
            let typing_word = word_end.is_some_and(|i| {
                let c: Vec<char> = text.chars().collect();
                i > 0 && (c[i - 1].is_alphanumeric() || c[i - 1] == '_' || c[i - 1] == '.' || c[i - 1] == ':')
            });
            let items = match (pos, typing_word) {
                (Some((line, ch)), true) => self
                    .plugins
                    .with_edit(&mut self.game.doc, |rt| {
                        let lua = &rt.lua;
                        if crate::plugin_host::api::script_editor::has_callbacks(lua).0 {
                            crate::plugin_host::api::script_editor::autocomplete(lua, r, line, ch).unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    })
                    .unwrap_or_default(),
                _ => Vec::new(),
            };
            if let Some(b) = self.scripts.bufs.get_mut(&r) {
                b.completions = items;
            }
        }
        if buf_needs_repaint(self.scripts.bufs.get(&r)) {
            ui.ctx().request_repaint_after(Duration::from_millis(300));
        }
    }

    /// Opens the script at instance path `path` (as the profiler names it),
    /// at `line` if given. False when no script has that path.
    pub fn open_script_named(&mut self, path: &str, line: Option<u32>) -> bool {
        let found = self
            .game
            .doc
            .dom
            .descendants()
            .filter(|i| matches!(i.class.as_str(), "Script" | "LocalScript" | "ModuleScript"))
            .map(|i| i.referent())
            .find(|&r| self.full_name(r) == path);
        match found {
            Some(r) => {
                self.open_script.push(r);
                if let Some(l) = line {
                    self.scripts.goto.insert(r, l as usize);
                }
                true
            }
            None => {
                self.warn(format!("MicroProfiler: no script named {path} in the DataModel being shown"));
                false
            }
        }
    }

    fn full_name(&self, r: Ref) -> String {
        let root = self.game.doc.dom.root_ref();
        let mut parts = Vec::new();
        let mut cur = r;
        while let Some(i) = self.game.doc.dom.get_by_ref(cur) {
            if cur == root {
                break;
            }
            parts.push(i.name.clone());
            cur = i.parent();
        }
        parts.reverse();
        parts.join(".")
    }
}

fn buf_needs_repaint(b: Option<&ScriptBuf>) -> bool {
    b.is_some_and(|b| b.checked.as_deref() != Some(b.text.as_str()))
}

fn mobile_luau_suggestions(
    dom: &rbx_dom_weak::WeakDom,
    script_ref: Ref,
    before_cursor: &str,
) -> Vec<(String, String)> {
    let line = before_cursor.rsplit('\n').next().unwrap_or(before_cursor);
    let expr: String = line
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | ':'))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if expr.is_empty() {
        return Vec::new();
    }

    // 1. Explorer / DataModel path completion (e.g. `workspace.`, `script.Parent.`, `ReplicatedStorage.`)
    if let Some((chain, filter)) = expr.rsplit_once('.') {
        let mut segs = chain.split('.');
        if let Some(first) = segs.next() {
            let mut cur: Option<Ref> = if first.eq_ignore_ascii_case("workspace") {
                dom.root()
                    .children()
                    .iter()
                    .copied()
                    .find(|&id| dom.get_by_ref(id).is_some_and(|i| i.class == "Workspace"))
            } else if first == "script" {
                Some(script_ref)
            } else {
                dom.root()
                    .children()
                    .iter()
                    .copied()
                    .find(|&id| dom.get_by_ref(id).is_some_and(|i| i.name == first))
            };
            for seg in segs {
                let Some(c) = cur else { break };
                cur = if seg == "Parent" {
                    dom.get_by_ref(c).map(|i| i.parent()).filter(|&p| p != Ref::none())
                } else {
                    dom.get_by_ref(c).and_then(|inst| {
                        inst.children()
                            .iter()
                            .copied()
                            .find(|&cid| dom.get_by_ref(cid).is_some_and(|ch| ch.name == seg))
                    })
                };
            }
            if let Some(target_ref) = cur {
                if let Some(inst) = dom.get_by_ref(target_ref) {
                    let fl = filter.to_ascii_lowercase();
                    let mut out: Vec<(String, String)> = inst
                        .children()
                        .iter()
                        .filter_map(|&cid| dom.get_by_ref(cid))
                        .filter(|ch| fl.is_empty() || ch.name.to_ascii_lowercase().starts_with(&fl))
                        .map(|ch| (format!("{} ({})", ch.name, ch.class), ch.name.clone()))
                        .collect();
                    if !out.is_empty() {
                        return out.into_iter().take(12).collect();
                    }
                    // Also allow `.Parent` on `script`
                    if target_ref == script_ref && ("parent".starts_with(&fl)) {
                        out.push(("Parent".to_string(), "Parent".to_string()));
                        return out;
                    }
                }
            }
        }
    }

    // 1b. Rustudio Reflection Intellisense for `.` or `:` (Properties, Methods, Events)
    if let Some((obj, sep, filter)) = expr
        .rsplit_once(':')
        .map(|(o, f)| (o, ':', f))
        .or_else(|| expr.rsplit_once('.').map(|(o, f)| (o, '.', f)))
    {
        let base = obj.rsplit(['.', ':', '(', ' ']).next().unwrap_or(obj);
        let members = rustudio_reflection_members(base, dom);
        if !members.is_empty() {
            let fl = filter.to_ascii_lowercase();
            let mut out: Vec<(String, String)> = members
                .iter()
                .filter(|(name, kind)| {
                    (sep == ':' && (*kind == "Method" || *kind == "Service"))
                        || (sep == '.' && *kind != "Method")
                })
                .filter(|(name, _)| fl.is_empty() || name.to_ascii_lowercase().starts_with(&fl))
                .map(|(name, kind)| {
                    let insert = if *kind == "Method" {
                        format!("{name}()")
                    } else {
                        (*name).to_owned()
                    };
                    (format!("{name} [{kind}]"), insert)
                })
                .take(12)
                .collect();
            if !out.is_empty() {
                return out;
            }
        }
    }

    // 2. Standard Luau & Roblox API completions
    let word = expr.rsplit(['.', ':']).next().unwrap_or(&expr).to_ascii_lowercase();
    if word.is_empty() {
        return Vec::new();
    }
    let std_items: &[(&str, &str)] = &[
        ("game:GetService(\"\")", "game:GetService(\"\")"),
        ("Instance.new(\"\")", "Instance.new(\"\")"),
        ("task.wait()", "task.wait()"),
        ("task.spawn()", "task.spawn()"),
        ("Vector3.new()", "Vector3.new()"),
        ("CFrame.new()", "CFrame.new()"),
        ("Color3.fromRGB()", "Color3.fromRGB()"),
        ("TweenService:Create()", "TweenService:Create()"),
        ("RunService.Heartbeat:Connect()", "RunService.Heartbeat:Connect(function(dt)\n  \nend)"),
        ("print()", "print()"),
        ("warn()", "warn()"),
        ("local ", "local "),
        ("function ", "function ()\n  \nend"),
        ("workspace", "workspace"),
    ];
    let mut matches: Vec<(String, String)> = std_items
        .iter()
        .filter(|(label, _)| label.to_ascii_lowercase().contains(&word))
        .map(|(l, ins)| ((*l).to_string(), (*ins).to_string()))
        .collect();
    matches.sort_by_key(|(label, _)| !label.to_ascii_lowercase().starts_with(&word));
    matches
}

fn move_cursor_dir(text: &str, dir: &str, ctx: &egui::Context, editor_id: egui::Id) {
    let Some(mut st) = egui::TextEdit::load_state(ctx, editor_id) else { return };
    let cur = st.cursor.char_range().map(|cr| cr.primary.index.0).unwrap_or(0);
    let total = text.chars().count();
    let new_idx = match dir {
        "left" => cur.saturating_sub(1),
        "right" => (cur + 1).min(total),
        "up" => {
            let (line, col) = crate::plugin_host::script_docs::pos_of(text, cur);
            if line <= 1 {
                0
            } else {
                crate::plugin_host::script_docs::index_of(text, line - 1, col)
            }
        }
        "down" => {
            let (line, col) = crate::plugin_host::script_docs::pos_of(text, cur);
            crate::plugin_host::script_docs::index_of(text, line + 1, col)
        }
        _ => cur,
    };
    st.cursor
        .set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(new_idx))));
    st.store(ctx, editor_id);
}

fn toggle_comment_at_cursor(text: &mut String, cursor_char_idx: usize) -> Option<usize> {
    let (line_no, _) = crate::plugin_host::script_docs::pos_of(text, cursor_char_idx);
    let mut lines: Vec<String> = text.split('\n').map(String::from).collect();
    let idx = (line_no.max(1) - 1) as usize;
    let line = lines.get_mut(idx)?;
    let indent_len = line.chars().take_while(|c| *c == ' ' || *c == '\t').count();
    let indent: String = line.chars().take(indent_len).collect();
    let rest: String = line.chars().skip(indent_len).collect();
    let new_cursor = if let Some(stripped) = rest.strip_prefix("-- ") {
        *line = format!("{indent}{stripped}");
        cursor_char_idx.saturating_sub(3)
    } else if let Some(stripped) = rest.strip_prefix("--") {
        *line = format!("{indent}{stripped}");
        cursor_char_idx.saturating_sub(2)
    } else {
        *line = format!("{indent}-- {rest}");
        cursor_char_idx + 3
    };
    *text = lines.join("\n");
    Some(new_cursor)
}

fn apply_smart_indent(text: &mut String, cursor: usize, ctx: &egui::Context, editor_id: egui::Id) -> bool {
    let chars: Vec<char> = text.chars().collect();
    if cursor == 0 || cursor > chars.len() || chars[cursor - 1] != '\n' {
        return false;
    }
    // Find previous line before the just-inserted '\n'
    let mut line_start = cursor - 1;
    while line_start > 0 && chars[line_start - 1] != '\n' {
        line_start -= 1;
    }
    let prev_line: String = chars[line_start..cursor - 1].iter().collect();
    let indent: String = prev_line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
    let trimmed = prev_line.trim();
    let opens_block = trimmed.ends_with("then")
        || trimmed.ends_with("do")
        || (trimmed.contains("function") && trimmed.ends_with(')'));

    let insert = if opens_block {
        format!("{indent}  \n{indent}end")
    } else if !indent.is_empty() {
        indent.clone()
    } else {
        return false;
    };

    let cursor_offset = if opens_block { indent.chars().count() + 2 } else { indent.chars().count() };
    let mut new_chars = chars;
    let ins_chars: Vec<char> = insert.chars().collect();
    new_chars.splice(cursor..cursor, ins_chars);
    *text = new_chars.into_iter().collect();

    if let Some(mut st) = egui::TextEdit::load_state(ctx, editor_id) {
        let c = egui::text::CCursor::new(cursor + cursor_offset);
        st.cursor.set_char_range(Some(egui::text::CCursorRange::one(c)));
        st.store(ctx, editor_id);
    }
    true
}

fn apply_auto_close_pair(text: &mut String, cursor: usize, ctx: &egui::Context, editor_id: egui::Id) -> bool {
    let mut chars: Vec<char> = text.chars().collect();
    if cursor == 0 || cursor > chars.len() {
        return false;
    }
    let ch = chars[cursor - 1];
    let close = match ch {
        '(' => ')',
        '{' => '}',
        '[' => ']',
        '"' => '"',
        '\'' => '\'',
        _ => return false,
    };
    if chars.get(cursor).copied() == Some(close) {
        return false;
    }
    chars.insert(cursor, close);
    *text = chars.into_iter().collect();
    if let Some(mut st) = egui::TextEdit::load_state(ctx, editor_id) {
        let c = egui::text::CCursor::new(cursor);
        st.cursor.set_char_range(Some(egui::text::CCursorRange::one(c)));
        st.store(ctx, editor_id);
    }
    true
}

fn select_word_at_cursor(text: &str, ctx: &egui::Context, editor_id: egui::Id) {
    let Some(mut st) = egui::TextEdit::load_state(ctx, editor_id) else { return };
    let cur = st.cursor.char_range().map(|cr| cr.primary.index.0).unwrap_or(0);
    let chars: Vec<char> = text.chars().collect();
    let mut start = cur.min(chars.len());
    let mut end = start;
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }
    st.cursor.set_char_range(Some(egui::text::CCursorRange::two(
        egui::text::CCursor::new(start),
        egui::text::CCursor::new(end),
    )));
    st.store(ctx, editor_id);
}

fn duplicate_line_at_cursor(text: &mut String, ctx: &egui::Context, editor_id: egui::Id) -> bool {
    let Some(mut st) = egui::TextEdit::load_state(ctx, editor_id) else { return false };
    let cur = st.cursor.char_range().map(|cr| cr.primary.index.0).unwrap_or(0);
    let (line_no, col) = crate::plugin_host::script_docs::pos_of(text, cur);
    let mut lines: Vec<String> = text.split('\n').map(String::from).collect();
    let idx = (line_no.max(1) - 1) as usize;
    let Some(line) = lines.get(idx).cloned() else { return false };
    lines.insert(idx + 1, line);
    *text = lines.join("\n");
    let new_idx = crate::plugin_host::script_docs::index_of(text, line_no + 1, col);
    st.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(new_idx))));
    st.store(ctx, editor_id);
    true
}

fn scan_function_outline(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (idx, raw) in text.lines().enumerate() {
        let t = raw.trim();
        if t.starts_with("local function ") || t.starts_with("function ") {
            out.push((idx + 1, t.chars().take(48).collect()));
        } else if t.starts_with("--[[") || t.starts_with("--//") {
            out.push((idx + 1, t.chars().take(48).collect()));
        }
    }
    out
}

fn scan_foldable_lines(text: &str) -> std::collections::HashMap<usize, usize> {
    let mut map = std::collections::HashMap::new();
    let mut stack: Vec<usize> = Vec::new();
    for (idx, raw) in text.lines().enumerate() {
        let line_no = idx + 1;
        let t = raw.trim();
        if t.starts_with("local function ") || t.starts_with("function ") || t.ends_with('{') {
            stack.push(line_no);
        } else if (t == "end" || t.starts_with("end)") || t == "}" || t.starts_with("},")) && !stack.is_empty() {
            if let Some(start) = stack.pop() {
                if line_no > start {
                    map.insert(start, line_no);
                }
            }
        }
    }
    map
}

fn rustudio_reflection_members(ident: &str, dom: &rbx_dom_weak::WeakDom) -> &'static [(&'static str, &'static str)] {
    let lower = ident.to_ascii_lowercase();
    let class_guess = if lower.contains("humanoid") {
        "Humanoid"
    } else if lower.contains("tween") {
        "TweenService"
    } else if lower.contains("player") {
        "Players"
    } else if lower.contains("run") {
        "RunService"
    } else if lower.contains("uis") || lower.contains("userinput") {
        "UserInputService"
    } else if lower.contains("remote") || lower.contains("event") {
        "RemoteEvent"
    } else if lower.contains("cam") {
        "Camera"
    } else if lower.contains("motor") {
        "Motor6D"
    } else if lower.contains("sound") {
        "Sound"
    } else if lower.contains("label") || lower.contains("text") {
        "TextLabel"
    } else if lower.contains("frame") || lower.contains("ui") {
        "Frame"
    } else if lower.contains("part") || lower.contains("arm") || lower.contains("torso") || lower.contains("head") {
        "BasePart"
    } else {
        let found = dom.descendants().find(|i| i.name.eq_ignore_ascii_case(ident)).map(|i| i.class.as_str());
        match found {
            Some("Humanoid") => "Humanoid",
            Some("RemoteEvent") => "RemoteEvent",
            Some("Model") => "Model",
            Some("Part") | Some("MeshPart") | Some("WedgePart") => "BasePart",
            Some("Motor6D") => "Motor6D",
            Some("Sound") => "Sound",
            _ => "",
        }
    };
    match class_guess {
        "BasePart" => &[
            ("Position", "Property"), ("Size", "Property"), ("CFrame", "Property"),
            ("Color", "Property"), ("Material", "Property"), ("Anchored", "Property"),
            ("CanCollide", "Property"), ("Transparency", "Property"), ("Touched", "Event"),
            ("Destroy", "Method"), ("Clone", "Method"), ("ApplyImpulse", "Method"),
        ],
        "Humanoid" => &[
            ("Health", "Property"), ("MaxHealth", "Property"), ("WalkSpeed", "Property"),
            ("JumpPower", "Property"), ("MoveDirection", "Property"), ("Died", "Event"),
            ("TakeDamage", "Method"), ("MoveTo", "Method"), ("ChangeState", "Method"),
        ],
        "RemoteEvent" => &[
            ("OnServerEvent", "Event"), ("OnClientEvent", "Event"),
            ("FireServer", "Method"), ("FireClient", "Method"), ("FireAllClients", "Method"),
        ],
        "TweenService" => &[("Create", "Method"), ("GetValue", "Method")],
        "Players" => &[
            ("LocalPlayer", "Property"), ("PlayerAdded", "Event"), ("PlayerRemoving", "Event"),
            ("GetPlayers", "Method"), ("GetPlayerFromCharacter", "Method"),
        ],
        "RunService" => &[
            ("Heartbeat", "Event"), ("RenderStepped", "Event"), ("Stepped", "Event"),
            ("IsClient", "Method"), ("IsServer", "Method"),
        ],
        "UserInputService" => &[
            ("InputBegan", "Event"), ("InputEnded", "Event"), ("TouchEnabled", "Property"),
            ("IsKeyDown", "Method"),
        ],
        "Motor6D" => &[
            ("Part0", "Property"), ("Part1", "Property"), ("C0", "Property"),
            ("C1", "Property"), ("Transform", "Property"),
        ],
        "Camera" => &[("CFrame", "Property"), ("FieldOfView", "Property"), ("CameraSubject", "Property")],
        "Sound" => &[("SoundId", "Property"), ("Volume", "Property"), ("Play", "Method"), ("Stop", "Method")],
        "TextLabel" | "Frame" => &[
            ("Position", "Property"), ("Size", "Property"), ("BackgroundColor3", "Property"),
            ("Text", "Property"), ("Visible", "Property"), ("TweenPosition", "Method"),
        ],
        "Model" => &[
            ("PrimaryPart", "Property"), ("PivotTo", "Method"), ("GetPivot", "Method"),
            ("FindFirstChild", "Method"), ("Clone", "Method"),
        ],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens() {
        let t = tokenize("local x = 5 -- hi\nprint(\"a\")");
        let kinds: Vec<Tok> = t
            .iter()
            .filter(|(_, k)| *k != Tok::Plain || false)
            .map(|(_, k)| *k)
            .collect();
        assert_eq!(kinds[0], Tok::Keyword);
        assert!(
            kinds.contains(&Tok::Num)
                && kinds.contains(&Tok::Comment)
                && kinds.contains(&Tok::Str)
                && kinds.contains(&Tok::Global)
        );
    }

    #[test]
    fn syntax_error_line() {
        let e = runtime::check_syntax("local a = 1\nlocal = 2\n", "Workspace.S").unwrap();
        assert_eq!(e.0, 2, "{e:?}");
    }
}
