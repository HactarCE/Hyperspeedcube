use super::Window;

pub(crate) const TIMER: Window = Window {
    name: "Timer",
    build: |ui, app| {
        if ui
            .add(egui::Button::new(
                egui::RichText::new(match app.timer_start_end {
                    (None, None) => "click".into(),
                    (None, Some(_)) => panic!("invalid timer state"),
                    (Some(start), None) => format!("{:.3}", start.elapsed().as_secs_f32()),
                    (Some(start), Some(end)) => format!("{:.3}", (end - start).as_secs_f32()),
                })
                .size(20.0),
            ))
            .clicked()
        {
            app.timer_start_end = match app.timer_start_end {
                (None, None) => (Some(std::time::Instant::now()), None),
                (None, Some(_)) => panic!("invalid timer state"),
                (Some(start), None) => (Some(start), Some(std::time::Instant::now())),
                (Some(_), Some(_)) => (Some(std::time::Instant::now()), None),
            };
        }
        // TODO: make Command::ToggleTimer or some other way for keyboard input
    },
    ..Window::DEFAULT
};
