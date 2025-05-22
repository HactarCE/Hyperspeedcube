use std::sync::Arc;

use ecow::eco_format;
use hypermath::VectorRef;
use itertools::Itertools;

use super::{Runtime, Scope};
use crate::{
    ErrorMsg, FnOverload, FnType, FnValue, Index, MapKey, Result, Span, Spanned, Type, Value,
    ValueData, ast,
};

pub struct EvalCtx<'a> {
    pub scope: &'a Arc<Scope>,
    pub runtime: &'a mut Runtime,
    pub caller_span: Span,
}

impl std::ops::Index<Span> for EvalCtx<'_> {
    type Output = str;

    fn index(&self, index: Span) -> &Self::Output {
        &self.runtime[index]
    }
}

impl EvalCtx<'_> {
    fn modify(
        &mut self,
        node: &ast::Node,
        update_fn: Box<dyn '_ + FnOnce(&mut EvalCtx<'_>, Span, Option<Value>) -> Result<Value>>,
    ) -> Result<()> {
        // TODO: avoid accessing old value unless it is actually used
        // TODO: avoid having multiple references active unless actually necessary
        let &(ref contents, span) = node;
        match contents {
            ast::NodeContents::Ident(ident_span) => {
                let name = self.runtime.substr(*ident_span);
                let old_value = self.scope.get(&name);
                let new_value = update_fn(self, span, old_value)?;
                self.scope.set(name, new_value);
                Ok(())
            }
            ast::NodeContents::Access { obj, field } => self.modify(
                obj,
                Box::new(|ctx, obj_span, obj| {
                    let mut obj = obj.ok_or(ErrorMsg::Undefined.at(obj_span))?;
                    let old_value = ctx.field_get(&obj, *field)?.at(span);
                    let new_value = update_fn(ctx, span, Some(old_value))?;
                    ctx.field_set(&mut obj, *field, new_value)?;
                    Ok(obj)
                }),
            ),
            ast::NodeContents::Index { obj, args } => {
                let &(ref args_nodes, args_span) = &**args;
                let args: Vec<Value> = args_nodes.iter().map(|arg| self.eval(arg)).try_collect()?;
                self.modify(
                    obj,
                    Box::new(|ctx, obj_span, obj| {
                        let mut obj = obj.ok_or(ErrorMsg::Undefined.at(obj_span))?;
                        let old_value = ctx.index_get(&obj, args.clone(), args_span)?.at(span);
                        let new_value = update_fn(ctx, span, Some(old_value.clone()))?;
                        ctx.index_set(span, &mut obj, args, new_value)?;
                        Ok(obj)
                    }),
                )
            }
            node_contents => Err(ErrorMsg::CannotAssignToExpr {
                kind: node_contents.kind_str(),
            }
            .at(span)),
        }
    }

    fn field_get(&mut self, obj: &Value, field: Span) -> Result<ValueData> {
        let field_name = &self[field];
        match &obj.data {
            // `.x`, `.y`, `.z`, etc.
            ValueData::Vec(v) | ValueData::EuclidPoint(hypermath::Point(v))
                if field_name.len() == 1 =>
            {
                field_name
                    .chars()
                    .next()
                    .and_then(hypermath::axis_from_char)
                    .map(|i| ValueData::Num(v.get(i)))
            }

            ValueData::Str(s) => match field_name {
                "len" => Some(ValueData::Num(s.len() as f64)),
                _ => None,
            },
            ValueData::List(l) => match field_name {
                "len" => Some(ValueData::Num(l.len() as f64)),
                _ => None,
            },
            ValueData::Map(m) => match m.get(field_name) {
                Some(v) => Some(v.data.clone()),
                None => Some(ValueData::Null),
            },
            ValueData::Vec(v) => match field_name {
                "unit" => Some(ValueData::Vec(
                    v.normalize()
                        .ok_or(ErrorMsg::NormalizeZeroVector.at(obj.span))?,
                )),
                "mag2" => Some(ValueData::Num(v.mag2())),
                "mag" => Some(ValueData::Num(v.mag())),
                _ => None,
            },
            _ => None,
        }
        .ok_or(ErrorMsg::NoField { obj: obj.span }.at(field))
    }
    fn field_set(&mut self, obj: &mut Value, field: Span, new_value: Value) -> Result<()> {
        match &mut obj.data {
            ValueData::Map(map) => {
                let map = Arc::make_mut(map);
                if new_value.is_null() {
                    map.swap_remove(&self.runtime[field]);
                } else {
                    map.insert(MapKey::Substr(self.runtime.substr(field)), new_value);
                }
                return Ok(());
            }
            ValueData::Vec(v) | ValueData::EuclidPoint(hypermath::Point(v)) => {
                let field_name = &self[field];
                if field_name.len() == 1 {
                    if let Some(i) = field_name
                        .chars()
                        .next()
                        .and_then(hypermath::axis_from_char)
                    {
                        v[i] = new_value.as_num()?;
                        return Ok(());
                    }
                }
            }
            _ => (),
        }
        Err(ErrorMsg::CannotSetField { obj: obj.span }.at(field))
    }
    fn index_get(&mut self, obj: &Value, index: Vec<Value>, index_span: Span) -> Result<ValueData> {
        let index_value = index.iter().exactly_one().map_err(|_| {
            ErrorMsg::WrongNumberOfIndices {
                obj_span: obj.span,
                count: index.len(),
                min: 1,
                max: 1,
            }
            .at(index_span)
        })?;
        match &obj.data {
            ValueData::Str(s) => {
                let index = index_value.as_index()?;
                let opt_char = match index {
                    Index::Front(i) => s.chars().nth(i),
                    Index::Back(i) => s.chars().nth_back(i),
                };
                match opt_char {
                    Some(c) => Ok(ValueData::from(c)),
                    None => Err(index
                        .out_of_bounds_pos_neg_err(s.chars().count())
                        .at(index_value.span)),
                }
            }
            ValueData::List(list) => {
                let index = index_value.as_index()?;
                let opt_value = match index {
                    Index::Front(i) => list.get(i),
                    Index::Back(i) => list.iter().nth_back(i), // optimized
                };
                match opt_value {
                    Some(v) => Ok(v.data.clone()),
                    None => Err(index
                        .out_of_bounds_pos_neg_err(list.len())
                        .at(index_value.span)),
                }
            }
            ValueData::Map(map) => match &index_value.data {
                ValueData::Str(s) => match map.get(s.as_str()) {
                    Some(v) => Ok(v.data.clone()),
                    None => Ok(ValueData::Null),
                },
                _ => Err(index_value.type_error(Type::Str)),
            },
            ValueData::Vec(vec) | ValueData::EuclidPoint(hypermath::Point(vec)) => {
                Ok(ValueData::Num(vec.get(index_value.as_u8()?)))
            }
            _ => Err(ErrorMsg::CannotIndex(obj.ty()).at(obj.span)),
        }
    }
    fn index_set(
        &mut self,
        span: Span,
        obj: &mut Value,
        index: Vec<Value>,
        new_value: Value,
    ) -> Result<()> {
        let index_value = index.iter().exactly_one().map_err(|_| {
            ErrorMsg::WrongNumberOfIndices {
                obj_span: obj.span,
                count: index.len(),
                min: 1,
                max: 1,
            }
            .at(span)
        })?;
        match &mut obj.data {
            ValueData::Str(_) => Err(ErrorMsg::CannotAssignToExpr {
                kind: "string indexing expression",
            }
            .at(span)),
            ValueData::List(list) => {
                let index = index_value.as_index()?;
                let opt_value = match index {
                    Index::Front(i) => Arc::make_mut(list).get_mut(i),
                    Index::Back(i) => Arc::make_mut(list).iter_mut().nth_back(i), // optimized
                };
                match opt_value {
                    Some(v) => Ok(*v = new_value),
                    None => Err(index
                        .out_of_bounds_pos_neg_err(list.len())
                        .at(index_value.span)),
                }
            }
            ValueData::Map(map) => match &index_value.data {
                ValueData::Str(s) => {
                    let map = Arc::make_mut(map);
                    if new_value.is_null() {
                        map.swap_remove(s.as_str());
                    } else {
                        map.insert(MapKey::String(s.clone()), new_value);
                    }
                    Ok(())
                }
                _ => Err(index_value.type_error(Type::Str)),
            },
            ValueData::Vec(vec) | ValueData::EuclidPoint(hypermath::Point(vec)) => {
                vec.resize_and_set(index_value.as_u8()?, new_value.as_num()?);
                Ok(())
            }
            _ => Err(ErrorMsg::CannotIndex(obj.ty()).at(obj.span)),
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
                let new_value = self.eval(value)?;

                // Check type.
                if let Some(ty_node) = ty {
                    new_value.typecheck(self.eval_ty(ty_node)?)?;
                }

                let assign_op_str = self[*assign_symbol]
                    .strip_suffix("=")
                    .ok_or(ErrorMsg::Internal("invalid operator").at(*assign_symbol))?;
                let assign_op = match assign_op_str {
                    "" => None,
                    _ => Some(
                        self.scope
                            .get(assign_op_str)
                            .ok_or(ErrorMsg::UnsupportedOperator.at(*assign_symbol))?,
                    ),
                };

                self.modify(
                    var,
                    Box::new(|ctx, old_value_span, old_value| match &assign_op {
                        Some(op_fn) => {
                            let old_value =
                                old_value.ok_or(ErrorMsg::Undefined.at(old_value_span))?;
                            let args = vec![old_value, new_value];
                            op_fn.as_func()?.call(span, *assign_symbol, ctx, args)
                        }
                        None => Ok(new_value),
                    }),
                )?;

                Ok(null)
            }
            ast::NodeContents::Export(inner) => {
                // TODO: exports
                Ok(self.eval(inner)?.data)
            }
            ast::NodeContents::FnDef { name, contents } => {
                let new_overload = self.eval_fn_contents(span, &**contents)?;
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
            ast::NodeContents::Block(items) => Ok(self.exec_in_child_scope(|ctx| {
                if items.len() == 1 {
                    Ok(ctx.eval(&items[0])?.data)
                } else {
                    for item in items {
                        ctx.eval(item)?;
                    }
                    Ok(null)
                }
            })?),
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
            } => {
                let &(ref loop_var_idents, vars_span) = &**loop_vars;
                let iter_value = self.eval(iterator)?;
                match &iter_value.data {
                    ValueData::Str(s) => {
                        self.exec_for_loop_indexed(loop_vars, iter_value.span, s.chars(), body)?;
                        Ok(null)
                    }
                    ValueData::List(list) => {
                        let elems = list.iter().map(|e| e.data.clone());
                        self.exec_for_loop_indexed(loop_vars, iter_value.span, elems, body)?;
                        Ok(null)
                    }
                    ValueData::Map(map) => {
                        let &[key_var, value_var] = loop_var_idents.as_slice() else {
                            return Err(ErrorMsg::WrongNumberOfLoopVars {
                                iter_span: iter_value.span,
                                count: loop_var_idents.len(),
                                min: 2,
                                max: 2,
                            }
                            .at(vars_span));
                        };
                        let iterations = map.iter().map(|(k, v)| {
                            [(key_var, k.as_ref().into()), (value_var, v.data.clone())]
                        });
                        self.exec_for_loop(iterations, body)?;
                        Ok(null)
                    }
                    ValueData::Vec(vec) | ValueData::EuclidPoint(hypermath::Point(vec)) => {
                        self.exec_for_loop_indexed(loop_vars, iter_value.span, vec.iter(), body)?;
                        Ok(null)
                    }
                    _ => return Err(ErrorMsg::CannotIterate(iter_value.ty()).at(iter_value.span)),
                }
            }
            ast::NodeContents::WhileLoop { condition, body } => {
                Ok(self.exec_in_child_scope(|ctx| {
                    while ctx.eval(&condition)?.as_bool()? {
                        match ctx.eval(body) {
                            Ok(_) => (),
                            Err(e) => match &e.msg {
                                ErrorMsg::Break => break,
                                ErrorMsg::Continue => continue,
                                _ => return Err(e),
                            },
                        }
                    }
                    Ok(null)
                })?)
            }
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
                .get(&self[*ident_span])
                .ok_or(ErrorMsg::Undefined.at(*ident_span))?
                .data),
            ast::NodeContents::Op { op, args } => {
                let f = self
                    .scope
                    .get(&self[*op])
                    .ok_or(ErrorMsg::UnsupportedOperator.at(*op))?;
                let args = args.iter().map(|arg| self.eval(arg)).try_collect()?;
                Ok(f.as_func()?.call(span, *op, self, args)?.data)
            }
            ast::NodeContents::FnCall { func, args } => {
                let mut arg_values = Vec::with_capacity(args.len() + 1);
                let func_value =
                    if let &(ast::NodeContents::Access { ref obj, field }, obj_method_span) =
                        &**func
                    {
                        let obj = self.eval(&obj)?;
                        // TODO: warn if ambiguous
                        let maybe_method = self.scope.get(&self[field]).filter(|method_value| {
                            method_value
                                .as_func()
                                .is_ok_and(|f| f.can_be_method_of(obj.ty()))
                        });
                        match maybe_method {
                            Some(m) => {
                                arg_values.push(obj);
                                m
                            }
                            None => self.field_get(&obj, field)?.at(obj_method_span),
                        }
                    } else {
                        self.eval(func)?
                    };
                let f = func_value.as_func()?;
                for arg in args {
                    arg_values.push(self.eval(arg)?);
                }
                Ok(f.call(span, func_value.span, self, arg_values)?.data)
            }
            ast::NodeContents::Paren(expr) => Ok(self.eval(expr)?.data),
            ast::NodeContents::Access { obj, field } => {
                let obj_value = self.eval(obj)?;
                Ok(self.field_get(&obj_value, *field)?)
            }
            ast::NodeContents::Index { obj, args } => {
                let &(ref args_nodes, args_span) = &**args;
                let obj_value = self.eval(obj)?;
                let arg_values = args_nodes.iter().map(|arg| self.eval(arg)).try_collect()?;
                Ok(self.index_get(&obj_value, arg_values, args_span)?)
            }
            ast::NodeContents::Fn(contents) => Ok(ValueData::Fn(Arc::new(FnValue {
                name: None,
                overloads: vec![self.eval_fn_contents(span, contents)?],
            }))),
            ast::NodeContents::NullLiteral => Ok(null),
            ast::NodeContents::BoolLiteral(b) => Ok(ValueData::Bool(*b)),
            ast::NodeContents::NumberLiteral(n) => Ok(ValueData::Num(*n)),
            ast::NodeContents::StringLiteral(string_segments) => Ok({
                let mut s = String::new();
                for segment in string_segments {
                    match segment {
                        ast::StringSegment::Literal(simple_span) => {
                            s.push_str(&self[*simple_span]);
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
                    .filter_ok(|(_k, v)| !v.is_null())
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
            ast::NodeContents::Ident(ident_span) => match &self[*ident_span] {
                "Any" => Ok(Type::Any),
                "Null" => Ok(Type::Null),
                "Bool" => Ok(Type::Bool),
                "Num" => Ok(Type::Num),
                "Str" => Ok(Type::Str),
                "List" => Ok(Type::List(Default::default())),
                "Map" => Ok(Type::Map(Default::default())),
                "Fn" => Ok(Type::Fn(Default::default())),

                "Vec" => Ok(Type::Vec),

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
                let &(ref obj_node, obj_span) = &**obj;
                let &(ref args_nodes, args_span) = &**args;
                let inner_type_node = args_nodes.iter().exactly_one().map_err(|_| {
                    ErrorMsg::WrongNumberOfIndices {
                        obj_span,
                        count: args_nodes.len(),
                        min: 1,
                        max: 1,
                    }
                    .at(args_span)
                })?;
                let inner_type = self.eval_ty(inner_type_node)?;
                match obj_node {
                    ast::NodeContents::Ident(ident_span) => match &self[*ident_span] {
                        "List" => Ok(Type::List(Box::new(inner_type))),
                        "Map" => Ok(Type::Map(Box::new(inner_type))),
                        "Fn" => Err(ErrorMsg::Unimplemented("specific function types").at(*span)),
                        _ => Err(ErrorMsg::ExpectedCollectionType.at(*ident_span)),
                    },
                    _ => Err(ErrorMsg::ExpectedCollectionType.at(obj_span)),
                }
            }
            _ => Err(ErrorMsg::ExpectedType {
                ast_node_kind: contents.kind_str(),
            }
            .at(*span)),
        }
    }

    fn eval_fn_contents(&mut self, span: Span, contents: &ast::FnContents) -> Result<FnOverload> {
        let param_names = contents.params.iter().map(|param| param.name).collect_vec();
        let param_types = contents
            .params
            .iter()
            .map(|param| self.eval_opt_ty(param.ty.as_deref()))
            .collect::<Result<Vec<Type>>>()?;
        let return_type = self.eval_opt_ty(contents.return_type.as_deref())?;
        let fn_body = Arc::clone(&contents.body);
        Ok(FnOverload {
            ty: FnType {
                params: Some(param_types),
                ret: return_type.clone(),
            },
            call: Arc::new(move |ctx, mut args| {
                for (i, &param_span) in param_names.iter().enumerate() {
                    let param_name = ctx.runtime.substr(param_span);
                    let arg_value = args.get_mut(i).ok_or(
                        ErrorMsg::Internal("missing this function argument in call").at(param_span),
                    )?;
                    ctx.scope.set(param_name, std::mem::take(arg_value));
                }
                let return_value = ctx.eval(&fn_body)?;
                return_value.typecheck(&return_type)?;
                Ok(return_value)
            }),
            debug_info: span.into(),
        })
    }

    fn set_var(&self, span: Span, value: impl Into<ValueData>) {
        self.scope
            .set(self.runtime.substr(span), value.into().at(span));
    }

    fn exec_for_loop_indexed<T: Into<ValueData>>(
        &mut self,
        loop_vars: &Spanned<Vec<Span>>,
        iter_value_span: Span,
        elems: impl IntoIterator<Item = T>,
        body: &ast::Node,
    ) -> Result<()> {
        let elems = elems.into_iter().map(|e| e.into());
        let &(ref loop_var_idents, loop_vars_span) = loop_vars;
        match loop_var_idents.as_slice() {
            &[elem_var] => self.exec_for_loop(elems.into_iter().map(|e| [(elem_var, e)]), body),
            &[index_var, elem_var] => self.exec_for_loop(
                elems
                    .into_iter()
                    .enumerate()
                    .map(|(i, e)| [(index_var, i.into()), (elem_var, e)]),
                body,
            ),
            _ => Err(ErrorMsg::WrongNumberOfLoopVars {
                iter_span: iter_value_span,
                count: loop_var_idents.len(),
                min: 1,
                max: 2,
            }
            .at(loop_vars_span)),
        }
    }

    fn exec_for_loop<I: IntoIterator<Item = (Span, ValueData)>>(
        &mut self,
        iterations: impl IntoIterator<Item = I>,
        body: &ast::Node,
    ) -> Result<()> {
        self.exec_in_child_scope(|ctx| {
            for iteration in iterations {
                for (loop_var, value) in iteration {
                    ctx.set_var(loop_var, value);
                }
                match ctx.eval(body) {
                    Ok(_) => (),
                    Err(e) => match &e.msg {
                        ErrorMsg::Break => break,
                        ErrorMsg::Continue => continue,
                        _ => return Err(e),
                    },
                }
            }
            Ok(())
        })
    }

    fn exec_in_child_scope<R>(&mut self, f: impl for<'a> FnOnce(&mut EvalCtx<'a>) -> R) -> R {
        f(&mut EvalCtx {
            scope: &Scope::new_block(Arc::clone(self.scope)),
            runtime: self.runtime,
            caller_span: self.caller_span,
        })
    }
}
