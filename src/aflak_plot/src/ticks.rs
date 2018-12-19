use imgui::{ImColor, ImString, ImVec2, Ui, WindowDrawList};

use super::AxisTransform;

const TICK_COUNT: usize = 10;

const COLOR: u32 = 0xFFFF_FFFF;
const GRID_COLOR: [f32; 4] = [0.78, 0.78, 0.78, 0.16];
const TICK_SIZE: f32 = 3.0;
const LABEL_HORIZONTAL_PADDING: f32 = 2.0;

pub struct XYTicks {
    x: XTicks,
    y: YTicks,
}

pub struct XTicks {
    labels: Vec<(ImString, ImVec2)>,
    axis_label: (ImString, ImVec2),
}

pub struct YTicks {
    labels: Vec<(ImString, ImVec2)>,
    axis_label: (ImString, ImVec2),
}

impl XYTicks {
    pub fn prepare<F1, F2>(
        ui: &Ui,
        xlims: (f32, f32),
        ylims: (f32, f32),
        xaxis: Option<&AxisTransform<F1>>,
        yaxis: Option<&AxisTransform<F2>>,
    ) -> Self
    where
        F1: Fn(f32) -> f32,
        F2: Fn(f32) -> f32,
    {
        Self {
            x: XTicks::prepare(ui, xlims, xaxis),
            y: YTicks::prepare(ui, ylims, yaxis),
        }
    }

    pub fn x_labels_height(&self) -> f32 {
        self.x.height()
    }

    pub fn y_labels_width(&self) -> f32 {
        self.y.width()
    }

    pub fn draw<P, S>(self, draw_list: &WindowDrawList, p: P, size: S)
    where
        P: Into<ImVec2>,
        S: Into<ImVec2>,
    {
        let p = p.into();
        let size = size.into();

        self.x.draw(draw_list, p, size);
        self.y.draw(draw_list, p, size);
    }
}

impl XTicks {
    pub fn prepare<F>(ui: &Ui, xlims: (f32, f32), axis: Option<&AxisTransform<F>>) -> Self
    where
        F: Fn(f32) -> f32,
    {
        let labels = (0..=TICK_COUNT)
            .map(|i| {
                let point = xlims.0 + i as f32 * (xlims.1 - xlims.0) / TICK_COUNT as f32;
                let label = if let Some(axis) = axis {
                    let transformed = axis.pix2world(point);
                    ImString::new(format!("{:.2}", transformed))
                } else {
                    ImString::new(format!("{:.0}", point))
                };
                let text_size = ui.calc_text_size(&label, false, -1.0);
                (label, text_size)
            })
            .collect();
        let axis_label = {
            let unit = axis.map(|axis| axis.unit()).unwrap_or("");
            let label = ImString::new(unit);
            let text_size = ui.calc_text_size(&label, false, -1.0);
            (label, text_size)
        };
        XTicks { labels, axis_label }
    }

    pub fn height(&self) -> f32 {
        let label_height = self.axis_label.1.y;
        let tick_height: f32 = self
            .labels
            .iter()
            .fold(0.0, |acc, (_, size)| acc.max(size.y));
        tick_height + label_height
    }

    pub fn draw<P, S>(self, draw_list: &WindowDrawList, p: P, size: S)
    where
        P: Into<ImVec2>,
        S: Into<ImVec2>,
    {
        let p = p.into();
        let size = size.into();

        let x_step = size.x / (self.labels.len() - 1) as f32;
        let mut x_pos = p.x;
        let y_pos = p.y + size.y;
        let mut label_height = 0.0f32;
        for (label, text_size) in self.labels {
            draw_list
                .add_line([x_pos, y_pos], [x_pos, y_pos - size.y], GRID_COLOR)
                .build();
            draw_list
                .add_line([x_pos, y_pos], [x_pos, y_pos - TICK_SIZE], COLOR)
                .build();
            draw_list.add_text([x_pos - text_size.x / 2.0, y_pos], COLOR, label.to_str());
            x_pos += x_step;
            label_height = label_height.max(text_size.y);
        }

        let (label, text_size) = self.axis_label;
        let middle_x = p.x + size.x / 2.0;
        draw_list.add_text(
            [middle_x - text_size.x / 2.0, y_pos + label_height],
            COLOR,
            label,
        );
    }
}

