//! Domain-specific language for defining puzzles for Hyperspeedcube.

#![warn(variant_size_differences)]

mod ast;
mod error;
mod eval;
mod lexer;
mod parser;
mod ty;
mod util;
mod value;

use eval::Ctx;
// use span::{Span, Spanned};
use ty::{FnType, Type};
use value::Value;

// A few type definitions to be used by our parsers below
pub type Span = chumsky::span::SimpleSpan;
pub type Spanned<T> = (T, Span);

#[test]
pub fn test_eval() {
    let src = include_str!("../resources/hps/polygonal.hps");
    let ast = parser::parse(src).unwrap();
    let mut ctx = Ctx { src: src.into() };
    println!("{:?}", ctx.eval(&ast))
}
