use hyperpuzzle::*;
use serde::{Deserialize, Serialize};

use super::expr::*;

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct FilterCheckboxes {
    #[serde(with = "crate::serde_impl::vec_opt_bool")]
    pub colors: PerColor<Option<bool>>,
    #[serde(with = "crate::serde_impl::vec_opt_bool")]
    pub piece_types: PerPieceType<Option<bool>>,
}
impl FilterCheckboxes {
    pub fn new<C, T>(colors: &PerColor<C>, piece_types: &PerPieceType<T>) -> Self {
        Self {
            colors: colors.map_ref(|_, _| None),
            piece_types: piece_types.map_ref(|_, _| None),
        }
    }
    pub fn from_puzzle(puz: &Puzzle) -> Self {
        Self::new(&puz.colors.list, &puz.piece_types)
    }

    pub fn eval(&self, puz: &Puzzle) -> PieceMask {
        let mut ret = PieceMask::new_full(puz.pieces.len());

        // Filter by piece type.
        for (piece_type, &state) in self.piece_types.iter().take(puz.piece_types.len()) {
            if let Some(wants_piece_type) = state {
                for piece in puz.pieces.iter_keys() {
                    if ret.contains(piece)
                        && (puz.pieces[piece].piece_type == Some(piece_type)) != wants_piece_type
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

    pub fn to_filter_expr(
        &self,
        colors: &PerColor<&str>,
        piece_types: &PerPieceType<&str>,
    ) -> FilterExpr {
        let color_expr = |c| FilterExpr::Terminal(format!("{}", colors[c]));
        let piece_type_expr = |t| FilterExpr::Terminal(format!("'{}", piece_types[t]));

        let colors_with_state = |state| self.colors.iter_filter(move |_, &s| s == state);
        let piece_types_with_state = |state| self.piece_types.iter_filter(move |_, &s| s == state);

        let mut big_intersection = vec![];

        // Colors
        big_intersection.push(FilterExpr::And(
            colors_with_state(Some(true)).map(color_expr).collect(),
        ));
        big_intersection.push(
            if colors_with_state(Some(false)).count() * 2 > self.colors.len() {
                FilterExpr::OnlyColors(
                    self.colors
                        .iter_filter(|_, &s| s != Some(false))
                        .map(|c| colors[c].to_owned())
                        .collect(),
                )
            } else {
                FilterExpr::And(
                    colors_with_state(Some(false))
                        .map(|c| FilterExpr::Not(Box::new(color_expr(c))))
                        .collect(),
                )
            },
        );

        // Piece types
        big_intersection.push(
            if piece_types_with_state(Some(false)).count() > piece_types_with_state(None).count() {
                FilterExpr::Or(piece_types_with_state(None).map(piece_type_expr).collect())
            } else {
                FilterExpr::And(
                    piece_types_with_state(Some(false))
                        .map(|t| FilterExpr::Not(Box::new(piece_type_expr(t))))
                        .collect(),
                )
            },
        );

        FilterExpr::And(big_intersection).simplify()
    }

    pub fn to_string(&self, colors: &PerColor<&str>, piece_types: &PerPieceType<&str>) -> String {
        self.to_filter_expr(colors, piece_types).to_string()
    }
}
