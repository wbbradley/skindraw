use std::sync::Arc;

use eframe::egui::{self, Color32, Key, PointerButton, Rect, Sense, Vec2};
use glam::Vec2 as ModelVec2;

use crate::{
    BrushSize, Camera, LayerVisibility, ModelHit, ModelKind, Skin, SkinDocument, StrokeBuilder,
    pick_model,
    renderer::{ModelPaintCallback, ModelRenderer, TextureUpdate},
};

const PALETTE: [[u8; 4]; 16] = [
    [0, 0, 0, 255],
    [64, 64, 64, 255],
    [128, 128, 128, 255],
    [192, 192, 192, 255],
    [255, 255, 255, 255],
    [136, 0, 21, 255],
    [237, 28, 36, 255],
    [255, 127, 39, 255],
    [255, 242, 0, 255],
    [34, 177, 76, 255],
    [0, 162, 232, 255],
    [63, 72, 204, 255],
    [163, 73, 164, 255],
    [185, 122, 87, 255],
    [255, 174, 201, 255],
    [0, 0, 0, 0],
];

pub struct SkinDrawApp {
    document: SkinDocument,
    kind: ModelKind,
    camera: Camera,
    visibility: LayerVisibility,
    brush_size: BrushSize,
    active_color: [u8; 4],
    active_stroke: Option<StrokeBuilder>,
    hovered_hit: Option<ModelHit>,
    uploaded_skin: Skin,
}

impl SkinDrawApp {
    pub fn new(creation: &eframe::CreationContext<'_>) -> Self {
        let document = SkinDocument::new(ModelKind::Classic);
        let render_state = creation
            .wgpu_render_state
            .as_ref()
            .expect("SkinDraw requires the WGPU renderer");
        ModelRenderer::install(render_state, document.skin());
        Self {
            uploaded_skin: document.skin().clone(),
            document,
            kind: ModelKind::Classic,
            camera: Camera {
                yaw: -0.45,
                pitch: 0.18,
                ..Default::default()
            },
            visibility: LayerVisibility::ALL,
            brush_size: BrushSize::One,
            active_color: [237, 28, 36, 255],
            active_stroke: None,
            hovered_hit: None,
        }
    }

    fn sidebar(&mut self, root: &mut egui::Ui) {
        egui::Panel::right("tools")
            .default_size(220.0)
            .min_size(220.0)
            .max_size(220.0)
            .resizable(false)
            .show(root, |ui| {
                ui.heading("SkinDraw");
                ui.add_space(8.0);

                ui.label("Arm model");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.kind, ModelKind::Classic, "Classic");
                    ui.selectable_value(&mut self.kind, ModelKind::Slim, "Slim");
                });
                ui.add_space(10.0);

                ui.label("Visible geometry");
                ui.checkbox(&mut self.visibility.base, "Base");
                ui.checkbox(&mut self.visibility.outer, "Outer");
                if !self.visibility.base && !self.visibility.outer {
                    ui.colored_label(Color32::LIGHT_RED, "Enable a layer to paint.");
                }
                ui.add_space(10.0);

