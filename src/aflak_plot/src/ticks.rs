use imgui::{ImColor, ImString, Ui, WindowDrawList};

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
    labels: Vec<(ImString, [f32; 2])>,
    axis_label: (ImString, [f32; 2]),
}

pub struct YTicks {
    labels: Vec<(ImString, [f32; 2])>,
    axis_label: (ImString, [f32; 2]),
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

    pub fn draw(self, draw_list: &WindowDrawList, p: [f32; 2], size: [f32; 2]) {
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
            let label = axis.map(|axis| axis.name()).unwrap_or_else(String::new);
            let label = ImString::new(label);
            let text_size = ui.calc_text_size(&label, false, -1.0);
            (label, text_size)
        };
        XTicks { labels, axis_label }
    }

    pub fn height(&self) -> f32 {
        let label_height = self.axis_label.1[1];
        let tick_height: f32 = self
            .labels
            .iter()
            .fold(0.0, |acc, (_, size)| acc.max(size[1]));
        tick_height + label_height
    }

    pub fn draw(self, draw_list: &WindowDrawList, p: [f32; 2], size: [f32; 2]) {
        let x_step = size[0] / (self.labels.len() - 1) as f32;
        let mut x_pos = p[0];
        let y_pos = p[1] + size[1];
        let mut label_height = 0.0f32;
        for (label, text_size) in self.labels {
            draw_list
                .add_line([x_pos, y_pos], [x_pos, y_pos - size[1]], GRID_COLOR)
                .build();
            draw_list
                .add_line([x_pos, y_pos], [x_pos, y_pos - TICK_SIZE], COLOR)
                .build();
            draw_list.add_text([x_pos - text_size[0] / 2.0, y_pos], COLOR, label.to_str());
            x_pos += x_step;
            label_height = label_height.max(text_size[1]);
        }

        let (label, text_size) = self.axis_label;
        let middle_x = p[0] + size[0] / 2.0;
        draw_list.add_text(
            [middle_x - text_size[0] / 2.0, y_pos + label_height],
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
            let label = axis.map(|axis| axis.name()).unwrap_or_else(String::new);
            let label = ImString::new(label);
            let text_size = ui.calc_text_size(&label, false, -1.0);
            (label, text_size)
        };
        YTicks { labels, axis_label }
    }

    pub fn width(&self) -> f32 {
        let label_width = self.axis_label.1[1];
        let tick_width: f32 = self
            .labels
            .iter()
            .fold(0.0, |acc, (_, size)| acc.max(size[0]));
        label_width + tick_width + YTicks::AXIS_NAME_RIGHT_PADDING
    }

    pub fn draw(self, draw_list: &WindowDrawList, p: [f32; 2], size: [f32; 2]) {
        let y_step = size[1] / (self.labels.len() - 1) as f32;
        let mut y_pos = p[1] + size[1];
        let x_pos = p[0];
        let mut label_width = 0.0f32;
        for (label, text_size) in self.labels {
            draw_list
                .add_line([x_pos, y_pos], [x_pos + size[0], y_pos], GRID_COLOR)
                .build();
            draw_list
                .add_line([x_pos, y_pos], [x_pos + TICK_SIZE, y_pos], COLOR)
                .build();
            draw_list.add_text(
                [
                    x_pos - text_size[0] - LABEL_HORIZONTAL_PADDING,
                    y_pos - text_size[1] / 2.0,
                ],
                COLOR,
                label.to_str(),
            );
            y_pos -= y_step;
            label_width = label_width.max(text_size[0])
        }

        let (label, text_size) = self.axis_label;
        let middle_y = p[1] + size[1] / 2.0;
        unsafe {
            add_text_vertical(
                [
                    p[0] - label_width - text_size[1] - YTicks::AXIS_NAME_RIGHT_PADDING,
                    middle_y + text_size[0] / 2.0,
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
unsafe fn add_text_vertical<C, T>(pos: [f32; 2], col: C, text: T)
where
    C: Into<ImColor>,
    T: AsRef<str>,
{
    use imgui::sys;

    let col = col.into();
    let text = text.as_ref();

    let mut y = pos[1];

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
            sys::ImVec2 {
                x: pos[0] + glyph.Y0,
                y: y - glyph.X0,
            },
            sys::ImVec2 {
                x: pos[0] + glyph.Y0,
                y: y - glyph.X1,
            },
            sys::ImVec2 {
                x: pos[0] + glyph.Y1,
                y: y - glyph.X1,
            },
            sys::ImVec2 {
                x: pos[0] + glyph.Y1,
                y: y - glyph.X0,
            },
            sys::ImVec2 {
                x: glyph.U0,
                y: glyph.V0,
            },
            sys::ImVec2 {
                x: glyph.U1,
                y: glyph.V0,
            },
            sys::ImVec2 {
                x: glyph.U1,
                y: glyph.V1,
            },
            sys::ImVec2 {
                x: glyph.U0,
                y: glyph.V1,
            },
            col.into(),
        );

        y -= glyph.AdvanceX;
    }
}
