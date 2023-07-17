macro_rules! impl_forward_bin_ops_to_ref {
    () => {};

    (
        impl $trait:ident for $type:ty { fn $func:ident() }
        $($remainder:tt)*
    ) => {
        impl_forward_bin_ops_to_ref! {
            impl $trait<$type> for $type { fn $func() -> $type }
            $($remainder)*
        }
    };

    (
        impl $trait:ident<$rhs:ty> for $type:ty { fn $func:ident() -> $ret:ty }
        $($remainder:tt)*
    ) => {
        impl $trait<$rhs> for $type {
            type Output = $ret;

            fn $func(self, rhs: $rhs) -> $ret {
                $trait::$func(&self, &rhs)
            }
        }
        impl<'a> $trait<$rhs> for &'a $type {
            type Output = $ret;

            fn $func(self, rhs: $rhs) -> $ret {
                $trait::$func(self, &rhs)
            }
        }
        impl<'a> $trait<&'a $rhs> for $type {
            type Output = $ret;

            fn $func(self, rhs: &'a $rhs) -> $ret {
                $trait::$func(&self, rhs)
            }
        }
        impl_forward_bin_ops_to_ref! { $($remainder)* }
    };
}

macro_rules! impl_forward_assign_ops_to_owned {
    () => {};

    (
        impl $trait:ident for $type:ty { fn $func:ident() { $op:tt } }
        $($remainder:tt)*
    ) => {
        impl_forward_assign_ops_to_owned! {
            impl $trait<$type> for $type { fn $func() { $op } }
            $($remainder)*
        }
    };

    (
        impl $trait:ident<$rhs:ty> for $type:ty { fn $func:ident() { $op:tt } }
        $($remainder:tt)*
    ) => {
        impl $trait<$rhs> for $type {
            fn $func(&mut self, rhs: $rhs) {
                *self = std::mem::take(self) $op &rhs;
            }
        }
        impl<'a> $trait<&'a $rhs> for $type {
            fn $func(&mut self, rhs: &'a $rhs) {
                *self = std::mem::take(self) $op rhs;
            }
        }
        impl_forward_assign_ops_to_owned! { $($remainder)* }
    };
}

macro_rules! impl_mul_sign {
    (impl Mul<Sign> for $type:ty) => {
        impl std::ops::Mul<$crate::math::Sign> for $type {
            type Output = Self;

            fn mul(self, rhs: $crate::math::Sign) -> Self {
                match rhs {
                    $crate::math::Sign::Pos => self,
                    $crate::math::Sign::Neg => -self,
                }
            }
        }
    };
}
macro_rules! impl_mulassign_sign {
    (impl MulAssign<Sign> for $type:ty) => {
        impl std::ops::MulAssign<$crate::math::Sign> for $type {
            fn mul_assign(&mut self, rhs: $crate::math::Sign) {
                match rhs {
                    $crate::math::Sign::Pos => (),
                    $crate::math::Sign::Neg => *self = -*self,
                }
            }
        }
    };
}
