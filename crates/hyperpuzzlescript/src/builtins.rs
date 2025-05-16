use ecow::eco_format;
use hypermath::approx_eq;
use itertools::Itertools;
use std::io::Write;

use crate::{
    eval::Ctx,
    ty::{FnType, Type},
    value::{FnOverload, Value},
};

pub fn add_builtin_functions(ctx: &mut Ctx) {
    ctx.register_func(
        "str",
        FnOverload {
            ty: FnType {
                params: Some(vec![Type::Any]),
                ret: Type::Str,
            },
            ptr: Box::new(|ctx, args| Ok(Value::Str(eco_format!("{}", args[0])))),
        },
    );
    ctx.register_func(
        "print",
        FnOverload {
            ty: FnType {
                params: None,
                ret: Type::Null,
            },
            ptr: Box::new(|ctx, args| {
                let mut stdout = std::io::stdout().lock();
                let mut is_first = true;
                for arg in args {
                    if is_first {
                        is_first = false;
                    } else {
                        write!(stdout, " ").unwrap();
                    }
                    write!(stdout, "{}", arg.to_string()).unwrap();
                }
                writeln!(stdout).unwrap();
                Ok(Value::Null)
            }),
        },
    );
    ctx.register_func(
        "+",
        FnOverload {
            ty: FnType {
                params: Some(vec![Type::Num, Type::Num]),
                ret: Type::Num,
            },
            ptr: Box::new(|ctx, args| Ok(Value::Num(args[0].unwrap_num() + args[1].unwrap_num()))),
        },
    );
    ctx.register_func(
        "-",
        FnOverload {
            ty: FnType {
                params: Some(vec![Type::Num, Type::Num]),
                ret: Type::Num,
            },
            ptr: Box::new(|ctx, args| Ok(Value::Num(args[0].unwrap_num() - args[1].unwrap_num()))),
        },
    );
}
