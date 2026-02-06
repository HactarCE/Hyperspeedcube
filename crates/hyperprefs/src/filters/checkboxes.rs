use hyperpuzzle_core::prelude::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use super::expr::*;

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct FilterCheckboxes {
    #[serde(with = "hypuz_util::serde_impl::tivec_opt_bool")]
    pub colors: PerColor<Option<bool>>,
    #[serde(with = "hypuz_util::serde_impl::tivec_opt_bool")]
    pub piece_types: PerPieceType<Option<bool>>,
}
impl FilterCheckboxes {
    pub fn eval(&self, puz: &Puzzle) -> PieceMask {
        let mut ret = PieceMask::new_full(puz.pieces.len());

        // Filter by piece type.
        for (piece_type, &state) in self.piece_types.iter().take(puz.piece_types.len()) {
            if let Some(wants_piece_type) = state {
                for piece in puz.pieces.iter_keys() {
                    if ret.contains(piece)
                        && (puz.pieces[piece].piece_type == piece_type) != wants_piece_type
                    {
                        ret.remove(piece);
                    }
                }
            }
        }

        // Filter by color.
        for (color, &state) in self.colors.iter().take(puz.colors.len()) {
            if let Some(wants_color) = state {
                for piece in puz.pieces.iter_keys() {
                    if ret.contains(piece) && puz.piece_has_color(piece, color) != wants_color {
                        ret.remove(piece);
                    }
                }
            }
        }

        ret
    }

    pub fn to_filter_expr(&self, ctx: &impl FilterCheckboxesCtx) -> FilterExpr {
        let colors_with_state = |state| self.colors.iter_filter(move |_, &s| s == state);
        let piece_types_with_state = |state| self.piece_types.iter_filter(move |_, &s| s == state);

        let mut big_intersection = vec![];

        // Colors
        big_intersection.push(FilterExpr::And(
            colors_with_state(Some(true))
                .map(|c| ctx.color_expr(c))
                .collect(),
        ));
        big_intersection.push(
            if colors_with_state(Some(false)).count() * 2 > self.colors.len() {
                FilterExpr::OnlyColors(
                    self.colors
                        .iter_filter(|_, &s| s != Some(false))
                        .map(|c| ctx.color_name(c).to_owned())
                        .collect(),
                )
            } else {
                FilterExpr::And(
                    colors_with_state(Some(false))
                        .map(|c| FilterExpr::Not(Box::new(ctx.color_expr(c))))
                        .collect(),
                )
            },
        );

        // Piece types
        let piece_type_mask =
            PieceTypeMask::from_iter(self.piece_types.len(), piece_types_with_state(None));
        let ret = optimized_piece_type_expr(ctx, ctx.piece_type_hierarchy(), "", &piece_type_mask);
        if let Some(expr) = ret.include {
            big_intersection.push(expr);
        } else {
            return FilterExpr::Nothing;
        }

        FilterExpr::And(big_intersection).simplify()
    }

    pub fn to_string(&self, ctx: &impl FilterCheckboxesCtx) -> String {
        self.to_filter_expr(ctx).to_string()
    }
}

pub trait FilterCheckboxesCtx {
    fn color_name(&self, id: Color) -> &str;
    fn piece_type_hierarchy(&self) -> &PieceTypeHierarchy;

    fn color_expr(&self, id: Color) -> FilterExpr {
        FilterExpr::Terminal(self.color_name(id).to_string())
    }
    fn piece_type_expr(&self, name: &str) -> FilterExpr {
        FilterExpr::Terminal(format!("'{name}"))
    }
}

impl FilterCheckboxesCtx for Puzzle {
    fn color_name(&self, id: Color) -> &str {
        &self.colors.names[id]
    }
    fn piece_type_hierarchy(&self) -> &PieceTypeHierarchy {
        &self.piece_type_hierarchy
    }
}

impl FilterCheckboxesCtx for (&PerColor<&str>, &PieceTypeHierarchy) {
    fn color_name(&self, id: Color) -> &str {
        self.0.get(id).copied().unwrap_or_default()
    }
    fn piece_type_hierarchy(&self) -> &PieceTypeHierarchy {
        self.1
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FilterIncludeOrExclude {
    /// Expression representing the piece types within the subtree that are
    /// **excluded**.
    include: Option<FilterExpr>,
    /// Expression representing the piece types within the subtree that are
    /// **included**.
    exclude: Option<FilterExpr>,
}

fn optimized_piece_type_expr(
    ctx: &impl FilterCheckboxesCtx,
    subtree: &PieceTypeHierarchy,
    path_prefix: &str,
    mask: &PieceTypeMask,
) -> FilterIncludeOrExclude {
    let whole_category_expr = match path_prefix {
        "" => FilterExpr::Everything,
        _ => ctx.piece_type_expr(path_prefix),
    };

    let intersection = subtree.types.clone() & mask;
    if intersection.is_empty() {
        return FilterIncludeOrExclude {
            include: None,
            exclude: Some(whole_category_expr),
        };
    }
    if intersection == subtree.types {
        return FilterIncludeOrExclude {
            include: Some(whole_category_expr),
            exclude: None,
        };
    }

    let mut includes = vec![];
    let mut excludes = vec![];

    for (name, node) in &subtree.nodes {
        let path = match path_prefix {
            "" => name.clone(),
            _ => format!("{path_prefix}/{name}"),
        };

        match &node.contents {
            PieceTypeHierarchyNodeContents::Category(cat) => {
                let result = optimized_piece_type_expr(ctx, cat, &path, mask);
                includes.extend(result.include);
                excludes.extend(result.exclude);
            }
            PieceTypeHierarchyNodeContents::Type(ty) => {
                let expr = ctx.piece_type_expr(&path);
                if mask.contains(*ty) {
                    includes.push(expr);
                } else {
                    excludes.push(expr);
                }
            }
        }
    }

    let include_ret;
    let exclude_ret;
    if excludes.len() >= includes.len() {
        include_ret = FilterExpr::Or(includes);
        exclude_ret = FilterExpr::And(vec![
            whole_category_expr,
            FilterExpr::Not(Box::new(include_ret.clone())),
        ]);
    } else {
        let mut factors = excludes
            .into_iter()
            .map(|expr| FilterExpr::Not(Box::new(expr)))
            .collect_vec();
        exclude_ret = FilterExpr::And(factors.clone());
        factors.insert(0, whole_category_expr);
        include_ret = FilterExpr::And(factors);
    };

    FilterIncludeOrExclude {
        include: Some(include_ret),
        exclude: Some(exclude_ret),
    }
}
