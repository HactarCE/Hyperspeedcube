use std::{ops::Index, sync::Arc};

use ecow::eco_format;
use itertools::Itertools;

use super::{BUILTIN_SCOPE, EMPTY_SCOPE, Runtime, Scope};
use crate::{
    Error, ErrorMsg, FnOverload, FnType, MapKey, Result, Span, Type, Value, ValueData, ast,
};

pub struct EvalCtx<'a> {
    pub scope: &'a Arc<Scope>,
    pub runtime: &'a mut Runtime,
}

impl Index<Span> for EvalCtx<'_> {
    type Output = str;

    fn index(&self, index: Span) -> &Self::Output {
        &self.runtime[index]
    }
}

impl EvalCtx<'_> {
    pub fn atomic_modify(
        &mut self,
        var: &ast::Node,
        modify: Box<dyn '_ + FnOnce(&mut Runtime, &mut Value) -> Result<()>>,
        if_undefined: Box<dyn '_ + FnOnce() -> Result<Option<Value>>>,
    ) -> Result<()> {
        let var_span = var.1;
        match &var.0 {
            ast::NodeContents::Ident(ident_span) => {
                self.scope.atomic_modify(
                    self.runtime.substr(*ident_span),
                    |val| modify(self.runtime, val),
                    || if_undefined(),
                )?;
                Ok(())
            }
            ast::NodeContents::Access { obj, field } => self.atomic_modify(
                &obj,
                Box::new(|this, obj_value| todo!("modify field of obj_value")),
                Box::new(move || Err(ErrorMsg::Undefined.at(var_span))),
            ),
            ast::NodeContents::Index { obj, args } => self.atomic_modify(
                &obj,
                Box::new(|this, obj_value| todo!("modify indexed value of obj_value")),
                Box::new(move || Err(ErrorMsg::Undefined.at(var_span))),
            ),
            node_contents => Err(ErrorMsg::CannotAssignToExpr {
                kind: node_contents.kind_str(),
            }
            .at(var_span)),
        }
    }

    pub fn eval(&mut self, node: &ast::Node) -> Result<Value> {
        let &(ref contents, span) = node;
        let null = ValueData::Null;
        match contents {
            ast::NodeContents::Assign {
                var,
                ty,
                assign_symbol,
                value,
            } => {
                let var_span = var.1;
                let new_value = self.eval(value)?;

                // Check type.
                if let Some(ty_node) = ty {
                    new_value.typecheck(self.eval_ty(ty_node)?)?;
                }

                let assign_symbol_substr = self.runtime.substr(*assign_symbol);

                self.atomic_modify(
                    var,
                    Box::new(|runtime, val| {
                        let op_str = assign_symbol_substr
                            .strip_suffix("=")
                            .ok_or(ErrorMsg::Internal("invalid operator").at(*assign_symbol))?;
                        match op_str {
                            "" => Ok(*val = new_value),
                            _ => Ok(*val = {
                                let op_fn = BUILTIN_SCOPE
                                    .get(&op_str)
                                    .ok_or(ErrorMsg::UnsupportedOperator.at(*assign_symbol))?;
                                op_fn.as_func()?.call(
                                    span,
                                    &mut EvalCtx {
                                        scope: &EMPTY_SCOPE,
                                        runtime,
                                    },
                                    vec![val.clone(), new_value],
                                )?
                            }),
                        }
                    }),
                    Box::new(|| match assign_symbol_substr.as_str() {
                        "=" => Ok(Some(Value::NULL)),
                        _ => Err(ErrorMsg::Undefined.at(var_span)),
                    }),
                )?;

                Ok(null)
            }
            ast::NodeContents::Export(inner) => {
                // TODO: exports
                Ok(self.eval(inner)?.data)
            }
            ast::NodeContents::FnDef { name, contents } => {
                let param_names = contents.params.iter().map(|param| param.name).collect_vec();
                let param_types = contents
                    .params
                    .iter()
                    .map(|param| self.eval_opt_ty(param.ty.as_deref()))
                    .collect::<Result<Vec<Type>>>()?;
                let return_type = self.eval_opt_ty(contents.return_type.as_deref())?;
                let fn_body = Arc::clone(&contents.body);
                let new_overload = FnOverload {
                    ty: FnType {
                        params: Some(param_types),
                        ret: return_type.clone(),
                    },
                    call: Arc::new(move |ctx, mut args| {
                        for (i, &param_span) in param_names.iter().enumerate() {
                            let param_name = ctx.runtime.substr(param_span);
                            let arg_value = args.get_mut(i).ok_or(
                                ErrorMsg::Internal("missing this function argument in call")
                                    .at(span),
                            )?;
                            ctx.scope.set(param_name, std::mem::take(arg_value));
                        }
                        let return_value = ctx.eval(&fn_body)?;
                        return_value.typecheck(&return_type)?;
                        Ok(return_value)
                    }),
                    debug_info: span.into(),
                };
                let fn_name = self.runtime.substr(*name);
                self.scope.register_func(span, fn_name, new_overload)?;
                Ok(null)
            }
            ast::NodeContents::ImportAllFrom(import_path) => todo!(),
            ast::NodeContents::ImportFrom(simple_spans, import_path) => todo!(),
            ast::NodeContents::ImportAs(import_path, simple_span) => todo!(),
            ast::NodeContents::Import(simple_span) => todo!(),
            ast::NodeContents::UseAllFrom(_) => todo!(),
            ast::NodeContents::UseFrom(simple_spans, _) => todo!(),
            ast::NodeContents::Block(items) => {
                if items.len() == 1 {
                    Ok(self.eval(&items[0])?.data)
                } else {
                    for item in items {
                        self.eval(item)?;
                    }
                    Ok(null)
                }
            }
            ast::NodeContents::IfElse {
                if_cases,
                else_case,
            } => {
                let mut if_cases = if_cases.iter();
                Ok(loop {
                    match if_cases.next() {
                        Some((cond, body)) => {
                            if self.eval(cond)?.as_bool()? {
                                break self.eval(body)?.data;
                            }
                        }
                        None => match else_case {
                            Some(body) => break self.eval(body)?.data,
                            None => break null,
                        },
                    }
                })
            }
            ast::NodeContents::ForLoop {
                loop_vars,
                iterator,
                body,
            } => todo!(),
            ast::NodeContents::WhileLoop { condition, body } => todo!(),
            ast::NodeContents::Continue => Err(ErrorMsg::Continue),
            ast::NodeContents::Break => Err(ErrorMsg::Break),
            ast::NodeContents::Return(ret_expr) => {
                Err(ErrorMsg::Return(Box::new(match ret_expr {
                    Some(expr) => self.eval(expr)?,
                    None => null.at(span),
                })))
            }
            ast::NodeContents::Ident(ident_span) => Ok(self
                .scope
                .get(&self.runtime[*ident_span])
                .ok_or(ErrorMsg::Undefined.at(*ident_span))?
                .data),
            ast::NodeContents::Op { op, args } => {
                let f = self
                    .scope
                    .get(&self.runtime[*op])
                    .ok_or(ErrorMsg::UnsupportedOperator.at(*op))?;
                let args = args.iter().map(|arg| self.eval(arg)).try_collect()?;
                Ok(f.as_func()?.call(span, self, args)?.data)
            }
            ast::NodeContents::FnCall { func, args } => {
                let f = self.eval(func)?;
                let args = args.iter().map(|arg| self.eval(arg)).try_collect()?;
                Ok(f.as_func()?.call(span, self, args)?.data)
            }
            ast::NodeContents::Paren(expr) => Ok(self.eval(expr)?.data),
            ast::NodeContents::Access { obj, field } => todo!(),
            ast::NodeContents::Index { obj, args } => todo!(),
            ast::NodeContents::Fn(fn_contents) => todo!(),
            ast::NodeContents::NullLiteral => Ok(null),
            ast::NodeContents::BoolLiteral(b) => Ok(ValueData::Bool(*b)),
            ast::NodeContents::NumberLiteral(n) => Ok(ValueData::Num(*n)),
            ast::NodeContents::StringLiteral(string_segments) => Ok({
                let mut s = String::new();
                for segment in string_segments {
                    match segment {
                        ast::StringSegment::Literal(simple_span) => {
                            s.push_str(&self.runtime[*simple_span]);
                        }
                        ast::StringSegment::Char(c) => {
                            s.push(*c);
                        }
                        ast::StringSegment::Interpolation(expr) => {
                            s.push_str(&self.eval(expr)?.to_string());
                        }
                    }
                }
                ValueData::Str(s.into())
            }),
            ast::NodeContents::ListLiteral(items) => Ok(ValueData::List(Arc::new(
                items.iter().map(|node| self.eval(node)).try_collect()?,
            ))),
            ast::NodeContents::MapLiteral(items) => Ok(ValueData::Map(Arc::new(
                items
                    .iter()
                    .map(|(key_node, value_node)| {
                        let (key_contents, key_span) = key_node;
                        let key = match key_contents {
                            ast::NodeContents::Ident(ident_span) => {
                                MapKey::Substr(self.runtime.substr(*ident_span))
                            }
                            ast::NodeContents::StringLiteral(_) => {
                                MapKey::String(eco_format!("{}", self.eval(key_node)?))
                            }
                            _ => return Err(ErrorMsg::ExpectedMapKey.at(*key_span)),
                        };
                        let value = self.eval(value_node)?;
                        Ok((key, value))
                    })
                    .try_collect()?,
            ))),

            ast::NodeContents::Error => Err(ErrorMsg::AstErrorNode),
        }
        .map(|val| val.at(span))
        .map_err(|err| err.at(span))
    }

    pub fn eval_opt_ty(&self, opt_node: Option<&ast::Node>) -> Result<Type> {
        match opt_node {
            Some(node) => self.eval_ty(node),
            None => Ok(Type::Any),
        }
    }
    pub fn eval_ty(&self, node: &ast::Node) -> Result<Type> {
        let (contents, span) = node;
        match contents {
            ast::NodeContents::Ident(ident_span) => match &self.runtime[*ident_span] {
                "Any" => Ok(Type::Any),
                "Null" => Ok(Type::Null),
                "Bool" => Ok(Type::Bool),
                "Num" => Ok(Type::Num),
                "Str" => Ok(Type::Str),
                "List" => Ok(Type::List(Default::default())),
                "Map" => Ok(Type::Map(Default::default())),
                "Fn" => Ok(Type::Fn(Default::default())),

                "Vector" => Ok(Type::Vector),

                "EuclidPoint" => Ok(Type::EuclidPoint),
                "EuclidTransform" => Ok(Type::EuclidTransform),
                "EuclidPlane" => Ok(Type::EuclidPlane),
                "EuclidRegion" => Ok(Type::EuclidRegion),

                "Cga2dBlade1" => Ok(Type::Cga2dBlade1),
                "Cga2dBlade2" => Ok(Type::Cga2dBlade2),
                "Cga2dBlade3" => Ok(Type::Cga2dBlade3),
                "Cga2dAntiscalar" => Ok(Type::Cga2dAntiscalar),
                "Cga2dRegion" => Ok(Type::Cga2dRegion),

                "Color" => Ok(Type::Color),
                "Axis" => Ok(Type::Axis),
                "Twist" => Ok(Type::Twist),

                "AxisSystem" => Ok(Type::AxisSystem),
                "TwistSystem" => Ok(Type::TwistSystem),
                "Puzzle" => Ok(Type::Puzzle),

                _ => Err(ErrorMsg::UnknownType.at(*span)),
            },
            ast::NodeContents::Index { obj, args } => {
                let (inner_contents, inner_span) = &**obj;
                let inner_type_node = args.iter().exactly_one().map_err(|_| {
                    ErrorMsg::WrongNumberOfIndices {
                        count: args.len(),
                        min: 1,
                        max: 1,
                    }
                    .at(*span)
                })?;
                let inner_type = self.eval_ty(inner_type_node)?;
                match inner_contents {
                    ast::NodeContents::Ident(ident_span) => match &self.runtime[*ident_span] {
                        "List" => Ok(Type::List(Box::new(inner_type))),
                        "Map" => Ok(Type::Map(Box::new(inner_type))),
                        "Fn" => Err(ErrorMsg::Unimplemented("specific function types").at(*span)),
                        _ => Err(ErrorMsg::ExpectedCollectionType.at(*span)),
                    },
                    _ => Err(ErrorMsg::ExpectedCollectionType.at(*span)),
                }
            }
            _ => Err(ErrorMsg::ExpectedType {
                ast_node_kind: contents.kind_str(),
            }
            .at(*span)),
        }
    }
}
