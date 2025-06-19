use std::collections::HashMap;
use std::ops::Index;
use std::sync::Arc;

use arcstr::Substr;
use hypermath::{VectorRef, approx_eq, vector};
use itertools::Itertools;

use super::{Runtime, Scope};
use crate::{
    Error, FnOverload, FnType, FnValue, Key, List, LoopControlFlow, Map, Result, Span, Spanned,
    Str, Type, Value, ValueData, Warning, ast,
};

/// Evaluation context.
pub struct EvalCtx<'a> {
    /// Innermost scope.
    pub scope: &'a Arc<Scope>,
    /// Language runtime.
    pub runtime: &'a mut Runtime,
    /// Span of the most recent caller.
    ///
    /// This is used as the span when an error occurs in a built-in function.
    pub caller_span: Span,
    /// Exports from the current function/file.
    pub exports: &'a mut Option<Map>,
}

impl EvalCtx<'_> {
    fn assign(&mut self, node: &ast::Node, new_value: Value) -> Result<()> {
        match &node.0 {
            ast::NodeContents::Ident(_)
            | ast::NodeContents::ListLiteral(_)
            | ast::NodeContents::MapLiteral(_) => self.assign_destructure(node, new_value),
            _ => self.assign_path(node, new_value),
        }
    }

    fn assign_path(&mut self, node: &ast::Node, new_value: Value) -> Result<()> {
        let &(ref contents, span) = node;
        match contents {
            ast::NodeContents::Ident(ident_span) => {
                self.set_var(*ident_span, new_value.data);
                Ok(())
            }
            ast::NodeContents::Access { obj, field } => self.modify(
                obj,
                Box::new(|ctx, obj_span, obj| {
                    let mut obj = obj.ok_or(Error::Undefined.at(obj_span))?;
                    ctx.field_set(&mut obj, *field, new_value)?;
                    Ok(obj)
                }),
            ),
            ast::NodeContents::Index { obj, args } => {
                let &(ref args_nodes, _args_span) = &**args;
                let args: Vec<Value> = args_nodes.iter().map(|arg| self.eval(arg)).try_collect()?;
                self.modify(
                    obj,
                    Box::new(|ctx, obj_span, obj| {
                        let mut obj = obj.ok_or(Error::Undefined.at(obj_span))?;
                        ctx.index_set(span, &mut obj, args, new_value)?;
                        Ok(obj)
                    }),
                )
            }

            ast::NodeContents::SpecialIdent(special_var) => {
                Err(Error::CannotAssignToSpecialVar(*special_var).at(span))
            }
            node_contents => Err(Error::CannotAssignToExpr {
                kind: node_contents.kind_str(),
            }
            .at(span)),
        }
    }

    fn assign_destructure(&mut self, node: &ast::Node, new_value: Value) -> Result<()> {
        let new_value_span = new_value.span;
        let &(ref contents, pattern_span) = node;
        match contents {
            ast::NodeContents::Ident(ident_span) => {
                self.set_var(*ident_span, new_value.data);
                Ok(())
            }
            ast::NodeContents::ListLiteral(items) => {
                let mut new_values_iter = new_value.unwrap_or_clone_arc::<Vec<_>>()?.into_iter();

                let mut items_iter = items.iter();
                let rest = items.last().and_then(|(item, _)| item.as_list_splat(self));
                if rest.is_some() {
                    items_iter.next_back();
                }

                let pattern_len = items_iter.len();
                let value_len = new_values_iter.len();
                let error = || {
                    Error::ListLengthMismatchInPattern {
                        pattern_span,
                        pattern_len,
                        allow_excess: rest.is_some(),
                        value_len,
                    }
                    .at(new_value_span)
                };

                for item in items_iter {
                    if item.0.as_list_splat(self).is_some() {
                        return Err(Error::SplatBeforeEndInPattern { pattern_span }.at(item.1));
                    }
                    self.assign_destructure(item, new_values_iter.next().ok_or_else(error)?)?;
                }
                if let Some(rest) = rest {
                    let remaining_values = Arc::new(new_values_iter.collect_vec());
                    let &(_, rest_span) = rest;
                    self.assign_destructure(rest, ValueData::List(remaining_values).at(rest_span))?;
                } else if new_values_iter.next().is_some() {
                    return Err(error());
                }

                Ok(())
            }
            ast::NodeContents::MapLiteral(entries) => {
                let mut new_values_map = new_value.unwrap_or_clone_arc::<Map>()?;

                let mut entries_iter = entries.iter();
                let rest = match entries.last() {
                    Some(ast::MapEntry::Splat { span: _, values }) => Some(values),
                    _ => None,
                };
                if rest.is_some() {
                    entries_iter.next_back();
                }

                let mut seen_keys = HashMap::new();

                for entry in entries_iter {
                    match entry {
                        ast::MapEntry::KeyValue {
                            key: key_node @ (_, key_span),
                            ty,
                            value: value_node,
                        } => {
                            let key = self.eval_map_key(key_node)?;
                            if let Some(previous_span) = seen_keys.insert(key.clone(), *key_span) {
                                return Err(Error::DuplicateMapKey { previous_span }.at(*key_span));
                            }
                            let new_value = new_values_map
                                .insert(key, ValueData::Null.at(*key_span))
                                .unwrap_or(ValueData::Null.at(*key_span));
                            new_value.typecheck(self.eval_opt_ty(ty.as_deref())?)?;
                            self.assign_destructure(
                                value_node.as_deref().unwrap_or(key_node),
                                new_value,
                            )?;
                        }
                        ast::MapEntry::Splat { span, .. } => {
                            return Err(Error::SplatBeforeEndInPattern { pattern_span }.at(*span));
                        }
                    }
                }

                new_values_map.retain(|_k, v| !v.is_null());
                if let Some(rest) = rest {
                    self.assign_destructure(
                        rest,
                        ValueData::Map(Arc::new(new_values_map)).at(rest.1),
                    )?;
                } else if !new_values_map.is_empty() {
                    return Err(Error::UnusedMapKeysInPattern {
                        pattern_span,
                        keys: new_values_map
                            .into_iter()
                            .map(|(k, v)| (k, v.span))
                            .collect(),
                    }
                    .at(new_value_span));
                }

                Ok(())
            }

            ast::NodeContents::SpecialIdent(special_var) => {
                Err(Error::CannotAssignToSpecialVar(*special_var).at(pattern_span))
            }
            node_contents => Err(Error::CannotAssignToExpr {
                kind: node_contents.kind_str(),
            }
            .at(pattern_span)),
        }
    }

    fn modify(
        &mut self,
        node: &ast::Node,
        update_fn: Box<dyn '_ + FnOnce(&mut EvalCtx<'_>, Span, Option<Value>) -> Result<Value>>,
    ) -> Result<()> {
        // TODO: avoid having multiple references active unless actually necessary
        let &(ref contents, span) = node;
        match contents {
            ast::NodeContents::Ident(ident_span) => {
                let name = self.substr(*ident_span);
                let old_value = self.scope.get(&name);
                let new_value = update_fn(self, span, old_value)?;
                self.scope.set(name, new_value);
                Ok(())
            }
            ast::NodeContents::Access { obj, field } => self.modify(
                obj,
                Box::new(|ctx, obj_span, obj| {
                    let mut obj = obj.ok_or(Error::Undefined.at(obj_span))?;
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
                        let mut obj = obj.ok_or(Error::Undefined.at(obj_span))?;
                        let old_value = ctx.index_get(&obj, args.clone(), args_span)?.at(span);
                        let new_value = update_fn(ctx, span, Some(old_value.clone()))?;
                        ctx.index_set(span, &mut obj, args, new_value)?;
                        Ok(obj)
                    }),
                )
            }

            ast::NodeContents::SpecialIdent(special_var) => {
                Err(Error::CannotAssignToSpecialVar(*special_var).at(span))
            }
            node_contents => Err(Error::CannotAssignToExpr {
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
                "is_empty" => Some(ValueData::Bool(s.is_empty())),
                "len" => Some(ValueData::Num(s.len() as f64)),
                _ => None,
            },
            ValueData::List(l) => match field_name {
                "is_empty" => Some(ValueData::Bool(l.is_empty())),
                "len" => Some(ValueData::Num(l.len() as f64)),
                _ => None,
            },
            ValueData::Map(m) => match m.get(field_name) {
                Some(v) => Some(v.data.clone()),
                None => Some(ValueData::Null),
            },
            ValueData::Vec(v) => match field_name {
                "angle" => {
                    if v.iter_nonzero().any(|(i, _)| i >= 2) {
                        let msg = "`angle` is undefined beyond 2D";
                        return Err(Error::bad_arg(obj.clone(), Some(msg)).at(obj.span));
                    } else if approx_eq(v, &vector![]) {
                        let msg = "`angle` is undefined for zero vector";
                        return Err(Error::bad_arg(obj.clone(), Some(msg)).at(obj.span));
                    }
                    Some(ValueData::Num(v.get(1).atan2(v.get(0))))
                }
                "unit" => Some(ValueData::Vec(
                    v.normalize().ok_or(
                        Error::bad_arg(obj.clone(), Some("cannot normalize the zero vector"))
                            .at(obj.span),
                    )?,
                )),
                "mag2" => Some(ValueData::Num(v.mag2())),
                "mag" => Some(ValueData::Num(v.mag())),
                _ => None,
            },
            ValueData::EuclidPlane(p) => match field_name {
                "flip" => Some(ValueData::EuclidPlane(Box::new(p.flip()))),
                "normal" => Some(ValueData::Vec(p.normal().clone())),
                "distance" => Some(ValueData::Num(p.distance())),
                _ => None,
            },
            _ => None,
        }
        .ok_or(Error::NoField((obj.ty(), obj.span)).at(field))
    }
    fn field_set(&mut self, obj: &mut Value, field: Span, new_value: Value) -> Result<()> {
        match &mut obj.data {
            ValueData::Map(map) => {
                let map = Arc::make_mut(map);
                if new_value.is_null() {
                    map.swap_remove(&self[field]);
                } else {
                    map.insert(self.substr(field), new_value);
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
                        v[i] = new_value.to()?;
                        return Ok(());
                    }
                }
            }
            _ => (),
        }
        Err(Error::CannotSetField((obj.ty(), obj.span)).at(field))
    }
    fn index_get(&mut self, obj: &Value, index: Vec<Value>, index_span: Span) -> Result<ValueData> {
        let index_value = index.iter().exactly_one().map_err(|_| {
            Error::WrongNumberOfIndices {
                obj_span: obj.span,
                count: index.len(),
                min: 1,
                max: 1,
            }
            .at(index_span)
        })?;
        match &obj.data {
            // Index string by character (O(n))
            ValueData::Str(s) => Ok(index_value
                .index_double_ended(s.chars(), || s.chars().count())?
                .into()),
            // Index list by element (O(1))
            ValueData::List(list) => Ok(index_value
                .index_double_ended(list.iter(), || list.len())?
                .data
                .clone()),
            ValueData::Map(map) => match &index_value.data {
                ValueData::Str(s) => match map.get(s.as_str()) {
                    Some(v) => Ok(v.data.clone()),
                    None => Ok(ValueData::Null),
                },
                _ => Err(index_value.type_error(Type::Str)),
            },
            ValueData::Type(ty) => match ty {
                Type::List(None) => {
                    let inner_type = index_value.clone_to()?;
                    Ok(Type::List(Some(Box::new(inner_type))).into())
                }
                Type::NonEmptyList(None) => {
                    let inner_type = index_value.clone_to()?;
                    Ok(Type::NonEmptyList(Some(Box::new(inner_type))).into())
                }
                _ => Err(Error::ExpectedCollectionType {
                    got_type: ty.clone(),
                }
                .at(obj.span)),
            },
            ValueData::Vec(vec) | ValueData::EuclidPoint(hypermath::Point(vec)) => {
                Ok(ValueData::Num(vec.get(index_value.ref_to()?)))
            }
            _ => Err(Error::CannotIndex(obj.ty()).at(obj.span)),
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
            Error::WrongNumberOfIndices {
                obj_span: obj.span,
                count: index.len(),
                min: 1,
                max: 1,
            }
            .at(span)
        })?;
        match &mut obj.data {
            ValueData::Str(_) => Err(Error::CannotAssignToExpr {
                kind: "string indexing expression",
            }
            .at(span)),
            ValueData::List(list) => {
                let len = list.len();
                *index_value.index_double_ended(Arc::make_mut(list).iter_mut(), || len)? =
                    new_value;
                Ok(())
            }
            ValueData::Map(map) => match &index_value.data {
                ValueData::Str(s) => {
                    let map = Arc::make_mut(map);
                    if new_value.is_null() {
                        map.swap_remove(s.as_str());
                    } else {
                        map.insert(s.as_str().into(), new_value);
                    }
                    Ok(())
                }
                _ => Err(index_value.type_error(Type::Str)),
            },
            ValueData::Vec(vec) | ValueData::EuclidPoint(hypermath::Point(vec)) => {
                vec.resize_and_set(index_value.ref_to()?, new_value.ref_to()?);
                Ok(())
            }
            _ => Err(Error::CannotIndex(obj.ty()).at(obj.span)),
        }
    }

    /// Evaluates an AST node to a value.
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
                let assign_op_str = self[*assign_symbol]
                    .strip_suffix("=")
                    .ok_or(Error::Internal("invalid operator").at(*assign_symbol))?
                    .to_owned();

                let get_new_value = |this: &mut EvalCtx<'_>| {
                    let new_value = this.eval(value)?;
                    new_value.typecheck(this.eval_opt_ty(ty.as_deref())?)?;
                    Ok(new_value)
                };

                if assign_op_str.is_empty() {
                    let new_value = get_new_value(self)?;
                    self.assign(var, new_value)?;
                } else {
                    self.modify(
                        var,
                        Box::new(move |ctx, old_value_span, old_value| {
                            if assign_op_str == "??" {
                                match old_value.filter(|v| !v.is_null()) {
                                    Some(v) => Ok(v), // don't eval new value
                                    None => get_new_value(ctx),
                                }
                            } else {
                                let op_fn = (ctx.scope.get(&assign_op_str))
                                    .ok_or(Error::UnsupportedOperator.at(*assign_symbol))?;
                                let old_value =
                                    old_value.ok_or(Error::Undefined.at(old_value_span))?;
                                let args = vec![old_value, get_new_value(ctx)?];
                                let kwargs = Map::new();
                                let f = op_fn.as_ref::<FnValue>()?;
                                f.call_at(span, *assign_symbol, ctx, args, kwargs)
                            }
                        }),
                    )?;
                }

                Ok(null)
            }
            ast::NodeContents::FnDef { name, contents } => {
                let new_overload = self.eval_fn_contents(span, contents)?;
                let fn_name = self.substr(*name);
                self.scope.register_func(span, fn_name, new_overload)?;
                Ok(null)
            }
            ast::NodeContents::ExportAllFrom(source) => {
                self.for_all_from_map(source, |this, k, v| {
                    this.export(span, k, v);
                    Ok(())
                })?;
                Ok(null)
            }
            ast::NodeContents::ExportFrom(items, source) => {
                self.for_each_item_from_map(items, source, |this, k, v| {
                    this.export(span, k, v);
                    Ok(())
                })?;
                Ok(null)
            }
            ast::NodeContents::ExportAs(item) => {
                let key = self.substr(item.alias());
                let value = self.get_var(item.target)?;
                self.export(span, key, value);
                Ok(null)
            }
            ast::NodeContents::ExportAssign { name, ty, value } => {
                let key = self.substr(*name);
                let new_value = self.eval(value)?;
                new_value.typecheck(self.eval_opt_ty(ty.as_deref())?)?;
                self.scope.set(key.clone(), new_value.clone());
                self.export(span, key, new_value);
                Ok(null)
            }
            ast::NodeContents::ExportFnDef { name, contents } => {
                let new_overload = self.eval_fn_contents(span, contents)?;
                let fn_name = self.substr(*name);
                self.scope
                    .register_func(span, fn_name.clone(), new_overload.clone())?;
                self.exports
                    .get_or_insert_default()
                    .entry(fn_name.clone())
                    .or_default()
                    .as_func_mut(span, Some(fn_name))
                    .push_overload(new_overload)?;
                Ok(null)
            }
            ast::NodeContents::UseAllFrom(source) => {
                self.for_all_from_map(source, |this, k, v| {
                    if let Some(old_var) = self.scope.get(&k) {
                        if !(old_var.is_func() && v.is_func()) {
                            let w = Warning::ShadowedVariable((k.clone(), old_var.span), true);
                            this.runtime.report_diagnostic(w.at(span));
                        }
                    }
                    self.scope.add(k, v)
                })?;
                Ok(null)
            }
            ast::NodeContents::UseFrom(items, source) => {
                self.for_each_item_from_map(items, source, |this, k, v| {
                    if let Some(old_var) = self.scope.get(&k) {
                        if !(old_var.is_func() && v.is_func()) {
                            let w = Warning::ShadowedVariable((k.clone(), old_var.span), false);
                            this.runtime.report_diagnostic(w.at(span));
                        }
                    }
                    self.scope.add(k, v)
                })?;
                Ok(null)
            }
            ast::NodeContents::Block(items) => {
                return self.exec_in_child_scope(|ctx| {
                    if items.len() == 1 {
                        ctx.eval(&items[0])
                    } else {
                        for item in items {
                            ctx.eval(item)?;
                        }
                        Ok(null.at(span))
                    }
                });
            }
            ast::NodeContents::IfElse {
                if_cases,
                else_case,
            } => {
                let mut if_cases = if_cases.iter();
                return Ok(loop {
                    match if_cases.next() {
                        Some((cond, body)) => {
                            if self.eval(cond)?.to::<bool>()? {
                                break self.eval(body)?;
                            }
                        }
                        None => match else_case {
                            Some(body) => break self.eval(body)?,
                            None => break null.at(span),
                        },
                    }
                });
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
                            return Err(Error::WrongNumberOfLoopVars {
                                iter_span: iter_value.span,
                                count: loop_var_idents.len(),
                                min: 2,
                                max: 2,
                            }
                            .at(vars_span));
                        };
                        let iterations = map.iter().map(|(k, v)| {
                            [(key_var, k.as_str().into()), (value_var, v.data.clone())]
                        });
                        self.exec_for_loop(iterations, body)?;
                        Ok(null)
                    }
                    ValueData::Vec(vec) | ValueData::EuclidPoint(hypermath::Point(vec)) => {
                        self.exec_for_loop_indexed(loop_vars, iter_value.span, vec.iter(), body)?;
                        Ok(null)
                    }
                    _ => return Err(Error::CannotIterate(iter_value.ty()).at(iter_value.span)),
                }
            }
            ast::NodeContents::WhileLoop { condition, body } => {
                Ok(self.exec_in_child_scope(|ctx| {
                    while ctx.eval(condition)?.to::<bool>()? {
                        match ctx.eval(body) {
                            Ok(_) => (),
                            Err(e) => match e.try_resolve_loop_control_flow()? {
                                LoopControlFlow::Break => break,
                                LoopControlFlow::Continue => continue,
                            },
                        }
                    }
                    Ok(null)
                })?)
            }
            ast::NodeContents::Continue => Err(Error::Continue),
            ast::NodeContents::Break => Err(Error::Break),
            ast::NodeContents::Return(ret_expr) => Err(Error::Return(Box::new(match ret_expr {
                Some(expr) => {
                    if let Some(exports) = &self.exports {
                        let export_spans = exports.values().map(|v| v.span).collect();
                        return Err(Error::ReturnAfterExport { export_spans }.at(span));
                    }
                    self.eval(expr)?
                }
                None => null.at(span),
            }))),
            ast::NodeContents::With(ident, expr, body) => {
                let new_value = self.eval(expr)?;
                let scope = Scope::new_with_block(Arc::clone(self.scope), |special| {
                    special.set(*ident, new_value)
                })?;

                Ok(EvalCtx {
                    scope: &scope,
                    runtime: self.runtime,
                    caller_span: self.caller_span,
                    exports: self.exports,
                }
                .eval(body)?
                .data)
            }
            ast::NodeContents::Ident(ident_span) => Ok(self.get_var(*ident_span)?.data),
            ast::NodeContents::SpecialIdent(special_ident) => Ok(match special_ident {
                ast::SpecialVar::Ndim => self.ndim_at(span)?.into(),
                ast::SpecialVar::Sym => todo!("#sym"),
            }),
            ast::NodeContents::Op { op, args } => {
                if &self[*op] == "??" {
                    if let Ok([l, r]) = <&[_; 2]>::try_from(args.as_slice()) {
                        match self.eval(l)?.data {
                            ValueData::Null => Ok(self.eval(r)?.data),
                            other => Ok(other),
                        }
                    } else {
                        return Err(Error::UnsupportedOperator.at(*op));
                    }
                } else {
                    let op_fn = self
                        .scope
                        .get(&self[*op])
                        .ok_or(Error::UnsupportedOperator.at(*op))?;
                    let args = args.iter().map(|arg| self.eval(arg)).try_collect()?;
                    let kwargs = Map::new();
                    let f = op_fn.as_ref::<FnValue>()?;
                    Ok(f.call_at(span, *op, self, args, kwargs)?.data)
                }
            }
            ast::NodeContents::FnCall { func, args } => {
                let mut arg_values = Vec::with_capacity(args.len() + 1);
                let func_value =
                    if let &(ast::NodeContents::Access { ref obj, field }, obj_method_span) =
                        &**func
                    {
                        let obj = self.eval(obj)?;
                        // TODO: warn if ambiguous
                        let maybe_method = self.scope.get(&self[field]).filter(|method_value| {
                            method_value
                                .as_ref::<FnValue>()
                                .is_ok_and(|f| f.can_be_method_of(&obj.ty()))
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
                let f = func_value.as_ref::<FnValue>()?;
                let mut args_splat_span = None;
                let mut kwarg_values = Map::new();
                let mut kwargs_splat_span = None;
                for arg in args {
                    if let Some(sp) = kwargs_splat_span {
                        // already seen named arguments splat
                        return Err(Error::FnArgSplatBeforeEnd.at(sp));
                    } else if let Some(splat) = arg.value.0.as_map_splat(self) {
                        // named arguments splat
                        kwarg_values.extend(self.eval(splat)?.unwrap_or_clone_arc::<Map>()?);
                        kwargs_splat_span = Some(splat.1);
                    } else if let Some(name) = arg.name {
                        // named argument
                        kwarg_values.insert(self.substr(name), self.eval(&arg.value)?);
                    } else if !kwarg_values.is_empty() {
                        // already seen named argument
                        return Err(Error::PositionalParamAfterNamedParam.at(arg.value.1));
                    } else if let Some(sp) = args_splat_span {
                        // already seen positional arguments splat
                        return Err(Error::FnArgSplatBeforeEnd.at(sp)); // TODO: not quite accurate
                    } else if let Some(splat) = arg.value.0.as_list_splat(self) {
                        // positional arguments splat
                        arg_values.extend(self.eval(splat)?.unwrap_or_clone_arc::<List>()?);
                        args_splat_span = Some(splat.1);
                    } else {
                        // positional argument
                        arg_values.push(self.eval(&arg.value)?);
                    }
                }
                let args = arg_values;
                let kwargs = kwarg_values;
                Ok(f.call_at(span, func_value.span, self, args, kwargs)?.data)
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
            ast::NodeContents::FilePath(span) => {
                let mut path = self[*span]
                    .strip_prefix('@')
                    .ok_or(Error::Internal("missing '@'").at(*span))?;

                let resolved_path;
                let is_relative;
                if path.starts_with(['^', '/']) {
                    let mut base = self
                        .runtime
                        .modules
                        .get_path(self.caller_span.context)
                        .ok_or(Error::Internal("relative import with no path").at(*span))?;
                    while let Some(rest) = path.strip_prefix('^') {
                        path = rest;
                        let (parent, _) =
                            base.rsplit_once('/').ok_or(Error::BeyondRoot.at(*span))?;
                        base = parent;
                    }
                    // `base` does not contain a trailing slash.
                    // `path` contains a leading slash.
                    resolved_path = format!("{base}{path}");
                    is_relative = true;
                } else {
                    resolved_path = path.to_owned();
                    is_relative = false;
                };

                Ok(self.import(*span, resolved_path, is_relative)?.data)
            }
            ast::NodeContents::NullLiteral => Ok(null),
            ast::NodeContents::BoolLiteral(b) => Ok(ValueData::Bool(*b)),
            ast::NodeContents::NumberLiteral(n) => Ok(ValueData::Num(*n)),
            ast::NodeContents::StringLiteral(string_segments) => Ok(self
                .eval_string_literal_contents(span, string_segments)?
                .data),
            ast::NodeContents::ListLiteral(items) => Ok(ValueData::List(Arc::new({
                let mut ret = vec![];
                for item @ (item_contents, _) in items {
                    if let Some(inner) = item_contents.as_list_splat(self) {
                        ret.extend(self.eval(inner)?.unwrap_or_clone_arc::<List>()?);
                    } else {
                        ret.push(self.eval(item)?);
                    }
                }
                ret
            }))),
            ast::NodeContents::MapLiteral(entries) => Ok(ValueData::Map(Arc::new({
                // TODO: handle duplicate keys (maybe let splat be fallback?)
                let mut ret = Map::new();
                for entry in entries {
                    match entry {
                        ast::MapEntry::Splat { span: _, values } => {
                            ret.extend(self.eval(values)?.unwrap_or_clone_arc::<Map>()?);
                        }

                        ast::MapEntry::KeyValue {
                            key: key_node,
                            ty: ty_node,
                            value: value_node,
                        } => {
                            let key = self.eval_map_key(key_node)?;
                            let Some(value_node) = value_node else {
                                return Err(Error::MissingMapValue.at(key_node.1));
                            };
                            let value = self.eval(value_node)?;
                            value.typecheck(self.eval_opt_ty(ty_node.as_deref())?)?;
                            if !value.is_null() {
                                ret.insert(key, value);
                            }
                        }
                    }
                }
                ret
            }))),

            ast::NodeContents::Error => Err(Error::AstErrorNode),
        }
        .map(|val| val.at(span))
        .map_err(|err| err.at(span))
    }

    /// Evaluates an optional AST node to a type annotation.
    ///
    /// Returns [`Type::Any`] if the node is `None`.
    pub fn eval_opt_ty(&mut self, opt_node: Option<&ast::Node>) -> Result<Type> {
        match opt_node {
            Some(node) => self.eval(node)?.to::<Type>(),
            None => Ok(Type::Any),
        }
    }

    fn eval_fn_contents(&mut self, span: Span, contents: &ast::FnContents) -> Result<FnOverload> {
        // Parse parameters.
        let mut seq_params = vec![];
        let mut seq_splat = None;
        let mut seq_end = None;
        let mut named_params = vec![];
        let mut named_splat = None;
        for param in &contents.params {
            if let Some(splat_span) = named_splat {
                return Err(Error::FnParamSplatBeforeEnd.at(splat_span));
            }
            match param {
                ast::FnParam::Param { name, ty, default } => {
                    let ty = self.eval_opt_ty(ty.as_deref())?;

                    match seq_end {
                        None => match default {
                            Some(expr) => return Err(Error::DefaultPositionalParamValue.at(expr.1)),
                            None => seq_params.push((*name, ty)),
                        },
                        Some(_) => {
                            let default = match default {
                                Some(expr) => Some({
                                    let v = self.eval(expr)?;
                                    v.typecheck(&ty)?;
                                    v
                                }),
                                None => None,
                            };
                            named_params.push((*name, ty, default));
                        }
                    }
                }
                ast::FnParam::SeqSplat(name) => match seq_end {
                    None => {
                        seq_splat = Some(*name);
                        seq_end = Some(*name);
                    }
                    Some(previous_span) => {
                        return Err(Error::DuplicateFnParamSeqEnd { previous_span }.at(*name));
                    }
                },
                ast::FnParam::SeqEnd(new_span) => match seq_end {
                    None => seq_end = Some(*new_span),
                    Some(previous_span) => {
                        return Err(Error::DuplicateFnParamSeqEnd { previous_span }.at(*new_span));
                    }
                },
                ast::FnParam::NamedSplat(name) => {
                    named_splat = Some(*name);
                    break;
                }
            }
        }

        let return_type = self.eval_opt_ty(contents.return_type.as_deref())?;
        let fn_body = Arc::clone(&contents.body);

        // Check for duplicates.
        let mut names_seen = HashMap::new();
        for &span in seq_params
            .iter()
            .map(|(span, _)| span)
            .chain(named_params.iter().map(|(name, _, _)| name))
            .chain(&named_splat)
        {
            if let Some(previous_span) = names_seen.insert(&self[span], span) {
                return Err(Error::DuplicateFnParamName { previous_span }.at(span));
            }
        }

        // If the user annotated `-> Null` and there is only one statement in
        // the function, do not implicitly return it.
        let ignore_return_value = return_type == Type::Null
            && matches!(&fn_body.0, ast::NodeContents::Block(statements) if statements.len() == 1);

        Ok(FnOverload {
            ty: FnType {
                params: seq_params.iter().map(|(_, ty)| ty.clone()).collect(),
                is_variadic: false,
                ret: return_type.clone(),
            },
            call: Arc::new(move |ctx, args, mut kwargs| {
                let mut args = args.into_iter();
                for &(param_span, ref ty) in &seq_params {
                    let arg_value = args.next().ok_or_else(|| {
                        Error::MissingRequiredPositionalParameter { ty: ty.clone() }.at(param_span)
                    })?;
                    ctx.scope.set(ctx.substr(param_span), arg_value);
                }
                if let Some(seq_splat) = seq_splat {
                    ctx.scope.set(
                        ctx.substr(seq_splat),
                        ValueData::List(Arc::new(args.collect())).at(seq_splat),
                    );
                } else {
                    crate::util::expect_end_of_args(args)?;
                }

                for &(param_span, ref ty, ref default) in &named_params {
                    let param_name = ctx.substr(param_span);
                    let arg_value = match kwargs.swap_remove(&param_name) {
                        Some(arg) => {
                            arg.typecheck(ty)?;
                            arg
                        }
                        None => default.clone().ok_or_else(|| {
                            Error::MissingRequiredNamedParameter {
                                name: param_name.clone(),
                                ty: ty.clone(),
                            }
                            .at(param_span)
                        })?,
                    };
                    ctx.scope.set(param_name, arg_value.clone());
                }

                if let Some(splat_var) = named_splat {
                    ctx.scope.set(
                        ctx.substr(splat_var),
                        ValueData::Map(Arc::new(kwargs)).at(splat_var),
                    );
                } else if !kwargs.is_empty() {
                    crate::util::expect_end_of_kwargs(kwargs, span)?;
                }

                let mut return_value = ctx.eval(&fn_body)?;
                if ignore_return_value {
                    return_value = ValueData::Null.at(fn_body.1);
                }

                return_value.typecheck(&return_type)?;
                Ok(return_value)
            }),
            debug_info: span.into(),
            parent_scope: Some(Arc::clone(self.scope)),
            docs: None,
        })
    }

    fn eval_string_literal_contents(
        &mut self,
        whole_span: Span,
        segments: &[Spanned<ast::StringSegment>],
    ) -> Result<Value> {
        let mut interp_fn = LazyBuiltinOperator::new("$");
        let mut concat_fn = LazyBuiltinOperator::new("++");

        let mut output = InterpolatedString::default();
        for &(ref segment, span) in segments {
            match segment {
                ast::StringSegment::Literal => output.push_str(&self[span], span),
                ast::StringSegment::Char(c) => output.push_char(*c, span),
                ast::StringSegment::Interpolation(expr) => {
                    let value = self.eval(expr)?;

                    let f = interp_fn.get(self)?;
                    if f.get_overload(span, std::slice::from_ref(&value)).is_ok() {
                        output.push_value(f.call_at(span, span, self, vec![value], Map::new())?);
                    } else {
                        output.push_str(&value.to_string(), span);
                    }
                }
            }
        }

        output.into_string(whole_span, |a, b| {
            let f = concat_fn.get(self)?;
            f.call_at(whole_span, whole_span, self, vec![a, b], Map::new())
        })
    }

    fn set_var(&self, span: Span, value: impl Into<ValueData>) {
        self.scope.set(self.substr(span), value.into().at(span));
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
        match *loop_var_idents.as_slice() {
            [elem_var] => self.exec_for_loop(elems.into_iter().map(|e| [(elem_var, e)]), body),
            [index_var, elem_var] => self.exec_for_loop(
                elems
                    .into_iter()
                    .enumerate()
                    .map(|(i, e)| [(index_var, i.into()), (elem_var, e)]),
                body,
            ),
            _ => Err(Error::WrongNumberOfLoopVars {
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
                    Err(e) => match e.try_resolve_loop_control_flow()? {
                        LoopControlFlow::Break => break,
                        LoopControlFlow::Continue => continue,
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
            exports: self.exports,
        })
    }

    fn eval_map_key(&mut self, node: &ast::Node) -> Result<Key> {
        let (node_contents, node_span) = node;
        match node_contents {
            ast::NodeContents::Ident(ident_span) => Ok(self.substr(*ident_span)),
            ast::NodeContents::StringLiteral(_) => self.eval(node)?.to::<Key>(),
            _ => Err(Error::ExpectedMapKey.at(*node_span)),
        }
    }

    fn get_var(&self, ident_span: Span) -> Result<Value> {
        Ok(self
            .scope
            .get(&self[ident_span])
            .ok_or(Error::Undefined.at(ident_span))?
            .data
            .at(ident_span))
    }

    fn export(&mut self, span: Span, key: Key, value: Value) {
        let old = self
            .exports
            .get_or_insert_default()
            .insert(key.clone(), value);
        if let Some(old_exported_value) = old {
            self.runtime.report_diagnostic(
                Warning::ShadowedExport((key, old_exported_value.span)).at(span),
            );
        }
    }

    fn for_all_from_map(
        &mut self,
        source: &ast::Node,
        mut f: impl FnMut(&mut Self, Key, Value) -> Result<()>,
    ) -> Result<()> {
        let m = self.eval(source)?.unwrap_or_clone_arc::<Map>()?;
        for (k, v) in m {
            f(self, k.clone(), v.clone())?;
        }
        Ok(())
    }
    fn for_each_item_from_map(
        &mut self,
        items: &[ast::IdentAs],
        source: &ast::Node,
        mut f: impl FnMut(&mut Self, Key, Value) -> Result<()>,
    ) -> Result<()> {
        let source = self.eval(source)?;
        let source_span = source.span;
        let m = source.unwrap_or_clone_arc::<Map>()?;
        for item in items {
            let alias = self.substr(item.alias());
            let value = m
                .get(&self[item.target])
                .ok_or(Error::UndefinedIn(source_span).at(item.target))?;
            f(self, alias, value.clone())?;
        }
        Ok(())
    }

    /// Imports a file and returns its return value.
    fn import(&mut self, span: Span, path: String, is_relative: bool) -> Result<Value> {
        let file_id_to_import = self.runtime.modules.id_from_module_name(&path);
        match file_id_to_import.and_then(|id| self.runtime.load_module(id)) {
            Some(Ok(value)) => Ok(value.clone()),
            Some(Err(())) => Err(Error::SilentImportError.at(span)),
            None => Err(Error::ModuleNotFound { path, is_relative }.at(span)),
        }
    }

    /// Returns a [`Substr`] from a [`Span`]. If the span is invalid, returns an
    /// empty string.
    pub fn substr(&self, span: Span) -> Substr {
        match self.runtime.modules.get_contents(span.context) {
            Some(contents) => contents.substr(span.start as usize..span.end as usize),
            None => Substr::new(),
        }
    }

    /// Returns `#ndim`, or errors if it is undefined. The error is reported as
    /// [`Self::caller_span`].
    pub fn ndim(&self) -> Result<u8> {
        self.ndim_at(self.caller_span)
    }
    /// Returns `#ndim`, or errors if it is undefined.
    pub fn ndim_at(&self, span: Span) -> Result<u8> {
        self.scope.special.ndim.ok_or(Error::NoNdim.at(span))
    }

    /// Reports a warning at [`Self::caller_span`].
    pub fn warn(&mut self, w: impl Into<Warning>) {
        self.warn_at(self.caller_span, w);
    }
    /// Reports a warning.
    pub fn warn_at(&mut self, span: Span, w: impl Into<Warning>) {
        self.runtime.report_diagnostic(w.into().at(span));
    }

    /// Returns a function that can be used to report warnings.
    pub fn warnf<T: ToString>(&mut self) -> impl FnMut(T) {
        |msg| self.warn(msg.to_string())
    }
}

impl Index<Span> for EvalCtx<'_> {
    type Output = str;

    fn index(&self, span: Span) -> &Self::Output {
        match self.runtime.modules.get_contents(span.context) {
            Some(contents) => &contents[span.start as usize..span.end as usize],
            None => "",
        }
    }
}

#[derive(Debug, Default)]
struct InterpolatedString {
    segment_values: Vec<Value>,
}
impl InterpolatedString {
    pub fn push_str(&mut self, s: &str, span: Span) {
        self.last_segment_as_str(span).push_str(s);
    }
    pub fn push_char(&mut self, c: char, span: Span) {
        self.last_segment_as_str(span).push(c);
    }
    pub fn push_value(&mut self, v: Value) {
        if let ValueData::Str(s) = &v.data {
            self.push_str(s, v.span);
        } else {
            self.segment_values.push(v);
        }
    }

    fn last_segment_as_str(&mut self, new_span: Span) -> &mut Str {
        if !matches!(
            self.segment_values.last().map(|v| &v.data),
            Some(ValueData::Str(_)),
        ) {
            self.segment_values
                .push(ValueData::Str(Str::new()).at(new_span));
        }

        match self.segment_values.last_mut() {
            Some(Value {
                data: ValueData::Str(s),
                span,
            }) => {
                // Extend `span` to include `new_span`
                *span = Span {
                    start: span.start,
                    end: new_span.end,
                    context: span.context,
                };
                s
            }

            _ => unreachable!(),
        }
    }

    pub fn into_string(
        self,
        whole_span: Span,
        mut concat: impl FnMut(Value, Value) -> Result<Value>,
    ) -> Result<Value> {
        self.segment_values
            .into_iter()
            .map(Ok)
            .reduce(|a, b| concat(a?, b?))
            .unwrap_or(Ok(ValueData::Str(Str::new()).at(whole_span)))
    }
}

#[derive(Debug)]
enum LazyBuiltinOperator {
    /// Uninitialized.
    Uninit(&'static str),
    /// Initialized.
    Init(Arc<FnValue>),
}
impl LazyBuiltinOperator {
    pub fn new(op_str: &'static str) -> Self {
        Self::Uninit(op_str)
    }
    pub fn get(&mut self, ctx: &EvalCtx<'_>) -> Result<&FnValue> {
        match self {
            Self::Init(f) => Ok(f),
            Self::Uninit(op_str) => {
                *self = Self::Init(match ctx.scope.get(op_str) {
                    Some(v) => v.to::<Arc<FnValue>>()?,
                    None => Arc::new(FnValue::new(Some("$".into()))),
                });
                self.get(ctx)
            }
        }
    }
}
