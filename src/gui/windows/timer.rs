use std::time::{Duration, Instant};

use super::Window;

// TODO: start/stop timer with keyboard input (Command::ToggleTimer maybe)
// TODO: start timer on mouse-release instead of mouse-down
// TODO: allow resizing the window

pub(crate) const TIMER: Window = Window {
    name: "Timer",
    build: |ui, app| {
        if ui
            .add(egui::Button::new(
                egui::RichText::new(match app.timer_start_end {
                    (None, None) => "click".into(),
                    (None, Some(_)) => panic!("invalid timer state"),
                    (Some(start), None) => duration_to_str(start.elapsed()),
                    (Some(start), Some(end)) => duration_to_str(end - start),
                })
                .size(20.0),
            ))
            .clicked()
        {
            app.timer_start_end = match app.timer_start_end {
                (None, None) => (Some(Instant::now()), None),
                (None, Some(_)) => panic!("invalid timer state"),
                (Some(start), None) => (Some(start), Some(Instant::now())),
                (Some(_), Some(_)) => (Some(Instant::now()), None),
            };
        }
    },
    ..Window::DEFAULT
};

fn duration_to_str(duration: Duration) -> String {
    let milliseconds = duration.as_millis();
    let seconds = milliseconds / 1000;
    let minutes = seconds / 60;
    let hours = minutes / 60;

    debug_assert_eq!(
        60 * 60 * 1000 * hours
            + 60 * 1000 * (minutes % 60)
            + 1000 * (seconds % 60)
            + milliseconds % 1000,
        duration.as_millis()
    );

    [
        if hours == 0 {
            "".to_owned()
        } else {
            format!("{}:", hours)
        },
        if minutes == 0 {
            "".to_owned()
        } else if hours == 0 {
            format!("{}:", minutes % 60)
        } else {
            format!("{:02}:", minutes % 60)
        },
        if minutes == 0 {
            format!("{}.", seconds % 60)
        } else {
            format!("{:02}.", seconds % 60)
        },
        format!("{:03}", milliseconds % 1000),
    ]
    .concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_duration_to_str() {
        for (s, millis) in [
            ("0.000", 0),
            ("0.001", 1),
            ("0.010", 10),
            ("0.100", 100),
            ("1.000", 1000),
            ("10.000", 10000),
            ("1:00.000", 60000),
            ("1:01.000", 61000),
            ("1:10.000", 70000),
            ("10:00.000", 600000),
            ("11:00.000", 660000),
            ("11:10.000", 670000),
            ("11:11.000", 671000),
            ("1:00:00.000", 3600000),
            ("10:00:00.000", 36000000),
            ("100:00:00.000", 360000000),
            ("23:02:14.903", 82934903),
        ] {
            assert_eq!(s, duration_to_str(Duration::from_millis(millis)));
        }
    }
}
