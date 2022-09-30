macro_rules! impl_forward_bin_ops_to_ref {
    ($(
        impl $trait:ident for $type:ty { fn $func:ident() }
    )*) => {
        $(
            impl $trait<$type> for $type {
                type Output = $type;

                fn $func(self, rhs: $type) -> $type {
                    $trait::$func(&self, &rhs)
                }
            }
            impl<'a> $trait<$type> for &'a $type {
                type Output = $type;

                fn $func(self, rhs: $type) -> $type {
                    $trait::$func(self, &rhs)
                }
            }
            impl<'a> $trait<&'a $type> for $type {
                type Output = $type;

                fn $func(self, rhs: &'a $type) -> $type {
                    $trait::$func(&self, rhs)
                }
            }
        )*
    };
}
