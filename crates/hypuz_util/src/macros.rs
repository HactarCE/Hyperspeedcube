/// Defines one or more structs that are simple wrappers around primitive
/// unsigned integer types and implements `TypedIndex` on them.
#[macro_export]
macro_rules! typed_index_struct {
    (
        $(
            $(#[$attr:meta])*
            $struct_vis:vis struct $struct_name:ident($inner_vis:vis $inner_type:ty);
        )+
    ) => {
        $(
            $(#[$attr])*
            #[derive(Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
            #[repr(transparent)]
            $struct_vis struct $struct_name($inner_vis $inner_type);

            impl ::std::fmt::Debug for $struct_name {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    write!(f, "#{:?}", self.0)
                }
            }
            impl ::std::fmt::Display for $struct_name {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    write!(f, "#{}", self.0)
                }
            }

            impl $crate::ti::Fits64 for $struct_name {
                unsafe fn from_u64(x: u64) -> Self {
                    Self(x as _)
                }

                fn to_u64(self) -> u64 {
                    self.0 as u64
                }
            }

            impl $crate::ti::TypedIndex for $struct_name {
                const MAX: Self = Self($crate::typed_index_struct!(@conservative_max($inner_type)));
                const MAX_INDEX: usize = $crate::typed_index_struct!(@conservative_max($inner_type)) as usize;
                const TYPE_NAME: &'static str = stringify!($struct_name);

                fn to_index(self) -> usize {
                    $crate::ti::Fits64::to_u64(self) as usize
                }

                fn try_from_index(index: usize) -> Result<Self, $crate::error::IndexOverflow> {
                    match index.try_into() {
                        Ok(i) => Ok(Self(i)),
                        Err(_) => Err($crate::error::IndexOverflow::new::<Self>()),
                    }
                }
            }
        )+
    };

    (@ conservative_max ($inner_type:ty)) => {
        // max value for the type, but subtract 1 if it's larger than a u16
        // (so overflow is less likely with `usize`)
        <$inner_type>::MAX - (<$inner_type>::MAX as usize > u16::MAX as usize) as $inner_type
    };
}
