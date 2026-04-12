use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui};
use std::sync::Arc;
use crate::params::{SpectralForgeParams, NUM_CURVE_SETS};
use crate::editor::theme as th;

const CURVE_LABELS: [&str; NUM_CURVE_SETS] =
    ["THRESHOLD", "RATIO", "ATTACK", "RELEASE", "KNEE", "MAKEUP", "MIX"];

pub fn create_editor(
    params: Arc<SpectralForgeParams>,
) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        (),
        |ctx, _| {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = th::BG;
            ctx.set_visuals(visuals);
        },
        move |ctx, _setter, _state| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(th::BG))
                .show(ctx, |ui| {
                    // Parameter selector row
                    ui.horizontal(|ui| {
                        ui.add_space(4.0);
                        let active = *params.active_curve.lock();
                        for (i, label) in CURVE_LABELS.iter().enumerate() {
                            let is_active = active == i as u8;
                            let (fill, text_color) = if is_active {
                                (th::BTN_ACTIVE, th::BTN_TEXT_ON)
                            } else {
                                (th::BTN_INACTIVE, th::BTN_TEXT_OFF)
                            };
                            let btn = egui::Button::new(
                                egui::RichText::new(*label)
                                    .color(text_color)
                                    .size(11.0)
                            ).fill(fill)
                             .stroke(egui::Stroke::new(th::STROKE_BORDER, th::BORDER));
                            if ui.add(btn).clicked() {
                                *params.active_curve.lock() = i as u8;
                            }
                        }
                    });

                    ui.add_space(2.0);
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().line_segment(
                        [rect.left_top(), rect.right_top()],
                        egui::Stroke::new(th::STROKE_BORDER, th::BORDER),
                    );

                    // Curve area placeholder — filled in Task 12
                    let curve_rect = ui.available_rect_before_wrap();
                    ui.painter().rect_filled(curve_rect, 0.0, th::BG);
                    ui.painter().text(
                        curve_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "curve editor — coming soon",
                        egui::FontId::monospace(12.0),
                        th::LABEL_DIM,
                    );
                    ui.allocate_rect(curve_rect, egui::Sense::hover());
                });
        },
    )
}
