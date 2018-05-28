use imgui::{ImString, Ui, WindowDrawList};

pub fn add_ticks(
    ui: &Ui,
    draw_list: &WindowDrawList,
    p: (f32, f32),
    size: (f32, f32),
    xlims: (f32, f32),
    ylims: (f32, f32),
) {
    // Add ticks
    const COLOR: u32 = 0xFFFFFFFF;
    const TICK_COUNT: u32 = 10;
    const TICK_SIZE: f32 = 3.0;
    const LABEL_HORIZONTAL_PADDING: f32 = 2.0;

    // X-axis
    let x_step = size.0 / TICK_COUNT as f32;
    let mut x_pos = p.0;
    let y_pos = p.1 + size.1;
    for i in 0..=TICK_COUNT {
        draw_list
            .add_line([x_pos, y_pos], [x_pos, y_pos - TICK_SIZE], COLOR)
            .build();
        let i = i as f32;
        let label = ImString::new(format!("{:.0}", xlims.0 + i * xlims.1 / TICK_COUNT as f32));
        let text_size = ui.calc_text_size(&label, false, -1.0);
        draw_list.add_text([x_pos - text_size.x / 2.0, y_pos], COLOR, label.to_str());
        x_pos += x_step;
    }

    // Y-axis
    let y_step = size.1 / TICK_COUNT as f32;
    let mut y_pos = p.1 + size.1;
    let x_pos = p.0;
    for i in 0..=TICK_COUNT {
        draw_list
            .add_line([x_pos, y_pos], [x_pos + TICK_SIZE, y_pos], COLOR)
            .build();
        let i = i as f32;
        let label = ImString::new(format!("{:.0}", ylims.0 + i * ylims.1 / TICK_COUNT as f32));
        let text_size = ui.calc_text_size(&label, false, -1.0);
        draw_list.add_text(
            [
                x_pos - text_size.x - LABEL_HORIZONTAL_PADDING,
                y_pos - text_size.y / 2.0,
            ],
            COLOR,
            label.to_str(),
        );
        y_pos -= y_step;
    }
}
