use hypermath::prelude::*;
use hypershape::prelude::*;
use itertools::Itertools;

pub use super::*;

lua_userdata_value_conversion_wrapper! {
    #[name = "coxeter group", convert_str = "coxeter group"]
    pub struct LuaCoxeterGroup(SchlafliSymbol) = |_lua| {
        <LuaTable<'_>>(t) => Ok(LuaCoxeterGroup::construct_from_table(t)?),
    }
}

impl LuaCoxeterGroup {
    fn construct_from_table(t: LuaTable<'_>) -> LuaResult<SchlafliSymbol> {
        t.sequence_values()
            .try_collect()
            .map(SchlafliSymbol::from_indices)
    }

    pub fn construct_from_cd_table(
        _lua: LuaContext<'_>,
        t: LuaTable<'_>,
    ) -> LuaResult<LuaCoxeterGroup> {
        Self::construct_from_table(t).map(LuaCoxeterGroup)
    }
}

impl LuaNamedUserData<SchlafliSymbol> {}

impl LuaUserData for LuaNamedUserData<SchlafliSymbol> {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_meta_method(LuaMetaMethod::Call, |lua, Self(this), args| {
            expand_vec_lua_iter(lua, this, args)
        });

        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, Self(this), ()| {
            Ok(this.indices().iter().map(|i| i.to_string()).join("o"))
        });

        methods.add_method("expand", |lua, Self(this), args| {
            expand_vec_lua_iter(lua, this, args)
        });

        methods.add_method("ndim", |_lua, Self(this), ()| Ok(this.ndim()));

        methods.add_method("vec", |lua, Self(this), args| {
            Ok(LuaVector(vec_from_args(lua, this, args)?))
        });
    }

    fn get_uvalues_count(&self) -> std::os::raw::c_int {
        1
    }
}

fn expand_vec_lua_iter<'lua>(
    lua: LuaContext<'lua>,
    s: &SchlafliSymbol,
    args: LuaMultiValue<'lua>,
) -> LuaResult<LuaFunction<'lua>> {
    let vector = vec_from_args(lua, s, args)?;
    let mut vectors_iter = s
        .expand(vector, |t, point| t.transform_vector(point))
        .into_iter()
        .map(LuaVector);
    lua.create_function_mut(move |_lua, ()| Ok(vectors_iter.next()))
}

fn vec_from_args<'lua>(
    lua: LuaContext<'lua>,
    s: &SchlafliSymbol,
    args: LuaMultiValue<'lua>,
) -> LuaResult<Vector> {
    if let Ok(string) = String::from_lua_multi(args.clone(), lua) {
        Ok(mirror_basis(s)? * parse_wendy_krieger_vector(s.ndim(), &string)?)
    } else if let Ok(LuaConstructVector(v)) = <_>::from_lua_multi(args, lua) {
        Ok(v)
    } else {
        Err(LuaError::external(
            "expected vector constructor or coxeter vector string",
        ))
    }
}

fn mirror_basis(s: &SchlafliSymbol) -> LuaResult<Matrix> {
    s.mirror_basis()
        .ok_or_else(|| LuaError::external("coxeter diagram matrix be invertible"))
}

fn parse_wendy_krieger_vector(ndim: u8, s: &str) -> LuaResult<Vector> {
    // if s.starts_with('|')&& s.ends_with('|') {
    //     s.strip_prefix('|').and_then(|s|s.strip_suffix('|'))
    // }
    if s.len() != ndim as usize {
        return Err(LuaError::external(format!(
            "expected coxeter vector of length {ndim}"
        )));
    }
    s.chars()
        .map(|c| match c {
            // Blame Wendy Krieger for this notation.
            // https://bendwavy.org/klitzing/explain/dynkin-notation.htm
            'o' => Ok(0.0),
            'x' => Ok(1.0),
            'q' => Ok(std::f64::consts::SQRT_2),
            'f' => Ok((5.0_f64.sqrt() + 1.0) * 0.5), // phi
            'u' => Ok(2.0),
            _ => Err(LuaError::external(
                "invalid character for coxeter vector. supported characters: [o, x, q, f, u]",
            )),
        })
        .collect()
}
