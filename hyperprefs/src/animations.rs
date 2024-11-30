use serde::{Deserialize, Serialize};

pub use interpolation::InterpolateFn;

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[serde(default)]
pub struct AnimationPreferences {
    pub dynamic_twist_speed: bool,
    pub twist_duration: f32,
    pub blocking_anim_duration: f32,
    pub twist_interpolation: InterpolateFn,
}

pub mod interpolation {
    //! Interpolation functions.

    use std::f32::consts::PI;

    use rand::Rng;
    use serde::{Deserialize, Serialize};

    /// Function that maps a float from the range 0.0 to 1.0 to another float
    /// from 0.0 to 1.0.
    #[derive(
        Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq, Eq, Hash, VariantArray,
    )]
    #[serde(rename_all = "snake_case")]
    pub enum InterpolateFn {
        Lerp,
        #[default]
        Cosine,
        Cubic,
        Circular,
        Bounce,
        Overshoot,
        Underdamped,
        CriticallyDamped,
        CriticallyDried,
        Random,
    }

    impl InterpolateFn {
        /// Returns the interpolation value in the range [0, 1] for `t` in the
        /// range [0, 1].
        pub fn interpolate(self, mut t: f32) -> f32 {
            match self {
                Self::Lerp => t,

                Self::Cosine => (1.0 - (t * PI).cos()) / 2.0,

                Self::Cubic => (3.0 - 2.0 * t) * t * t,

                Self::Circular => {
                    if t < 0.5 {
                        (1.0 - (1.0 - (2.0 * t).powf(2.0)).sqrt()) * 0.5
                    } else {
                        (1.0 + (1.0 - (-2.0 * t + 2.0).powf(2.0)).sqrt()) * 0.5
                    }
                }

                Self::Bounce => {
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
                Self::Overshoot => {
                    // https://easings.net/#easeOutBack
                    let c1 = 1.70158;
                    let c3 = c1 + 1.0;
                    1.0 + c3 * (t - 1.0).powf(3.0) + c1 * (t - 1.0).powf(2.0)
                }
                Self::Underdamped => {
                    // https://easings.net/#easeOutElastic
                    let c4 = (2.0 * PI) / 3.0;
                    2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
                }
                Self::CriticallyDamped => {
                    // fine-tuned by Milo Jacquet
                    (-5.0 * t - 1.0) * (-8.0 * t).exp() + 1.0
                }
                Self::CriticallyDried => 1.0 - Self::CriticallyDamped.interpolate(1.0 - t),

                Self::Random => t + rand::thread_rng().gen_range(-3.0..3.0) * t * (t - 1.0),
            }
        }
    }
}
