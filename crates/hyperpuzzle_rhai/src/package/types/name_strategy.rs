use hypermath::{ApproxHashMap, ApproxHashMapKey, IndexNewtype, TransformByMotor, pga};

use super::{symmetry::RhaiSymmetry, *};

pub enum RhaiNameStrategy {
    Constant(String),
    Function(FnPtr),
    GenSeqMap(Map),
}

// pub type RhaiNameFn<'a, T> = Box<dyn 'a + Fn(&pga::Motor, &T) -> Result<Option<String>>>;
// pub type RhaiNameFnWithArgs<'a, T, A> =
//     Box<dyn 'a + Fn(&pga::Motor, &T, A) -> Result<Option<String>>>;

impl Default for RhaiNameStrategy {
    fn default() -> Self {
        Self::Constant(String::new())
    }
}

impl FromRhai for RhaiNameStrategy {
    fn expected_string() -> String {
        "string, function, or map".into()
    }

    fn try_from_rhai(ctx: impl RhaiCtx, value: Dynamic) -> Result<Self, ConvertError> {
        if value.is_string() {
            // Allow naming just the first element, for prototyping.
            Ok(Self::Constant(from_rhai::<String>(ctx, value)?))
        } else if value.is_fnptr() {
            Ok(Self::Function(from_rhai::<FnPtr>(ctx, value)?))
        } else if value.is_map() {
            Ok(Self::GenSeqMap(from_rhai::<Map>(ctx, value)?))
        } else {
            Err(ConvertError::new::<Self>(ctx, Some(&value)))
        }
    }

    fn try_from_none(_ctx: impl RhaiCtx) -> Result<Self, ConvertError> {
        Ok(Self::default())
    }
}

impl RhaiNameStrategy {
    /// Returns whether the strategy assigns a constant empty name.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Constant(s) => s.is_empty(),
            _ => false,
        }
    }

    /// Returns a function that can be used to name the elements of an orbit.
    ///
    /// This may be expensive to construct, so try to reuse it if possible.
    pub fn name_fn<'a, I: IndexNewtype, T: 'a + ApproxHashMapKey + Clone + TransformByMotor>(
        &'a self,
        ctx: &'a Ctx<'_>,
        symmetry: Option<&RhaiSymmetry>,
        object: &T,
    ) -> Result<RhaiNameFn<T>> {
        match self {
            Self::Constant(name) => Ok(RhaiNameFn::Constant(name.clone())),
            Self::Function(f) => Ok(RhaiNameFn::Function(f.clone())),
            Self::GenSeqMap(map) => {
                let mut value_to_name = ApproxHashMap::<T, String>::new();
                for (gen_seq, name) in orbit_names::names_from_map::<I>(ctx, map.clone())? {
                    match symmetry {
                        Some(sym) => {
                            let motor = sym.motor_for_gen_seq(&gen_seq)?;
                            value_to_name.insert(motor.transform(object), name.spec);
                        }
                        None if gen_seq.0.is_empty() => {
                            value_to_name.insert(object.clone(), name.spec);
                        }
                        None => (), // oh well
                    }
                }
                Ok(RhaiNameFn::GenSeqMap(value_to_name))
            }
        }
    }
}

#[derive(Debug)]
pub enum RhaiNameFn<T: ApproxHashMapKey> {
    Constant(String),
    Function(FnPtr),
    GenSeqMap(ApproxHashMap<T, String>),
}
impl<T: ApproxHashMapKey> RhaiNameFn<T> {
    /// Returns the name for an element in the orbit. If the name strategy is a
    /// Rhai function, then only `motor` is passed as an argument.
    pub fn call(&self, ctx: &Ctx<'_>, motor: &pga::Motor, obj: &T) -> Result<Option<String>> {
        self.call_with_args(ctx, obj, vec![Dynamic::from(motor.clone())])
    }

    /// Returns the name for an element in the orbit. If the name strategy is a
    /// Rhai function, then only `args` is passed as the arguments.
    pub fn call_with_args(
        &self,
        ctx: &Ctx<'_>,
        obj: &T,
        args: Vec<Dynamic>,
    ) -> Result<Option<String>> {
        match self {
            RhaiNameFn::Constant(name) => Ok(Some(name.clone())),
            RhaiNameFn::Function(f) => f.call_within_context(ctx, args).and_then(|ret: Dynamic| {
                if ret.is_unit() {
                    Ok(None)
                } else {
                    Ok(from_rhai::<Option<String>>(ctx, ret)?)
                }
            }),
            RhaiNameFn::GenSeqMap(approx_hash_map) => Ok(approx_hash_map.get(obj).cloned()),
        }
    }
}
