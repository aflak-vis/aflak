use imgui::{DrawListMut, ImColor32, ImString, Ui};

use super::AxisTransform;
use std::collections::VecDeque;

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
        t: (bool, bool, bool, bool),
    ) -> Self
    where
        F1: Fn(f32) -> f32,
        F2: Fn(f32) -> f32,
    {
        Self {
            x: XTicks::prepare(ui, xlims, xaxis, (t.0, t.1)),
            y: YTicks::prepare(ui, ylims, yaxis, (t.2, t.3)),
        }
    }

    pub fn x_labels_height(&self) -> f32 {
        self.x.height()
    }

    pub fn y_labels_width(&self) -> f32 {
        self.y.width()
    }

    pub fn draw(self, draw_list: &DrawListMut, p: [f32; 2], size: [f32; 2]) {
        self.x.draw(draw_list, p, size);
        self.y.draw(draw_list, p, size);
    }
}

fn transform_to_format_imstrings<F>(
    ui: &Ui,
    lims: (f32, f32),
    axis: Option<&AxisTransform<F>>,
    use_ms: bool,
    relative: bool,
) -> Vec<(ImString, [f32; 2])>
where
    F: Fn(f32) -> f32,
{
    if relative {
        let right = (0..=TICK_COUNT / 2 - 1)
            .rev()
            .map(|i| {
                let l_point = lims.0 + i as f32 * (lims.1 - lims.0) / TICK_COUNT as f32;
                let r_point =
                    lims.0 + (TICK_COUNT - i) as f32 * (lims.1 - lims.0) / TICK_COUNT as f32;
                if let Some(axis) = axis {
                    let center = (lims.0 + lims.1) * 0.5;
                    let l_transformed = axis.pix2world(l_point) - axis.pix2world(center);
                    let r_transformed = axis.pix2world(r_point) - axis.pix2world(center);
                    let label = if use_ms {
                        match axis.label() {
                            "RA---TAN" => {
                                let l_hours = (l_transformed / 15.0).trunc();
                                let l_minutes =
                                    ((l_transformed / 15.0).fract() * 60.0).trunc().abs();
                                let l_seconds =
                                    (((l_transformed / 15.0).fract() * 60.0).fract() * 60.0).abs();
                                let r_hours = (r_transformed / 15.0).trunc();
                                let r_minutes =
                                    ((r_transformed / 15.0).fract() * 60.0).trunc().abs();
                                let r_seconds =
                                    (((r_transformed / 15.0).fract() * 60.0).fract() * 60.0).abs();

                                let mut l_s = String::new();
                                if l_transformed < 0.0 {
                                    l_s += "-";
                                } else {
                                    l_s += "+";
                                }
                                if l_hours != 0.0 {
                                    l_s +=
                                        format!("{:2}h{:2}m{:4.2}s", l_hours, l_minutes, l_seconds)
                                            .as_str();
                                } else {
                                    if l_minutes != 0.0 {
                                        l_s +=
                                            format!("{:2}m{:4.2}s", l_minutes, l_seconds).as_str();
                                    } else {
                                        if l_seconds != 0.0 {
                                            l_s += format!("{:4.2}s", l_seconds).as_str();
                                        }
                                    }
                                }
                                let mut r_s = String::new();
                                if r_transformed < 0.0 {
                                    r_s += "-";
                                } else {
                                    r_s += "+";
                                }
                                if r_hours != 0.0 {
                                    r_s +=
                                        format!("{:2}h{:2}m{:4.2}s", r_hours, r_minutes, r_seconds)
                                            .as_str();
                                } else {
                                    if r_minutes != 0.0 {
                                        r_s +=
                                            format!("{:2}m{:4.2}s", r_minutes, r_seconds).as_str();
                                    } else {
                                        if r_seconds != 0.0 {
                                            r_s += format!("{:4.2}s", r_seconds).as_str();
                                        }
                                    }
                                }
                                [ImString::new(l_s), ImString::new(r_s)]
                            }
                            "DEC--TAN" => {
                                let l_degree = l_transformed.trunc();
                                let l_minutes = (l_transformed.fract() * 60.0).trunc().abs();
                                let l_seconds =
                                    ((l_transformed.fract() * 60.0).fract() * 60.0).abs();
                                let r_degree = r_transformed.trunc();
                                let r_minutes = (r_transformed.fract() * 60.0).trunc().abs();
                                let r_seconds =
                                    ((r_transformed.fract() * 60.0).fract() * 60.0).abs();

                                let mut l_s = String::new();
                                if l_transformed < 0.0 {
                                    l_s += "-";
                                } else {
                                    l_s += "+";
                                }
                                if l_degree != 0.0 {
                                    l_s += format!(
                                        "{:2}°{:2}'{:4.2}''",
                                        l_degree, l_minutes, l_seconds
                                    )
                                    .as_str();
                                } else {
                                    if l_minutes != 0.0 {
                                        l_s +=
                                            format!("{:2}'{:4.2}''", l_minutes, l_seconds).as_str();
                                    } else {
                                        if l_seconds != 0.0 {
                                            l_s += format!("{:4.2}''", l_seconds).as_str();
                                        }
                                    }
                                }
                                let mut r_s = String::new();
                                if r_transformed < 0.0 {
                                    r_s += "-";
                                } else {
                                    r_s += "+";
                                }
                                if r_degree != 0.0 {
                                    r_s += format!(
                                        "{:2}°{:2}'{:4.2}''",
                                        r_degree, r_minutes, r_seconds
                                    )
                                    .as_str();
                                } else {
                                    if r_minutes != 0.0 {
                                        r_s +=
                                            format!("{:2}'{:4.2}''", r_minutes, r_seconds).as_str();
                                    } else {
                                        if r_seconds != 0.0 {
                                            r_s += format!("{:4.2}''", r_seconds).as_str();
                                        }
                                    }
                                }
                                [ImString::new(l_s), ImString::new(r_s)]
                            }
                            _ => [
                                ImString::new(format!("{:.2}", l_transformed)),
                                ImString::new(format!("{:.2}", r_transformed)),
                            ],
                        }
                    } else {
                        [
                            ImString::new(format!("{:.2}", l_transformed)),
                            ImString::new(format!("{:.2}", r_transformed)),
                        ]
                    };
                    let text_size = [ui.calc_text_size(&label[0]), ui.calc_text_size(&label[1])];
                    (label, text_size)
                } else {
                    let label = [
                        ImString::new(format!("{:.2}", l_point)),
                        ImString::new(format!("{:.2}", r_point)),
                    ];
                    let text_size = [ui.calc_text_size(&label[0]), ui.calc_text_size(&label[1])];
                    (label, text_size)
                }
            })
            .collect::<Vec<_>>();
        let mut t = VecDeque::new();
        let center_label = if let Some(axis) = axis {
            if use_ms {
                match axis.label() {
                    "RA---TAN" => ImString::new(format!("0.00s")),
                    "DEC--TAN" => ImString::new(format!("0.00''")),
                    _ => ImString::new(format!("0.00")),
                }
            } else {
                ImString::new(format!("0.00"))
            }
        } else {
            ImString::new(format!("0.00"))
        };
        let center_text_size = ui.calc_text_size(&center_label);
        t.push_back((center_label, center_text_size));
        let mut ret = Vec::new();
        for (label, text_size) in right {
            t.push_front((label[0].clone(), text_size[0]));
            t.push_back((label[1].clone(), text_size[1]));
        }
        for s in t {
            ret.push(s);
        }
        ret
    } else {
        (0..=TICK_COUNT)
            .map(|i| {
                let point = lims.0 + i as f32 * (lims.1 - lims.0) / TICK_COUNT as f32;
                if let Some(axis) = axis {
                    let transformed = axis.pix2world(point);
                    let label = if use_ms {
                        match axis.label() {
                            "RA---TAN" => {
                                let hours = (transformed / 15.0).trunc();
                                let minutes = ((transformed / 15.0).fract() * 60.0).trunc().abs();
                                let seconds =
                                    (((transformed / 15.0).fract() * 60.0).fract() * 60.0).abs();
                                ImString::new(format!("{:2}h{:2}m{:4.2}s", hours, minutes, seconds))
                            }
                            "DEC--TAN" => {
                                let degree = transformed.trunc();
                                let minutes = (transformed.fract() * 60.0).trunc().abs();
                                let seconds = ((transformed.fract() * 60.0).fract() * 60.0).abs();
                                ImString::new(format!(
                                    "{:2}°{:2}'{:4.2}''",
                                    degree, minutes, seconds
                                ))
                            }
                            _ => ImString::new(format!("{:.2}", transformed)),
                        }
                    } else {
                        ImString::new(format!("{:.2}", transformed))
                    };
                    let text_size = ui.calc_text_size(&label);
                    (label, text_size)
                } else {
                    let label = ImString::new(format!("{:.2}", point));
                    let text_size = ui.calc_text_size(&label);
                    (label, text_size)
                }
            })
            .collect()
    }
}

