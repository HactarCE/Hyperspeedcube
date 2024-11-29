use instant::{Duration, Instant};

use crate::gui::ext::ResponseExt;

use super::Window;

// TODO: resizing of timer text (eg keybind reference)
// TODO: should Timer/Stopwatch be in components?

pub(crate) const TIMER: Window = Window {
    name: "Timer",
    build: |ui, app| {
        ui.add(egui::Button::new(
            egui::RichText::new(match app.timer.stopwatch {
                Stopwatch::NotStarted => "Ready".into(),
                Stopwatch::Running(start) => duration_to_str(start.elapsed()),
                Stopwatch::Stopped(duration) => duration_to_str(duration),
            })
            .size(20.0),
        ));
        if ui
            .selectable_label(app.timer.is_blind, "Blind mode")
            .on_hover_explanation(
                "normal mode : blind mode",
                "start on (first twist : scramble)\nstop on (solved : blindfold off)\ntoggling will reset the timer and puzzle",
            )
            .clicked()
        {
            app.timer.is_blind ^= true;
            app.timer.stopwatch.reset();
            app.puzzle.reset();
        }
    },
    ..Window::DEFAULT
};

#[derive(Debug)]
pub(crate) enum Stopwatch {
    NotStarted,
    Running(Instant),
    Stopped(Duration),
}
impl Stopwatch {
    fn reset(&mut self) {
        *self = Stopwatch::NotStarted;
    }

    fn start(&mut self) {
        if let Self::NotStarted = self {
            *self = Self::Running(Instant::now());
        } else {
            debug_assert!(false, "Can only start a NotStarted timer. This is a horrible unrecoverable logic error in the scope of timer, but it's recoverable in the scope of the entire program.");
            self.reset();
        }
    }

    fn stop(&mut self) {
        if let Self::Running(beginning) = *self {
            *self = Self::Stopped(beginning.elapsed());
        } else {
            debug_assert!(false, "Can only stop a Running timer. This is a horrible unrecoverable logic error in the scope of timer, but it's recoverable in the scope of the entire program.");
            self.reset();
        }
    }
}

#[derive(Debug)]
pub(crate) struct Timer {
    stopwatch: Stopwatch,
    is_blind: bool,
}
impl Timer {
    pub(crate) fn new() -> Self {
        Self {
            stopwatch: Stopwatch::NotStarted,
            is_blind: false,
        }
    }

    pub(crate) fn on_scramble(&mut self) {
        self.stopwatch.reset();
        if self.is_blind {
            self.stopwatch.start();
        }
    }

    pub(crate) fn on_non_rotation_twist(&mut self) {
        // check if the twist is the first one
        if !self.is_blind && matches!(self.stopwatch, Stopwatch::NotStarted) {
            self.stopwatch.start();
        }
    }

    pub(crate) fn on_solve(&mut self) {
        if !self.is_blind {
            self.stopwatch.stop();
        }
    }

    pub(crate) fn on_blindfold_off(&mut self) {
        if self.is_blind {
            self.stopwatch.stop();
        }
    }
}

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
