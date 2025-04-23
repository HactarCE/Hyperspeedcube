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
