use super::interactions::{
    Interaction, InteractionId, InteractionIterMut, Interactions, PersistenceFilter, ValueIter,
};
use super::Error;
use crate::imshow::cake::{OutputId, TransformIdx};
use aflak_primitives::PersistencePairs;
use imgui::{MenuItem, MouseButton, MouseCursor, Ui};
use implot::{
    get_plot_limits, get_plot_mouse_position, is_plot_hovered, pixels_to_plot_f32,
    plot_to_pixels_f32, push_style_color, push_style_var_i32, set_colormap_from_preset, AxisFlags,
    Colormap, Condition, ImPlotLimits, ImPlotRange, Marker, Plot, PlotColorElement, PlotFlags,
    PlotLine, PlotUi, StyleVar, YAxisChoice,
};
use std::collections::HashMap;
type EditableValues = HashMap<InteractionId, TransformIdx>;

pub struct State {
    interactions: Interactions,
    persistence_filter: Option<f64>,
    filter_moving: bool,
    reset_view: bool,
    now_plot_limits: ImPlotLimits,
}

impl Default for State {
    fn default() -> Self {
        State {
            interactions: Interactions::new(),
            persistence_filter: None,
            filter_moving: false,
            reset_view: true,
            now_plot_limits: ImPlotLimits {
                X: ImPlotRange { Min: 0.0, Max: 1.0 },
                Y: ImPlotRange { Min: 0.0, Max: 1.0 },
            },
        }
    }
}

