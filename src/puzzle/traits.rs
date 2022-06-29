use super::*; // TODO better import

#[delegatable_trait]
#[enum_dispatch]
pub trait PuzzleType {
    fn name(&self) -> &str;
    fn ty(&self) -> PuzzleTypeEnum;

    fn layer_count(&self) -> u8;
    /// Returns the maximum extent of any single coordinate along the X, Y, or Z
    /// axes in the 3D projection.
    fn max_extent(&self) -> f32;
    fn scramble_moves_count(&self) -> usize;

    fn faces(&self) -> &[FaceInfo];
    fn pieces(&self) -> &[PieceInfo];
    fn stickers(&self) -> &[StickerInfo];
    fn twist_axes(&self) -> &[TwistAxisInfo];
    fn twist_directions(&self) -> &[TwistDirectionInfo];

    fn check_layers(&self, layer_mask: LayerMask) -> Result<(), &'static str> {
        let layer_count = self.layer_count() as u32;
        if layer_mask.0 > 0 || layer_mask.0 < 1 << layer_count {
            Ok(())
        } else {
            Err("invalid layer mask")
        }
    }
    fn all_layers(&self) -> LayerMask {
        let layer_count = self.layer_count() as u32;
        LayerMask((1 << layer_count) - 1)
    }

    fn reverse_twist_direction(&self, direction: TwistDirection) -> TwistDirection;
    fn reverse_twist(&self, twist: Twist) -> Twist {
        Twist {
            axis: twist.axis,
            direction: self.reverse_twist_direction(twist.direction),
            layer_mask: twist.layer_mask,
        }
    }
    fn make_recenter_twist(&self, face: Face) -> Result<Twist, String>;
    fn canonicalize_twist(&self, twist: Twist) -> Twist;

    fn can_combine_twists(&self, prev: Option<Twist>, curr: Twist, metric: TwistMetric) -> bool {
        // TODO: at least try?
        false
    }
}

#[enum_dispatch]
pub trait PuzzleState: PuzzleType {
    fn twist(&mut self, twist: Twist) -> Result<(), &'static str>;
    fn pieces_affected_by_twist(&self, twist: Twist) -> Vec<Piece> {
        (0..self.pieces().len() as _)
            .map(Piece)
            .filter(|&p| twist.layer_mask[self.layer_from_twist_axis(twist.axis, p)])
            .collect()
    }
    fn layer_from_twist_axis(&self, twist_axis: TwistAxis, piece: Piece) -> u8;

    fn sticker_geometry(
        &self,
        sticker: Sticker,
        p: StickerGeometryParams,
    ) -> Option<StickerGeometry>;

    fn is_solved(&self) -> bool;
}

pub trait PuzzleInfo<T> {
    type Output;

    fn info(&self, thing: T) -> Self::Output;
}
macro_rules! impl_puzzle_info_trait {
    (fn $method:ident($thing:ty) -> $thing_info:ty) => {
        impl<T: PuzzleType + ?Sized> PuzzleInfo<$thing> for T {
            type Output = $thing_info;

            fn info(&self, thing: $thing) -> $thing_info {
                self.$method()[thing.0 as usize].clone()
            }
        }
    };
}
impl_puzzle_info_trait!(fn faces(Face) -> FaceInfo);
impl_puzzle_info_trait!(fn pieces(Piece) -> PieceInfo);
impl_puzzle_info_trait!(fn stickers(Sticker) -> StickerInfo);
impl_puzzle_info_trait!(fn twist_axes(TwistAxis) -> TwistAxisInfo);
impl_puzzle_info_trait!(fn twist_directions(TwistDirection) -> TwistDirectionInfo);
