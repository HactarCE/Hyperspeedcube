use std::{collections::HashSet, fmt};

use itertools::{Itertools, PutBack};
use regex::Regex;

use hyperpuzzle::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FilterExpr {
    Nothing,    // @nothing
    Everything, // @everything

    And(Vec<FilterExpr>),
    Or(Vec<FilterExpr>),
    Not(Box<FilterExpr>),

    OnlyColors(Vec<String>), // @only(...)

    Terminal(String), // colors, piece types, other symbols
}
impl fmt::Display for FilterExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_internal(f, None)
    }
}
impl FilterExpr {
    fn write_internal(
        &self,
        f: &mut fmt::Formatter<'_>,
        parent: Option<ParentExprType>,
    ) -> fmt::Result {
        let needs_parens = match (self.expr_type(), parent) {
            (Some(this), Some(parent)) => this < parent, // compare precedence
            _ => false,
        };

        if needs_parens {
            write!(f, "(")?;
        }
        match self {
            Self::Nothing => write!(f, "@nothing")?,
            Self::Everything => write!(f, "@everything")?,

            Self::And(exprs) if exprs.is_empty() => write!(f, "@everything")?,
            Self::Or(exprs) if exprs.is_empty() => write!(f, "@nothing")?,
            Self::And(exprs) => {
                let mut is_first = true;
                for e in exprs {
                    if is_first {
                        is_first = false;
                    } else {
                        write!(f, " ")?;
                    }
                    e.write_internal(f, self.expr_type())?;
                }
            }
            Self::Or(exprs) => {
                let mut is_first = true;
                for e in exprs {
                    if is_first {
                        is_first = false;
                    } else {
                        write!(f, " + ")?;
                    }
                    e.write_internal(f, self.expr_type())?;
                }
            }
            Self::Not(expr) => {
                write!(f, "!")?;
                expr.write_internal(f, self.expr_type())?;
            }

            Self::OnlyColors(cs) => write!(f, "@only({})", cs.iter().join(", "))?,

            Self::Terminal(s) => write!(f, "{s}")?,
        }
        if needs_parens {
            write!(f, ")")?;
        }

        Ok(())
    }
    fn expr_type(&self) -> Option<ParentExprType> {
        match self {
            Self::And(_) => Some(ParentExprType::And),
            Self::Or(_) => Some(ParentExprType::Or),
            Self::Not(_) => Some(ParentExprType::Not),
            _ => None,
        }
    }

    /// Apply basic simplifications.
    pub fn simplify(self) -> Self {
        match self {
            Self::And(exprs) => Self::simplify_intersection(exprs),
            Self::Or(exprs) => Self::simplify_union(exprs),
            Self::Not(inner) => inner.simplify_complement(),

            Self::OnlyColors(colors) if colors.is_empty() => Self::Nothing,

            other => other, // Can't simplify anything else
        }
    }
    /// Simplifies an intersection of expressions.
    fn simplify_intersection(exprs: impl IntoIterator<Item = Self>) -> Self {
        let mut operands = vec![];
        for e in exprs {
            match e.simplify() {
                Self::Nothing => return Self::Nothing,
                Self::Everything => (),
                Self::And(args) => operands.extend(args), // does not contain `Nothing` or `Everything`
                other => operands.push(other),
            }
        }
        if operands.is_empty() {
            Self::Everything
        } else {
            take_exactly_one(operands).unwrap_or_else(Self::And)
        }
    }
    /// Simplifies a union of expressions.
    fn simplify_union(exprs: impl IntoIterator<Item = Self>) -> Self {
        let mut operands = vec![];
        for e in exprs {
            match e.simplify() {
                Self::Nothing => (),
                Self::Everything => return Self::Everything,
                Self::Or(args) => operands.extend(args), // does not contain `Nothing` or `Everything`
                other => operands.push(other),
            }
        }
        if operands.is_empty() {
            Self::Nothing
        } else {
            take_exactly_one(operands).unwrap_or_else(Self::Or)
        }
    }
    /// Takes the complement of an expression, then simplifies it.
    fn simplify_complement(self) -> Self {
        match self {
            Self::Nothing => Self::Everything,
            Self::Everything => Self::Nothing,

            // De Morgan's laws
            Self::And(exprs) => {
                Self::simplify_union(exprs.into_iter().map(Self::simplify_complement))
            }
            Self::Or(exprs) => {
                Self::simplify_intersection(exprs.into_iter().map(Self::simplify_complement))
            }
            Self::Not(e) => *e,

            other => Self::Not(Box::new(other)),
        }
    }

