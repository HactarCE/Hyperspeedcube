//! Interpolation functions.

use std::f32::consts::PI;

/// Function that maps a float from the range 0.0 to 1.0 to another float
/// from 0.0 to 1.0.
pub enum InterpolateFn {
    Lerp,             // true neutral
    Cosine,           // neutral good
    Bounce,           // chaotic neutral
    Overshoot,        // chaotic good
    Underdamped,      // lawful evil
    CriticallyDamped, // lawful good
}

impl InterpolateFn {
    /// Returns the interpolation value in the range [0, 1] for `t` in the range
    /// [0, 1].
    pub fn interpolate(self, mut t: f32) -> f32 {
        match self {
            InterpolateFn::Lerp => t,

            InterpolateFn::Cosine => (1.0 - (t * PI).cos()) / 2.0,

            InterpolateFn::Bounce => {
                // https://easings.net/#easeOutBounce
                let n1 = 7.5625;
                let d1 = 2.75;

                if t < 1.0 / d1 {
                    n1 * t * t
                } else if t < 2.0 / d1 {
                    t -= 1.5 / d1;
                    n1 * t * t + 0.75
                } else if t < 2.5 / d1 {
                    t -= 2.25 / d1;
                    n1 * t * t + 0.9375
                } else {
                    t -= 2.625 / d1;
                    n1 * t * t + 0.984375
                }
            }
            InterpolateFn::Overshoot => {
                // https://easings.net/#easeOutBack
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                1.0 + c3 * (t - 1.0).powf(3.0) + c1 * (t - 1.0).powf(2.0)
            }
            InterpolateFn::Underdamped => {
                // https://easings.net/#easeOutElastic
                let c4 = (2.0 * PI) / 3.0;
                2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
            }
            InterpolateFn::CriticallyDamped => {
                // fine-tuned by Milo Jacquet
                (-5.0 * t - 1.0) * (-8.0 * t).exp() + 1.0
            }
        }
    }
}
