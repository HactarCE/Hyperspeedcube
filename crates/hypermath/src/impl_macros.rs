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

macro_rules! impl_for_tuples {
    ($impl_macro:ident) => {
        $impl_macro!(T0; 0);
        $impl_macro!(T0, T1; 0, 1);
        $impl_macro!(T0, T1, T2; 0, 1, 2);
        $impl_macro!(T0, T1, T2, T3; 0, 1, 2, 3);
        $impl_macro!(T0, T1, T2, T3, T4; 0, 1, 2, 3, 4);
        $impl_macro!(T0, T1, T2, T3, T4, T5; 0, 1, 2, 3, 4, 5);
        $impl_macro!(T0, T1, T2, T3, T4, T5, T6; 0, 1, 2, 3, 4, 5, 6);
        $impl_macro!(T0, T1, T2, T3, T4, T5, T6, T7; 0, 1, 2, 3, 4, 5, 6, 7);
        $impl_macro!(T0, T1, T2, T3, T4, T5, T6, T7, T8; 0, 1, 2, 3, 4, 5, 6, 7, 8);
        $impl_macro!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9);
    };
}
