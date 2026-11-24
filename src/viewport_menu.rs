//! Studio's viewport menu. Commands share the ribbon's availability and dispatch.
use crate::app::Editor;
use egui::{PointerButton, Response, Ui};

impl Editor {
    pub(crate) fn viewport_context_menu(&mut self, ui: &mut Ui, response: &Response) {
        if self.game.is_playing() { return; }
        let ctx = ui.ctx().clone();
        // Long-press on a Part in the 3D Viewport opens a quick Clone / Edit popover
        let lp_open_id = egui::Id::new("vp_part_lp_open");
        let lp_pos_id = egui::Id::new("vp_part_lp_pos");
        let lp_down_id = egui::Id::new("vp_part_lp_down");
        let now = ctx.input(|i| i.time);
        let ptr_down = ctx.input(|i| i.pointer.primary_down());
        let ptr_pos = ctx.input(|i| i.pointer.interact_pos());
        if ptr_down && response.hovered() && !response.dragged() {
            if let Some(p) = ptr_pos {
                let state = ctx.data(|d| d.get_temp::<(f64, egui::Pos2, bool)>(lp_down_id));
                match state {
                    None => {
                        ctx.data_mut(|d| d.insert_temp(lp_down_id, (now, p, false)));
                    }
                    Some((t0, p0, fired)) => {
                        if p.distance(p0) > 10.0 {
                            ctx.data_mut(|d| d.insert_temp(lp_down_id, (now, p, true)));
                        } else if !fired && now - t0 >= 0.48 {
                            if let Some(hit) = self.pick(response.rect, p0, false) {
                                if !self.game.doc.selection.contains(&hit) {
                                    self.select(vec![hit]);
                                }
                                ctx.data_mut(|d| {
                                    d.insert_temp(lp_open_id, true);
                                    d.insert_temp(lp_pos_id, p0);
                                    d.insert_temp(lp_down_id, (t0, p0, true));
                                });
                            } else {
                                ctx.data_mut(|d| d.insert_temp(lp_down_id, (t0, p0, true)));
                            }
                        }
                    }
                }
            }
        } else if !ptr_down {
            ctx.data_mut(|d| d.remove_temp::<(f64, egui::Pos2, bool)>(lp_down_id));
        }

        let show_selection_pill = !self.game.doc.selection.is_empty()
            && (cfg!(any(target_os = "ios", target_os = "android"))
                || rustudio_core::mobile::is_drawer_mode()
                || ctx.data(|d| d.get_temp::<bool>(lp_open_id).unwrap_or(false)));
        if show_selection_pill {
            let default_pos = egui::pos2(response.rect.center().x, response.rect.top() + 86.0);
            let pop_pos = ctx
                .data(|d| d.get_temp::<egui::Pos2>(lp_pos_id))
                .unwrap_or(default_pos);
            let mut close = false;
            let mut lp_act: Option<&'static str> = None;
            let shown = egui::Area::new(egui::Id::new("vp_part_lp_popover"))
                .order(egui::Order::Foreground)
                .fixed_pos(pop_pos + egui::vec2(-160.0, -54.0))
                .show(&ctx, |ui| {
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgba_unmultiplied(18, 21, 28, 245))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 42)))
                        .corner_radius(egui::CornerRadius::same(14))
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 5.0;
                                for (label, cmd, col) in [
                                    ("Clone", "duplicate", egui::Color32::from_rgb(33, 112, 228)),
                                    ("Delete", "delete", egui::Color32::from_rgb(185, 42, 42)),
                                    ("Focus", "focus", egui::Color32::from_rgb(38, 44, 58)),
                                    ("Anchor", "anchor", egui::Color32::from_rgb(38, 44, 58)),
                                    ("Lock", "lock", egui::Color32::from_rgb(38, 44, 58)),
                                ] {
                                    if ui
                                        .add_sized(
                                            [60.0, 30.0],
                                            egui::Button::new(
                                                egui::RichText::new(label)
                                                    .size(12.0)
                                                    .strong()
                                                    .color(egui::Color32::WHITE),
                                            )
                                            .fill(col)
                                            .corner_radius(8.0),
                                        )
                                        .clicked()
                                    {
                                        lp_act = Some(cmd);
                                        close = true;
                                    }
                                }
                            });
                        });
                });
            let clicked_out = ctx.input(|i| {
                i.pointer.any_pressed()
                    && i.pointer.interact_pos().is_some_and(|p| !shown.response.rect.expand(8.0).contains(p))
            });
            if close || clicked_out {
                ctx.data_mut(|d| d.insert_temp(lp_open_id, false));
            }
            if let Some(cmd) = lp_act {
                if cmd == "focus" {
                    self.focus_selection();
                } else {
                    self.ui_action(&ctx, cmd);
                }
            }
        }

        if response.clicked_by(PointerButton::Secondary) {
            if let Some(p) = response.interact_pointer_pos() {
                let hit = self.pick(response.rect, p, ui.input(|i| i.modifiers.alt));
                match hit {
                    Some(r) if !self.game.doc.selection.contains(&r) => self.select(vec![r]),
                    None => self.select(Vec::new()),
                    _ => {}
                }
            }
        }
        let mut action = None;
        response.context_menu(|ui| {
            ui.set_min_width(254.0);
            ui.spacing_mut().item_spacing.y = 0.0;
            let mut item = |ui: &mut Ui, label: &str, shortcut: &str, command: &'static str| {
                let enabled = match command {
                    "parents" => self.game.doc.selection.iter().any(|r| self.game.doc.dom.get_by_ref(*r).is_some_and(|i| i.parent().is_some())),
                    "children" => self.game.doc.selection.iter().any(|r| self.game.doc.dom.get_by_ref(*r).is_some_and(|i| !i.children().is_empty())),
                    "focus" => self.selection_bounds().is_some(),
                    _ => self.action_state(command).0,
                };
                // Checkable items (Align Dragged Objects) show their tick.
                let checked = if command == "align_dragged" { self.action_state(command).1 } else { None };
                if crate::studio_actions::menu_item_at(ui, label, shortcut, enabled, checked, 182.0, 254.0).clicked() {
                    action = Some(command);
                    ui.close();
                }
            };
            item(ui,"Cut","Ctrl+X","cut");
            item(ui,"Copy","Ctrl+C","copy");
            ui.add_enabled_ui(false, |ui| { ui.menu_button("Copy As", |_| {}); });
            item(ui,"Paste","Ctrl+V","paste");
            item(ui,"Paste Into","Ctrl+Shift+V","paste_into");
            item(ui,"Paste at Original Location","","paste_original");
            item(ui,"Paste into at Original Location","","paste_into_original");
            item(ui,"Duplicate","Ctrl+D","duplicate");
            item(ui,"Delete","Del","delete");
            ui.separator();
            item(ui,"Group as Model","Ctrl+G","group");
            item(ui,"Group as Folder","Ctrl+Alt+G","group_folder");
            item(ui,"Ungroup","Ctrl+U","ungroup");
            item(ui,"Union","Ctrl+Shift+G","union");
            item(ui,"Intersect","Ctrl+Shift+I","intersect");
            item(ui,"Negate","Ctrl+Shift+N","negate");
            item(ui,"Separate","Ctrl+Shift+U","separate");
            ui.add_enabled_ui(false, |ui| { ui.menu_button("Generate", |_| {}); });
            item(ui,"Select Parent(s)","","parents");
            item(ui,"Select Children","","children");
            item(ui,"Zoom to","F","focus");
            item(ui,"Show Orientation Indicator","","orientation_indicator");
            item(ui,"Align Dragged Objects","","align_dragged");
            ui.separator();
            item(ui,"Select Connections","Alt+C","connections");
            item(ui,"Select Connection 0","Alt+1","connection_0");
            item(ui,"Select Connection 1","Alt+2","connection_1");
            item(ui,"Swap Attachments","","swap_attachments");
            ui.separator();
            item(ui,"Insert Part","","part");
            item(ui,"Insert Object…","Ctrl+I","insert_object");
            ui.separator();
            for label in ["Convert to Package", "Publish to Package", "Undo Changes to Package", "Get Latest Package", "View Script Changes…", "Package Details", "Update All", "Compare Package Versions"] {
                item(ui,label,"","unsupported_package");
            }
            ui.separator();
            item(ui,"Help","","documentation");
        });
        if let Some(action) = action {
            match action {
                "parents" | "children" => {
                    let mut refs = Vec::new();
                    for r in &self.game.doc.selection {
                        if let Some(i) = self.game.doc.dom.get_by_ref(*r) {
                            if action == "parents" {
                                if i.parent().is_some() && !refs.contains(&i.parent()) { refs.push(i.parent()); }
                            } else {
                                for r in i.children() { if !refs.contains(r) { refs.push(*r); } }
                            }
                        }
                    }
                    self.select(refs);
                }
                "focus" => self.focus_selection(),
                _ => self.ui_action(ui.ctx(), action),
            }
        }
    }
}