    /// Evaluates the filter to a set of pieces.
    pub fn eval(&self, puz: &Puzzle) -> PieceMask {
        let len = puz.pieces.len();

        match self {
            Self::Nothing => PieceMask::new_empty(len),
            Self::Everything => PieceMask::new_full(len),

            Self::And(exprs) => {
                let mut ret = PieceMask::new_full(len);
                for e in exprs {
                    ret &= e.eval(puz);
                }
                ret
            }
            Self::Or(exprs) => {
                let mut ret = PieceMask::new_empty(len);
                for e in exprs {
                    ret |= e.eval(puz);
                }
                ret
            }
            Self::Not(expr) => expr.eval(puz).complement(),

            Self::OnlyColors(colors) => {
                let cs = colors
                    .iter()
                    .filter_map(|color_name| puz.colors.list.find(|_, c| c.name == *color_name))
                    .collect_vec();
                let piece_iter = puz
                    .pieces
                    .iter_filter(|p, _| cs.iter().any(|c| puz.piece_has_color(p, *c)));
                PieceMask::from_iter(len, piece_iter)
            }

            Self::Terminal(s) => {
                if let Some(piece_type) = s.strip_prefix('\'') {
                    // TODO: optimize O(n) linear search to O(1)
                    match puz.piece_types.find(|_, t| t.name == *piece_type) {
                        Some(t) => {
                            let piece_iter =
                                puz.pieces.iter_filter(|_, info| info.piece_type == Some(t));
                            PieceMask::from_iter(len, piece_iter)
                        }
                        None => PieceMask::new_empty(len), // piece type doesn't exist!
                    }
                } else {
                    let color = s;
                    match puz.colors.list.find(|_, c| c.name == *color) {
                        Some(c) => {
                            let piece_iter =
                                puz.pieces.iter_filter(|p, _| puz.piece_has_color(p, c));
                            PieceMask::from_iter(len, piece_iter)
                        }
                        None => PieceMask::new_empty(len), // color doesn't exist!
                    }
                }
            }
        }
    }

    /// Returns an error if the filter is invalid for the given puzzle.
    pub fn validate(&self, puz: &Puzzle) -> Result<(), String> {
        let mut references = HashSet::new();
        self.accumulate_references(&mut references);
        for color_info in puz.colors.list.iter_values() {
            references.remove(&color_info.name);
        }
        for piece_type_info in puz.piece_types.iter_values() {
            references.remove(&format!("'{}", piece_type_info.name));
        }

        if references.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "unknown references: {:?}",
                references.into_iter().sorted().collect_vec(),
            ))
        }
    }
    fn accumulate_references<'a>(&'a self, references: &mut HashSet<&'a String>) {
        match self {
            Self::Nothing | Self::Everything => (),
            Self::And(exprs) | Self::Or(exprs) => {
                for e in exprs.iter() {
                    e.accumulate_references(references);
                }
            }
            Self::Not(e) => e.accumulate_references(references),
            Self::OnlyColors(cs) => {
                references.extend(cs);
            }
            Self::Terminal(s) => {
                references.insert(s);
            }
        }
    }

    pub fn from_str(s: &str) -> Self {
        parser::parse(s)
    }
}

fn take_exactly_one<T>(mut v: Vec<T>) -> Result<T, Vec<T>> {
    if v.len() == 1 {
        v.pop().ok_or(v)
    } else {
        Err(v)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum ParentExprType {
    Or = 0, // lowest precedence
    And = 1,
    Not = 2, // highest precedence
}

/// Extremely dubious parser for the filter expression language. It's optimized
/// to always produce something reasonable and never produce an error.
mod parser {
    use super::*;

    pub(super) fn parse(s: &str) -> FilterExpr {
        eat_union(&mut itertools::put_back(tokenize(s)))
    }

    fn eat_union<'a>(tokens: &mut PutBack<impl Iterator<Item = &'a str>>) -> FilterExpr {
        let mut terms = vec![];
        loop {
            terms.push(eat_intersection(tokens));
            match tokens.next() {
                None | Some(")") => return FilterExpr::Or(terms),
                Some("+" | "|") => (),
                Some(t) => {
                    tokens.put_back(t);
                }
            }
        }
    }

    fn eat_intersection<'a>(tokens: &mut PutBack<impl Iterator<Item = &'a str>>) -> FilterExpr {
        FilterExpr::And(std::iter::from_fn(|| eat_atom(tokens)).collect())
    }

    fn eat_atom<'a>(tokens: &mut PutBack<impl Iterator<Item = &'a str>>) -> Option<FilterExpr> {
        lazy_static! {
            static ref NAME_REGEX: Regex =
                Regex::new(&format!(r"^{}$", hyperpuzzle::NAME_REGEX)).unwrap();
        }

        loop {
            match tokens.next()? {
                "(" => return Some(eat_union(tokens)),
                t @ (")" | "+" | "|") => {
                    tokens.put_back(t);
                    return None;
                }
                "!" | "~" => return Some(FilterExpr::Not(Box::new(eat_atom(tokens)?))),

                "@everything" => return Some(FilterExpr::Everything),
                "@nothing" => return Some(FilterExpr::Nothing),
                "@only" => {
                    if tokens.next() != Some("(") {
                        continue;
                    }
                    let mut depth = 1;
                    let mut colors = vec![];
                    while depth > 0 {
                        match tokens.next() {
                            Some("(") => depth += 1,
                            Some(")") => depth -= 1,
                            Some(other) if NAME_REGEX.is_match(other) => {
                                colors.push(other.to_owned());
                            }
                            None => depth = 0,
                            _ => (), // ignore unknown symbols (including commas)
                        }
                    }
                    return Some(FilterExpr::OnlyColors(colors));
                }
                s => {
                    if NAME_REGEX.is_match(s) {
                        return Some(FilterExpr::Terminal(s.to_owned()));
                    } else if s.chars().all(|c| c.is_whitespace()) || s == "&" {
                        continue;
                    } else {
                        return Some(FilterExpr::Terminal(s.to_owned()));
                    }
                }
            }
        }
    }

    fn tokenize<'a>(s: &'a str) -> impl Iterator<Item = &'a str> {
        lazy_static! {
            // regex for symbols we actually care about: `[+|()!~]`
            static ref TOKEN: Regex =
                Regex::new(&format!(r"['@]?{}|.", hyperpuzzle::NAME_REGEX)).unwrap();
        }

        // Just ignore unrecognized characters
        TOKEN.find_iter(&s).map(|m| m.as_str())
    }
}