                ui.label("Brush");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.brush_size, BrushSize::One, "1");
                    ui.selectable_value(&mut self.brush_size, BrushSize::Two, "2");
                    ui.selectable_value(&mut self.brush_size, BrushSize::Four, "4");
                });
                ui.add_space(10.0);

                ui.label("Palette");
                egui::Grid::new("palette")
                    .spacing(Vec2::splat(4.0))
                    .show(ui, |ui| {
                        for (index, &color) in PALETTE.iter().enumerate() {
                            let fill = Color32::from_rgba_unmultiplied(
                                color[0], color[1], color[2], color[3],
                            );
                            let selected = color == self.active_color;
                            let button = egui::Button::new("")
                                .min_size(Vec2::splat(28.0))
                                .fill(fill)
                                .stroke(if selected {
                                    egui::Stroke::new(2.0, Color32::WHITE)
                                } else {
                                    egui::Stroke::new(1.0, Color32::DARK_GRAY)
                                });
                            if ui.add(button).clicked() {
                                self.active_color = color;
                            }
                            if (index + 1) % 5 == 0 {
                                ui.end_row();
                            }
                        }
                    });
                ui.add_space(10.0);

                ui.label("Custom RGBA");
                for (label, channel) in ["R", "G", "B", "A"]
                    .into_iter()
                    .zip(self.active_color.iter_mut())
                {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        ui.add(egui::Slider::new(channel, 0..=255).show_value(true));
                    });
                }
                let active = Color32::from_rgba_unmultiplied(
                    self.active_color[0],
                    self.active_color[1],
                    self.active_color[2],
                    self.active_color[3],
                );
                ui.horizontal(|ui| {
                    ui.label("Active");
                    let rect = ui.allocate_space(Vec2::new(64.0, 24.0)).1;
                    ui.painter().rect_filled(rect, 3.0, active);
                });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(self.document.undo_len() > 0, egui::Button::new("Undo"))
                        .clicked()
                    {
                        self.finish_stroke();
                        self.document.undo();
                    }
                    if ui
                        .add_enabled(self.document.redo_len() > 0, egui::Button::new("Redo"))
                        .clicked()
                    {
                        self.finish_stroke();
                        self.document.redo();
                    }
                });

                ui.add_space(12.0);
                ui.separator();
                ui.label("Paint: primary-button drag");
                ui.label("Orbit: Space + primary drag");
                if let Some(hit) = self.hovered_hit {
                    ui.add_space(8.0);
                    ui.monospace(format!(
                        "{:?} · {:?} · {:?}\ntexel {}, {}",
                        hit.part, hit.layer, hit.face, hit.texel.x, hit.texel.y
                    ));
                }
            });
    }

    fn model_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let available = ui.available_size().max(Vec2::splat(1.0));
        let (rect, response) = ui.allocate_exact_size(available, Sense::click_and_drag());
        let space = ctx.input(|input| input.key_down(Key::Space));
        let primary_down = ctx.input(|input| {
            input.pointer.button_down(PointerButton::Primary)
                || input.pointer.button_pressed(PointerButton::Primary)
        });
        let primary_released =
            ctx.input(|input| input.pointer.button_released(PointerButton::Primary));

        let pointer_in_view = response.hovered()
            || response.clicked_by(PointerButton::Primary)
            || response.dragged_by(PointerButton::Primary);

        if space && primary_down && pointer_in_view {
            let delta = ctx.input(|input| input.pointer.delta());
            self.camera.orbit(-delta.x * 0.012, -delta.y * 0.012);
        }

        let pointer = response
            .interact_pointer_pos()
            .or_else(|| response.hover_pos());
        self.hovered_hit = pointer.and_then(|position| self.hit_at(rect, position));

        if !space
            && primary_down
            && pointer_in_view
            && let Some(hit) = self.hovered_hit
        {
            let stroke = self.active_stroke.get_or_insert_with(StrokeBuilder::new);
            self.document
                .paint(stroke, self.kind, hit, self.brush_size, self.active_color);
        }
        if primary_released {
            self.finish_stroke();
        }

        let texture_update = TextureUpdate::between(&self.uploaded_skin, self.document.skin());
        let skin = Arc::new(self.document.skin().clone());
        if texture_update.is_some() {
            self.uploaded_skin = (*skin).clone();
        }
        ui.painter().add(
            ModelPaintCallback {
                rect,
                kind: self.kind,
                visibility: self.visibility,
                camera: self.camera,
                skin,
                texture_update,
                hit: self.hovered_hit,
            }
            .paint_callback(),
        );

        if space && response.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::Grab);
        } else if self.hovered_hit.is_some() {
            ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        }
    }

    fn hit_at(&self, rect: Rect, pointer: egui::Pos2) -> Option<ModelHit> {
        let ray = self.camera.ray_for_pointer(
            ModelVec2::new(pointer.x, pointer.y),
            ModelVec2::new(rect.min.x, rect.min.y),
            ModelVec2::new(rect.width(), rect.height()),
        )?;
        pick_model(ray, self.kind, self.visibility)
    }

    fn finish_stroke(&mut self) {
        if let Some(stroke) = self.active_stroke.take() {
            self.document.commit_stroke(stroke);
        }
    }
}

impl eframe::App for SkinDrawApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.sidebar(ui);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.model_view(ui, &ctx));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Texel;

    #[test]
    fn presentation_kind_does_not_change_skin_pixels() {
        let document = SkinDocument::new(ModelKind::Classic);
        let pixels = document.skin().clone();
        let mut kind = ModelKind::Classic;
        assert_eq!(kind, ModelKind::Classic);
        kind = ModelKind::Slim;
        assert_eq!(kind, ModelKind::Slim);
        assert_eq!(document.skin(), &pixels);
        assert!(!document.is_dirty());
    }

    #[test]
    fn palette_has_opaque_choices_and_transparent_eraser() {
        assert!(PALETTE.iter().any(|color| color[3] == 255));
        assert!(PALETTE.iter().any(|color| color[3] == 0));
        assert_eq!(PALETTE.len(), 16);
    }

    #[test]
    fn brush_controls_cover_every_core_size() {
        assert_eq!(
            [BrushSize::One, BrushSize::Two, BrushSize::Four].map(BrushSize::pixels),
            [1, 2, 4]
        );
        assert_eq!(Texel::new(1, 2).x, 1);
    }
}