impl State {
    pub(crate) fn persistence_diagram(
        &mut self,
        ui: &Ui,
        data: &PersistencePairs,
        plot_ui: &PlotUi,
        _copying: &mut Option<(InteractionId, TransformIdx)>,
        _store: &mut EditableValues,
        _attaching: &mut Option<(OutputId, TransformIdx, usize)>,
        created_on: std::time::Instant,
        _outputid: OutputId,
    ) -> Result<(), Error> {
        let PersistencePairs::Pairs(data) = data;
        if data.len() < 2 {
        } else {
            let mut xmin = f32::MAX;
            let mut xmax = f32::MIN;
            let mut ymax = f32::MIN;
            for (_, _, birth, death) in data {
                if xmin > *birth {
                    xmin = *birth;
                }
                if xmax < *birth {
                    xmax = *birth;
                }
                if ymax < *death {
                    ymax = *death;
                }
            }
            let content_area = ui.content_region_avail();
            let mut data = data.clone();
            data.sort_by(|a, b| (a.3 - a.2).partial_cmp(&(b.3 - b.2)).unwrap());
            data.reverse();
            self.now_plot_limits = if self.reset_view {
                self.reset_view = false;
                ImPlotLimits {
                    X: ImPlotRange {
                        Min: (2.0 * xmin - xmax) as f64,
                        Max: (2.0 * xmax - xmin) as f64,
                    },
                    Y: ImPlotRange {
                        Min: xmin as f64 - 1.0,
                        Max: ymax as f64 + 1.0,
                    },
                }
            } else {
                self.now_plot_limits
            };
            let plot_cond = if self.filter_moving {
                Condition::Always
            } else {
                Condition::FirstUseEver
            };
            Plot::new(format!("Persistence Diagram##{:?}", created_on).as_str())
                .size([content_area[0], content_area[1]])
                .x_label("Birth")
                .y_label("Death")
                .x_limits(
                    (self.now_plot_limits.X.Min, self.now_plot_limits.X.Max),
                    plot_cond,
                )
                .y_limits(
                    (self.now_plot_limits.Y.Min, self.now_plot_limits.Y.Max),
                    YAxisChoice::First,
                    plot_cond,
                )
                .with_plot_flags(&(PlotFlags::NONE))
                .with_y_axis_flags(YAxisChoice::First, &(AxisFlags::NONE))
                .build(plot_ui, || {
                    let plot_limits = Some(get_plot_limits(None));
                    self.now_plot_limits = plot_limits.unwrap();
                    let hover_pos_plot = get_plot_mouse_position(None);
                    if is_plot_hovered() {
                        if ui.is_mouse_clicked(MouseButton::Right) {
                            ui.open_popup(format!("add-pf"));
                        }
                    }
                    ui.popup(format!("add-pf"), || {
                        if MenuItem::new(&format!("Add persistence filter")).build(ui) {
                            self.persistence_filter = Some(hover_pos_plot.y - hover_pos_plot.x);
                            let new = Interaction::PersistenceFilter(PersistenceFilter::new(
                                self.persistence_filter.unwrap() as f32,
                            ));
                            self.interactions.insert(new);
                        }
                    });
                    if let Some(plot_limits) = plot_limits {
                        let pmin = plot_limits.X.Min.min(plot_limits.Y.Min);
                        let pmax = plot_limits.X.Max.max(plot_limits.Y.Max);
                        PlotLine::new(format!("y=x").as_str())
                            .plot(&vec![pmin, pmax], &vec![pmin, pmax]);
                        for (id, interaction) in self.interactions.iter_mut() {
                            let stack = ui.push_id(id.id());
                            match interaction {
                                Interaction::PersistenceFilter(PersistenceFilter {
                                    val,
                                    moving,
                                }) => {
                                    PlotLine::new(format!("persistence_filter").as_str()).plot(
                                        &vec![pmin, pmax],
                                        &vec![pmin + *val as f64, pmax + *val as f64],
                                    );
                                    let ps_pixel_y = plot_to_pixels_f32(
                                        hover_pos_plot.x,
                                        hover_pos_plot.x + *val as f64,
                                        Some(YAxisChoice::First),
                                    )
                                    .y;
                                    let (pf_clickable_ll, pf_clickable_ul) = (
                                        pixels_to_plot_f32(
                                            0.0,
                                            ps_pixel_y + 10.0,
                                            Some(YAxisChoice::First),
                                        )
                                        .y,
                                        pixels_to_plot_f32(
                                            0.0,
                                            ps_pixel_y - 10.0,
                                            Some(YAxisChoice::First),
                                        )
                                        .y,
                                    );
                                    if pf_clickable_ll < hover_pos_plot.y
                                        && hover_pos_plot.y < pf_clickable_ul
                                        && is_plot_hovered()
                                    {
                                        ui.set_mouse_cursor(Some(MouseCursor::ResizeNS));
                                        if ui.is_mouse_down(MouseButton::Left) {
                                            *moving = true;
                                        }
                                    }
                                    if *moving {
                                        *val = (hover_pos_plot.y - hover_pos_plot.x) as f32;
                                    }
                                    if !ui.is_mouse_down(MouseButton::Left) {
                                        *moving = false;
                                    }
                                    self.persistence_filter = Some(*val as f64);
                                    self.filter_moving = *moving;
                                }
                                _ => {}
                            }
                            stack.pop();
                        }
                    }
                    //Diagram
                    //set_colormap_from_preset(Colormap::Plasma, data.len() as u32);

                    let markerchoice = push_style_var_i32(&StyleVar::Marker, Marker::Circle as i32);
                    let max_persistence = data[0].3 - data[0].2;
                    for (_, _, birth, death) in data {
                        let mut pushed_var = push_style_color(
                            &PlotColorElement::Line,
                            1.0,
                            1.0,
                            1.0,
                            (death - birth) / max_persistence + 0.5,
                        );
                        if let Some(persistence) = self.persistence_filter {
                            if ((death - birth) as f64) < persistence {
                                pushed_var.pop();
                                pushed_var = push_style_color(
                                    &PlotColorElement::Line,
                                    0.5,
                                    0.5,
                                    1.0,
                                    (death - birth) / max_persistence + 0.5,
                                );
                                PlotLine::new(format!("##P: {}", death - birth).as_str()).plot(
                                    &vec![birth as f64, birth as f64],
                                    &vec![birth as f64, death as f64],
                                );
                            } else {
                                PlotLine::new(format!("P: {}", death - birth).as_str()).plot(
                                    &vec![birth as f64, birth as f64],
                                    &vec![birth as f64, death as f64],
                                );
                            }
                        } else {
                            PlotLine::new(format!("P: {}", death - birth).as_str()).plot(
                                &vec![birth as f64, birth as f64],
                                &vec![birth as f64, death as f64],
                            );
                        }
                        pushed_var.pop();
                    }
                    set_colormap_from_preset(Colormap::Standard, 0);
                    markerchoice.pop();
                });
        }
        Ok(())
    }

    pub fn stored_values(&self) -> ValueIter {
        self.interactions.value_iter()
    }

    pub fn stored_values_mut(&mut self) -> InteractionIterMut {
        self.interactions.iter_mut()
    }
}