#[cfg(test)]
mod tests {
    use crate::preferences::filters::*;
    use hyperpuzzle::*;

    #[test]
    fn test_filter_expressions() {
        let colors = ["A", "B", "C", "D", "E", "F"].into_iter().collect();
        let piece_types = ["p0", "p1", "p2", "p3", "p4", "p5"].into_iter().collect();
        let s = |cb: &FilterCheckboxes| cb.to_string(&colors, &piece_types);

        let init = FilterCheckboxes::new(&colors, &piece_types);
        let mut checkboxes;

        checkboxes = init.clone();
        assert_eq!("@everything", s(&checkboxes));
        checkboxes.colors[Color(0)] = Some(true);
        assert_eq!("A", s(&checkboxes));
        checkboxes.colors[Color(1)] = Some(true);
        assert_eq!("A B", s(&checkboxes));
        checkboxes.colors[Color(2)] = Some(true);
        assert_eq!("A B C", s(&checkboxes));
        checkboxes.colors[Color(3)] = Some(true);
        assert_eq!("A B C D", s(&checkboxes));
        checkboxes.colors[Color(4)] = Some(true);
        checkboxes.colors[Color(5)] = Some(true);
        assert_eq!("A B C D E F", s(&checkboxes));

        checkboxes.colors[Color(0)] = Some(false);
        assert_eq!("B C D E F !A", s(&checkboxes));
        checkboxes.colors[Color(1)] = Some(false);
        assert_eq!("C D E F !A !B", s(&checkboxes));
        checkboxes.colors[Color(2)] = Some(false);
        assert_eq!("D E F !A !B !C", s(&checkboxes));
        checkboxes.colors[Color(3)] = Some(false);
        assert_eq!("E F @only(E, F)", s(&checkboxes));
        checkboxes.colors[Color(4)] = None;
        assert_eq!("F @only(E, F)", s(&checkboxes));
        checkboxes.colors[Color(5)] = None;
        assert_eq!("@only(E, F)", s(&checkboxes));
        checkboxes.colors[Color(5)] = Some(false);
        assert_eq!("@only(E)", s(&checkboxes));

        checkboxes.piece_types[PieceType(0)] = Some(false);
        assert_eq!("@only(E) !'p0", s(&checkboxes));
        checkboxes.piece_types[PieceType(1)] = Some(false);
        assert_eq!("@only(E) !'p0 !'p1", s(&checkboxes));
        checkboxes.piece_types[PieceType(2)] = Some(false);
        assert_eq!("@only(E) !'p0 !'p1 !'p2", s(&checkboxes));
        checkboxes.piece_types[PieceType(3)] = Some(false);
        assert_eq!("@only(E) ('p4 + 'p5)", s(&checkboxes));
        checkboxes.piece_types[PieceType(4)] = Some(false);
        assert_eq!("@only(E) 'p5", s(&checkboxes));
        checkboxes.piece_types[PieceType(5)] = Some(false);
        assert_eq!("@nothing", s(&checkboxes));
    }

    #[test]
    fn test_formula_expr_parser() {
        let s1 = "  A   & ( B|!'c (DE.(F+G))&~~H";
        let s2 = "A (B + !'c (DE (F + G)) !!H)";
        let s3 = "A (B + !'c DE (F + G) H)";

        let expr1 = FilterExpr::from_str(s1);
        let expr2 = FilterExpr::from_str(s2);
        println!("Testing {s1:?} = {s2:?}");
        assert_eq!(expr1, expr2);
        println!("Testing {s2:?} identity");
        assert_eq!(s2, expr2.to_string());
        println!("Testing {s2:?} simplifies to {s3:?}");
        assert_eq!(s3, expr2.simplify().to_string());
    }
}