impl XTicks {
    pub fn prepare<F>(
        ui: &Ui,
        xlims: (f32, f32),
        axis: Option<&AxisTransform<F>>,
        (use_ms, relative): (bool, bool),
    ) -> Self
    where
        F: Fn(f32) -> f32,
    {
        let labels = transform_to_format_imstrings(ui, xlims, axis, use_ms, relative);
        let axis_label = {
            let label = axis.map(|axis| axis.name()).unwrap_or_else(String::new);
            let label = ImString::new(label);
            let text_size = ui.calc_text_size(&label);
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

    pub fn draw(self, draw_list: &DrawListMut, p: [f32; 2], size: [f32; 2]) {
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

    pub fn prepare<F>(
        ui: &Ui,
        ylims: (f32, f32),
        axis: Option<&AxisTransform<F>>,
        (use_ms, relative): (bool, bool),
    ) -> Self
    where
        F: Fn(f32) -> f32,
    {
        let labels = transform_to_format_imstrings(ui, ylims, axis, use_ms, relative);
        let axis_label = {
            let label = axis.map(|axis| axis.name()).unwrap_or_else(String::new);
            let label = ImString::new(label);
            let text_size = ui.calc_text_size(&label);
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

    pub fn draw(self, draw_list: &DrawListMut, p: [f32; 2], size: [f32; 2]) {
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
                    y_pos - text_size[1],
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
    C: Into<ImColor32>,
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
