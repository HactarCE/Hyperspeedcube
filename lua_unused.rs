impl<'lua> ToLua<'lua> for cga::Blade {
    fn to_lua(self, lua: LuaContext<'lua>) -> LuaResult<LuaValue<'lua>> {
        self.into_mv().to_lua(lua)
    }
}
impl<'lua> FromLua<'lua> for cga::Blade {
    fn from_lua(lua_value: LuaValue<'lua>, lua: LuaContext<'lua>) -> LuaResult<Self> {
        cga::Blade::try_from(LuaMultivector::from_lua(lua_value, lua)?.0).map_err(
            |cga::MismatchedGrade| LuaError::FromLuaConversionError {
                from: "multivector",
                to: "blade",
                message: Some("mismatched grade".to_owned()),
            },
        )
    }
}
