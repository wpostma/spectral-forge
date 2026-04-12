use nih_plug_egui::egui::{Rect, Painter, Pos2};
use crate::editor::theme as th;

/// Paint log-scaled magnitude spectrum bars behind the curve.
/// `magnitudes`: linear magnitude per bin.
pub fn paint_spectrum(painter: &Painter, rect: Rect, magnitudes: &[f32]) {
    if magnitudes.is_empty() { return; }
    let n = magnitudes.len();
    let bar_width = rect.width() / n as f32;
    let peak = magnitudes.iter().cloned().fold(1e-10f32, f32::max);

    for k in 0..n {
        let x_norm = k as f32 / n as f32;
        let x = rect.left() + x_norm * rect.width();
        let mag_norm = (magnitudes[k] / peak).clamp(0.0, 1.0);
        let height_norm = if mag_norm > 1e-6 {
            (1.0 + 20.0 * mag_norm.log10() / 60.0).clamp(0.0, 1.0)
        } else { 0.0 };
        let bar_height = height_norm * rect.height();
        let top = rect.bottom() - bar_height;
        let color = th::magnitude_color(height_norm);
        painter.rect_filled(
            nih_plug_egui::egui::Rect::from_min_max(
                Pos2::new(x, top),
                Pos2::new(x + bar_width.max(1.0), rect.bottom()),
            ),
            0.0,
            color,
        );
    }
}
