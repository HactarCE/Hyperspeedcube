use arcstr::ArcStr;

use crate::{ast, value::Value};

pub struct Ctx {
    pub src: ArcStr,
}

impl Ctx {
    pub fn eval(&mut self, node: &ast::Node) -> Result<Value, String> {
        let (contents, span) = node;
        match contents {
            ast::NodeContents::Assign {
                var,
                ty,
                assign_symbol,
                value,
            } => todo!(),
            ast::NodeContents::Export(_) => todo!(),
            ast::NodeContents::FnDef { name, contents } => todo!(),
            ast::NodeContents::ImportAllFrom(import_path) => todo!(),
            ast::NodeContents::ImportFrom(simple_spans, import_path) => todo!(),
            ast::NodeContents::ImportAs(import_path, simple_span) => todo!(),
            ast::NodeContents::Import(simple_span) => todo!(),
            ast::NodeContents::UseAllFrom(_) => todo!(),
            ast::NodeContents::UseFrom(simple_spans, _) => todo!(),
            ast::NodeContents::Block(items) => {
                if items.len() == 1 {
                    self.eval(&items[0])
                } else {
                    for item in items {
                        self.eval(item)?;
                    }
                    Ok(Value::Null)
                }
            }
            ast::NodeContents::IfElse {
                if_cases,
                else_case,
            } => todo!(),
            ast::NodeContents::ForLoop {
                loop_vars,
                iterator,
                body,
            } => todo!(),
            ast::NodeContents::WhileLoop { condition, body } => todo!(),
            ast::NodeContents::Continue => todo!(),
            ast::NodeContents::Break => todo!(),
            ast::NodeContents::Return(_) => todo!(),
            ast::NodeContents::Ident(simple_span) => todo!(),
            ast::NodeContents::Op { op, args } => match &self.src[op.into_range()] {
                "-" => Ok(Value::Number(
                    self.eval(&args[0]).unwrap().unwrap_num()
                        - self.eval(&args[1]).unwrap().unwrap_num(),
                )),
                "<" => Ok(Value::Bool(
                    self.eval(&args[0]).unwrap().unwrap_num()
                        < self.eval(&args[1]).unwrap().unwrap_num(),
                )),
                s => panic!("{s:?}"),
            },
            ast::NodeContents::FnCall { func, args } => todo!(),
            ast::NodeContents::Paren(_) => todo!(),
            ast::NodeContents::Access { obj, field } => todo!(),
            ast::NodeContents::Index { obj, args } => todo!(),
            ast::NodeContents::Fn(fn_contents) => todo!(),
            ast::NodeContents::NullLiteral => Ok(Value::Null),
            ast::NodeContents::BoolLiteral(b) => Ok(Value::Bool(*b)),
            ast::NodeContents::NumberLiteral(n) => Ok(Value::Number(*n)),
            ast::NodeContents::StringLiteral(string_segments) => todo!(),
            ast::NodeContents::ListLiteral(items) => todo!(),
            ast::NodeContents::MapLiteral(items) => todo!(),
        }
    }
}
