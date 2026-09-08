use web_time::Duration;

/// Duration of the whole animation.
const DURATION: Duration = Duration::new(3, 0); // 3 seconds

#[derive(Debug, Default, Clone)]
pub struct SpecialAnimationState {
    progress: Option<f32>,
}
impl SpecialAnimationState {
    /// Steps the animation forward. Returns whether the puzzle should be
    /// redrawn next frame.
    pub fn proceed(&mut self, delta: Duration) -> bool {
        if let Some(progress) = &mut self.progress {
            *progress += delta.as_secs_f32() / DURATION.as_secs_f32();
            if *progress > 1.0 {
                self.progress = None;
            }
            true
        } else {
            false
        }
    }

    pub fn get(&self) -> Option<f32> {
        self.progress
    }

    pub fn start(&mut self) {
        if self.progress.is_none() {
            self.progress = Some(0.0);
        }
    }
}