impl YTicks {
    const AXIS_NAME_RIGHT_PADDING: f32 = 2.0;

    pub fn prepare<F>(ui: &Ui, ylims: (f32, f32), axis: Option<&AxisTransform<F>>) -> Self
    where
        F: Fn(f32) -> f32,
    {
        let labels = (0..=TICK_COUNT)
            .map(|i| {
                let point = ylims.0 + i as f32 * (ylims.1 - ylims.0) / TICK_COUNT as f32;
                let label = if let Some(axis) = axis {
                    let transformed = axis.pix2world(point);
                    ImString::new(format!("{:.2}", transformed))
                } else {
                    ImString::new(format!("{:.0}", point))
                };
                let text_size = ui.calc_text_size(&label, false, -1.0);
                (label, text_size)
            })
            .collect();
        let axis_label = {
            let unit = axis.map(|axis| axis.unit()).unwrap_or("");
            let label = ImString::new(unit);
            let text_size = ui.calc_text_size(&label, false, -1.0);
            (label, text_size)
        };
        YTicks { labels, axis_label }
    }

    pub fn width(&self) -> f32 {
        let label_width = self.axis_label.1.y;
        let tick_width: f32 = self
            .labels
            .iter()
            .fold(0.0, |acc, (_, size)| acc.max(size.x));
        label_width + tick_width + YTicks::AXIS_NAME_RIGHT_PADDING
    }

    pub fn draw<P, S>(self, draw_list: &WindowDrawList, p: P, size: S)
    where
        P: Into<ImVec2>,
        S: Into<ImVec2>,
    {
        let p = p.into();
        let size = size.into();

        let y_step = size.y / (self.labels.len() - 1) as f32;
        let mut y_pos = p.y + size.y;
        let x_pos = p.x;
        let mut label_width = 0.0f32;
        for (label, text_size) in self.labels {
            draw_list
                .add_line([x_pos, y_pos], [x_pos + size.x, y_pos], GRID_COLOR)
                .build();
            draw_list
                .add_line([x_pos, y_pos], [x_pos + TICK_SIZE, y_pos], COLOR)
                .build();
            draw_list.add_text(
                [
                    x_pos - text_size.x - LABEL_HORIZONTAL_PADDING,
                    y_pos - text_size.y / 2.0,
                ],
                COLOR,
                label.to_str(),
            );
            y_pos -= y_step;
            label_width = label_width.max(text_size.x)
        }

        let (label, text_size) = self.axis_label;
        let middle_y = p.y + size.y / 2.0;
        unsafe {
            add_text_vertical(
                [
                    p.x - label_width - text_size.y - YTicks::AXIS_NAME_RIGHT_PADDING,
                    middle_y + text_size.x / 2.0,
                ],
                COLOR,
                label,
            );
        }
    }
}

/// Draw vertical text using direct draw calls.
///
/// Inspired from: https://github.com/ocornut/imgui/issues/705#issuecomment-247959437
unsafe fn add_text_vertical<P, C, T>(pos: P, col: C, text: T)
where
    P: Into<ImVec2>,
    C: Into<ImColor>,
    T: AsRef<str>,
{
    use imgui::sys;

    let pos = pos.into();
    let col = col.into();
    let text = text.as_ref();

    let mut y = pos.y;

    let font = sys::igGetFont();
    for c in text.chars() {
        let glyph = sys::ImFont_FindGlyph(font, c as sys::ImWchar);
        if glyph.is_null() {
            continue;
        }
        let glyph = &*glyph;
        let draw_list = sys::igGetWindowDrawList();
        sys::ImDrawList_PrimReserve(draw_list, 6, 4);
        sys::ImDrawList_PrimQuadUV(
            draw_list,
            ImVec2::new(pos.x + glyph.y0, y - glyph.x0),
            ImVec2::new(pos.x + glyph.y0, y - glyph.x1),
            ImVec2::new(pos.x + glyph.y1, y - glyph.x1),
            ImVec2::new(pos.x + glyph.y1, y - glyph.x0),
            ImVec2::new(glyph.u0, glyph.v0),
            ImVec2::new(glyph.u1, glyph.v0),
            ImVec2::new(glyph.u1, glyph.v1),
            ImVec2::new(glyph.u0, glyph.v1),
            col.into(),
        );

        y -= glyph.advance_x;
    }
}
