//! 3x3x3 Rubik's cube.

use cgmath::*;
use itertools::Itertools;
use rand::Rng;
use smallvec::{smallvec, SmallVec};
use std::collections::HashMap;
use std::fmt;
use std::ops::{Add, Index, IndexMut, Mul, Neg};
use std::sync::Arc;
use std::sync::Mutex;

use super::{
    generic::*, traits::*, LayerMask, PuzzleTypeEnum, Sign, StickerGeometry, StickerGeometryParams,
    TwistAxis, TwistMetric,
};

const DEFAULT_LAYER_COUNT: u8 = 3;
pub const MIN_LAYER_COUNT: u8 = 1;
pub const MAX_LAYER_COUNT: u8 = 9;

pub(super) fn puzzle_type(layer_count: u8) -> &'static dyn PuzzleType {
    puzzle_description(layer_count)
}

fn puzzle_description(layer_count: u8) -> &'static Rubiks3DDescription {
    lazy_static! {
        static ref CACHE: Mutex<HashMap<u8, &'static Rubiks3DDescription>> =
            Mutex::new(HashMap::new());
    }

    assert!(layer_count >= MIN_LAYER_COUNT);
    assert!(layer_count <= MAX_LAYER_COUNT);

    CACHE.lock().unwrap().entry(layer_count).or_insert_with(|| {
        let mut pieces = vec![];
        let mut stickers = vec![];

        let full_range = (0..layer_count).collect_vec();
        let ends = [0, layer_count - 1];

        let mut piece_locations = vec![];
        for z in 0..layer_count {
            let z_min = z == 0;
            let z_max = z == layer_count - 1;

            for y in 0..layer_count {
                let y_min = y == 0;
                let y_max = y == layer_count - 1;

                let x_range = if z_min || z_max || y_min || y_max {
                    full_range.as_slice()
                } else {
                    ends.as_slice()
                };
                for &x in x_range {
                    let x_min = x == 0;
                    let x_max = x == layer_count - 1;

                    let piece = Piece(pieces.len() as _);
                    let mut piece_stickers = smallvec![];

                    let mut push_sticker_if = |condition, face| {
                        if condition {
                            piece_stickers.push(Sticker(stickers.len() as _));
                            stickers.push(StickerInfo { piece, face });
                        }
                    };
                    push_sticker_if(x_max, Face::R);
                    push_sticker_if(x_min, Face::L);
                    push_sticker_if(y_max, Face::U);
                    push_sticker_if(y_min, Face::D);
                    push_sticker_if(z_max, Face::F);
                    push_sticker_if(z_min, Face::B);

                    piece_locations.push([x, y, z]);
                    pieces.push(PieceInfo {
                        stickers: piece_stickers,
                    })
                }
            }
        }

        // It's not like we'll ever clear the cache anyway, so just leak it
        // and let us have the 'static lifetimes.
        Box::leak(Box::new(Rubiks3DDescription {
            name: format!("{0}x{0}x{0}", layer_count),

            layer_count,

            faces: Face::INFO_LIST.to_vec(),
            pieces,
            stickers,
            twist_axes: TwistAxisInfo::list_from_faces(&Face::INFO_LIST),
            twist_directions: TwistDirection::INFO_LIST.to_vec(),

            piece_locations,
        }))
    })
}

#[derive(Debug, Clone)]
struct Rubiks3DDescription {
    name: String,

    layer_count: u8,

    faces: Vec<FaceInfo>,
    pieces: Vec<PieceInfo>,
    stickers: Vec<StickerInfo>,
    twist_axes: Vec<TwistAxisInfo>,
    twist_directions: Vec<TwistDirectionInfo>,

    piece_locations: Vec<[u8; 3]>,
}
impl PuzzleType for Rubiks3DDescription {
    fn ty(&self) -> PuzzleTypeEnum {
        PuzzleTypeEnum::Rubiks3D {
            layer_count: self.layer_count,
        }
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn family_display_name(&self) -> &'static str {
        "Rubik's 3D"
    }
    fn family_internal_name(&self) -> &'static str {
        "Rubiks3D"
    }

    fn layer_count(&self) -> u8 {
        self.layer_count
    }
    fn family_max_layer_count(&self) -> u8 {
        MAX_LAYER_COUNT
    }
    fn radius(&self) -> f32 {
        self.layer_count as f32 * 0.5 * 3.0_f32.sqrt()
    }
    fn scramble_moves_count(&self) -> usize {
        10 * self.layer_count as usize // TODO pulled from thin air; probably insufficient for big cubes
    }

    fn faces(&self) -> &[FaceInfo] {
        &self.faces
    }
    fn pieces(&self) -> &[PieceInfo] {
        &self.pieces
    }
    fn stickers(&self) -> &[StickerInfo] {
        &self.stickers
    }
    fn twist_axes(&self) -> &[TwistAxisInfo] {
        &self.twist_axes
    }
    fn twist_directions(&self) -> &[TwistDirectionInfo] {
        &self.twist_directions
    }

    fn reverse_twist_direction(&self, direction: TwistDirection) -> TwistDirection {
        direction.rev()
    }
    fn make_recenter_twist(&self, axis: TwistAxis) -> Result<Twist, String> {
        Ok(Twist {
            axis: match axis.face() {
                Face::R => TwistAxis(Face::U.0),
                Face::L => TwistAxis(Face::D.0),
                Face::U => TwistAxis(Face::L.0),
                Face::D => TwistAxis(Face::R.0),
                Face::F => return Err("cannot recenter near face".to_string()),
                Face::B => return Err("cannot recenter far face".to_string()),
                _ => return Err("invalid face".to_string()),
            },
            direction: TwistDirection::CW_90,
            layers: self.all_layers(),
        })
    }
    fn canonicalize_twist(&self, twist: Twist) -> Twist {
        let rev_layers = self.reverse_layers(twist.layers);
        let is_canonical = twist.layers.0 < rev_layers.0
            || twist.layers == rev_layers && twist.axis.face().sign() == Sign::Pos;
        if is_canonical {
            twist
        } else {
            Twist {
                axis: TwistAxis::from_face(twist.axis.face().opposite()),
                direction: twist.direction.rev(),
                layers: rev_layers,
            }
        }
    }

    fn twist_short_description(&self, twist: Twist) -> String {
        let face_upper = self.info(twist.axis.face()).symbol;
        let face_lower = face_upper.to_ascii_lowercase();
        let fwd = self.info(twist.direction).symbol;
        let rev = self.info(twist.direction.rev()).symbol;

        if twist.layers == LayerMask(0) {
            crate::util::INVALID_STR.to_string()
        } else if twist.layers == self.all_layers() {
            match twist.axis.face() {
                Face::R => format!("x{fwd}"),
                Face::L => format!("x{rev}"),
                Face::U => format!("y{fwd}"),
                Face::D => format!("y{rev}"),
                Face::F => format!("z{fwd}"),
                Face::B => format!("z{rev}"),
                _ => crate::util::INVALID_STR.to_string(),
            }
        } else if twist.layers.is_default() {
            format!("{face_upper}{fwd}")
        } else if twist.layers == self.slice_layers() {
            match twist.axis.face() {
                Face::R => format!("M{rev}"),
                Face::L => format!("M{fwd}"),
                Face::U => format!("E{rev}"),
                Face::D => format!("E{fwd}"),
                Face::F => format!("S{fwd}"),
                Face::B => format!("S{rev}"),
                _ => crate::util::INVALID_STR.to_string(),
            }
        } else if twist.layers == LayerMask(3) {
            format!("{face_upper}w{fwd}")
        } else if twist.layers == LayerMask(2) {
            format!("{face_lower}{fwd}")
        } else if twist.layers.is_contiguous_from_outermost() {
            format!("{}{face_upper}{fwd}", twist.layers.count())
        } else if let Some(layer) = twist.layers.get_single_layer() {
            format!("{layer}{face_lower}{fwd}")
        } else {
            format!("{{{}}}{face_upper}{fwd}", twist.layers.short_description(),)
        }
    }
}

#[derive(Debug, Clone)]
pub struct Rubiks3D {
    desc: &'static Rubiks3DDescription,
    piece_states: Box<[PieceState]>,
}
impl Eq for Rubiks3D {}
impl PartialEq for Rubiks3D {
    fn eq(&self, other: &Self) -> bool {
        self.piece_states == other.piece_states
    }
}
impl Index<Piece> for Rubiks3D {
    type Output = PieceState;

    fn index(&self, piece: Piece) -> &Self::Output {
        &self.piece_states[piece.0 as usize]
    }
}
impl IndexMut<Piece> for Rubiks3D {
    fn index_mut(&mut self, piece: Piece) -> &mut Self::Output {
        &mut self.piece_states[piece.0 as usize]
    }
}
impl PuzzleState for Rubiks3D {
    fn twist(&mut self, twist: Twist) -> Result<(), &'static str> {
        for piece in self.pieces_affected_by_twist(twist) {
            self[piece] = self[piece].twist(twist.axis, twist.direction);
        }
        Ok(())
    }
    fn layer_from_twist_axis(&self, twist_axis: TwistAxis, piece: Piece) -> u8 {
        let face = twist_axis.face();
        let face_coord = match face.sign() {
            Sign::Pos => self.layer_count() - 1,
            Sign::Neg => 0,
        };
        let piece_coord = self.piece_location(piece)[face.axis() as usize];
        u8::abs_diff(face_coord, piece_coord)
    }

    fn sticker_geometry(
        &self,
        sticker: Sticker,
        p: StickerGeometryParams,
    ) -> Option<StickerGeometry> {
        let piece = self.info(sticker).piece;
        let sticker_face = self.sticker_face(sticker);

        let mut transform = p.view_transform;
        if let Some((twist, t)) = p.twist_animation {
            if self.is_piece_affected_by_twist(twist, piece) {
                transform = transform * twist.transform(t);
            }
        }

        // Compute the center of the sticker.
        let center = transform.transform_point(self.sticker_center_3d(sticker, p));

        // Add a radius to the sticker along each axis.
        let sticker_radius = p.face_scale() * p.sticker_scale() / 2.0;

        // Compute the vectors that span the plan of the sticker.
        let (u_span_axis, v_span_axis) = sticker_face.parallel_axes();
        let u: Vector3<f32> = <Matrix3<f32> as Transform<Point3<f32>>>::transform_vector(
            &transform,
            crate::util::unit_vec3(u_span_axis as usize) * sticker_radius,
        );
        let v: Vector3<f32> = <Matrix3<f32> as Transform<Point3<f32>>>::transform_vector(
            &transform,
            crate::util::unit_vec3(v_span_axis as usize) * sticker_radius,
        );

        let twist_axis = TwistAxis::from_face(sticker_face);
        let twist_ccw = Twist {
            axis: twist_axis,
            direction: TwistDirection::CCW_90,
            layers: LayerMask::default(),
        };
        let twist_cw = self.reverse_twist(twist_ccw);
        let twist_recenter = self.make_recenter_twist(twist_axis).ok();

        Some(StickerGeometry::new_double_quad(
            [
                center - u - v,
                center - u + v,
                center + u - v,
                center + u + v,
            ],
            [Some(twist_ccw), Some(twist_cw), twist_recenter],
        ))
    }

    fn is_solved(&self) -> bool {
        todo!("is it solved?")
    }
}
#[delegate_to_methods]
#[delegate(PuzzleType, target_ref = "desc")]
impl Rubiks3D {
    pub fn new(layer_count: u8) -> Self {
        let desc = puzzle_description(layer_count);
        let piece_states = vec![PieceState::default(); desc.pieces().len()].into_boxed_slice();
        Self { desc, piece_states }
    }

    fn desc(&self) -> &Rubiks3DDescription {
        self.desc
    }

    fn piece_location(&self, piece: Piece) -> [u8; 3] {
        let piece_state = self[piece];
        let initial_location = self.desc.piece_locations[piece.0 as usize];
        let mut ret = [0_u8; 3];
        for (i, axis) in Axis::iter().enumerate() {
            let r = piece_state[axis].axis() as usize;
            ret[r] = initial_location[i];
            if piece_state[axis].sign() == Sign::Neg {
                ret[r] = self.layer_count() - 1 - ret[r];
            }
        }
        ret
    }
    fn sticker_face(&self, sticker: Sticker) -> Face {
        let sticker_info = self.info(sticker);
        let ret = self[sticker_info.piece][sticker_info.face.axis()];
        match sticker_info.face.sign() {
            Sign::Pos => ret,
            Sign::Neg => ret.opposite(),
        }
    }

    fn piece_center_3d(&self, piece: Piece, p: StickerGeometryParams) -> Point3<f32> {
        let mut ret = Point3::origin();
        let piece = self.piece_location(piece);
        for axis in Axis::iter() {
            let pos = piece[axis as usize] as f32;
            ret[axis as usize] =
                p.face_scale() * (pos / (self.layer_count() as f32 - 1.0) * 2.0 - 1.0);
        }
        ret
    }
    fn sticker_center_3d(&self, sticker: Sticker, p: StickerGeometryParams) -> Point3<f32> {
        let sticker_info = self.info(sticker);
        let piece = sticker_info.piece;
        let mut ret = self.piece_center_3d(piece, p);

        let sticker_face = self.sticker_face(sticker);
        ret[sticker_face.axis() as usize] =
            self.layer_count() as f32 * 0.5 * sticker_face.sign().float();
        ret
    }
}

impl TwistAxis {
    const fn face(self) -> Face {
        // Face-turning puzzles use the same numbering for faces and twist axes.
        Face(self.0)
    }
    const fn from_face(f: Face) -> Self {
        // Face-turning puzzles use the same numbering for faces and twist axes.
        Self(f.0)
    }

    fn rot_matrix(self, angle: Rad<f32>) -> Matrix3<f32> {
        Matrix3::from_axis_angle(self.face().vector(), angle)
    }
}
impl Face {
    const INFO_LIST: [FaceInfo; 6] = [
        FaceInfo::new("R", "Right"),
        FaceInfo::new("L", "Left"),
        FaceInfo::new("U", "Up"),
        FaceInfo::new("D", "Down"),
        FaceInfo::new("F", "Front"),
        FaceInfo::new("B", "Back"),
    ];

    const R: Self = Self::new(Axis::X, Sign::Pos);
    const L: Self = Self::new(Axis::X, Sign::Neg);
    const U: Self = Self::new(Axis::Y, Sign::Pos);
    const D: Self = Self::new(Axis::Y, Sign::Neg);
    const F: Self = Self::new(Axis::Z, Sign::Pos);
    const B: Self = Self::new(Axis::Z, Sign::Neg);

    const fn new(axis: Axis, sign: Sign) -> Self {
        let i = ((axis as u8) << 1)
            | match sign {
                Sign::Neg => 1,
                Sign::Pos => 0,
            };
        Self(i)
    }
    const fn axis(self) -> Axis {
        match self.0 >> 1 {
            0 => Axis::X,
            1 => Axis::Y,
            2 => Axis::Z,
            _ => panic!("invalid face"),
        }
    }
    const fn sign(self) -> Sign {
        match self.0 & 1 {
            0 => Sign::Pos,
            1 => Sign::Neg,
            _ => unreachable!(),
        }
    }
    fn vector(self) -> Vector3<f32> {
        (match self.axis() {
            Axis::X => Vector3::unit_x(),
            Axis::Y => Vector3::unit_y(),
            Axis::Z => Vector3::unit_z(),
        } * self.sign().float())
    }
    #[must_use]
    const fn opposite(self) -> Face {
        Self(self.0 ^ 1)
    }

    /// Returns the axes parallel to this face (all except the perpendicular
    /// axis).
    const fn parallel_axes(self) -> (Axis, Axis) {
        let (ax1, ax2) = self.axis().perpendiculars();
        match self.sign() {
            Sign::Neg => (ax2, ax1),
            Sign::Pos => (ax1, ax2),
        }
    }
}

impl TwistDirection {
    const INFO_LIST: [TwistDirectionInfo; 4] = [
        TwistDirectionInfo::new("", "CW"),
        TwistDirectionInfo::new("'", "CCW"),
        TwistDirectionInfo::new("2", "180 CW"),
        TwistDirectionInfo::new("2'", "180 CCW"),
    ];

    const CW_90: Self = Self(0);
    const CCW_90: Self = Self(1);
    const CW_180: Self = Self(2);
    const CCW_180: Self = Self(3);
    const COUNT: u8 = 4; // TODO: is this good or bad?

    #[must_use]
    fn rev(self) -> Self {
        match self {
            TwistDirection::CW_90 => Self::CCW_90,
            TwistDirection::CCW_90 => Self::CW_90,
            TwistDirection::CW_180 => Self::CCW_180,
            TwistDirection::CCW_180 => Self::CW_180,
            TwistDirection(TwistDirection::COUNT..) => panic!("invalid twist direction"),
        }
    }
    fn period(self) -> usize {
        match self {
            TwistDirection::CW_90 => 4,
            TwistDirection::CCW_90 => 4,
            TwistDirection::CW_180 => 2,
            TwistDirection::CCW_180 => 2,
            TwistDirection(TwistDirection::COUNT..) => panic!("invalid twist direction"),
        }
    }
    fn sign(self) -> Sign {
        match self {
            TwistDirection::CW_90 => Sign::Neg,
            TwistDirection::CCW_90 => Sign::Pos,
            TwistDirection::CW_180 => Sign::Neg,
            TwistDirection::CCW_180 => Sign::Pos,
            TwistDirection(TwistDirection::COUNT..) => panic!("invalid twist direction"),
        }
    }
}

impl Twist {
    fn transform(self, progress: f32) -> Matrix3<f32> {
        let angle = Rad::full_turn() * self.direction.sign().float()
            / self.direction.period() as f32
            * progress;
        self.axis.rot_matrix(angle)
    }
}

/// The facing directions of the X+, Y+, and Z+ stickers on this piece (assuming
/// it has those stickers).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PieceState([Face; 3]);
impl Default for PieceState {
    fn default() -> Self {
        Self([Face::R, Face::U, Face::F])
    }
}
impl Index<Axis> for PieceState {
    type Output = Face;

    fn index(&self, axis: Axis) -> &Self::Output {
        &self.0[axis as usize]
    }
}
impl IndexMut<Axis> for PieceState {
    fn index_mut(&mut self, axis: Axis) -> &mut Self::Output {
        &mut self.0[axis as usize]
    }
}
impl PieceState {
    #[must_use]
    fn rotate(mut self, from: Axis, to: Axis) -> Self {
        let diff = (from as u8 ^ to as u8) << 1;
        for face in &mut self.0 {
            if face.axis() == from || face.axis() == to {
                face.0 ^= diff; // Swap axes
            }
        }
        self.mirror(from) // Flip sign of one axis
    }
    #[must_use]
    fn mirror(mut self, axis: Axis) -> Self {
        for face in &mut self.0 {
            if face.axis() == axis {
                *face = face.opposite();
            }
        }
        self
    }

    #[must_use]
    fn twist(self, twist_axis: TwistAxis, mut direction: TwistDirection) -> Self {
        let face = twist_axis.face();
        let axis = face.axis();
        if face.sign() == Sign::Neg {
            direction = direction.rev();
        }
        let (a, b) = axis.perpendiculars();
        match direction {
            TwistDirection::CW_90 => self.rotate(a, b),
            TwistDirection::CCW_90 => self.rotate(b, a),
            TwistDirection::CW_180 => self.mirror(a).mirror(b),
            TwistDirection::CCW_180 => self.mirror(a).mirror(b),
            TwistDirection(TwistDirection::COUNT..) => panic!("invalid twist direction"),
        }
    }
}

/*

/// State of a 3x3x3 Rubik's cube.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Rubiks33([[[Orientation; 3]; 3]; 3]);
impl Index<Piece> for Rubiks33 {
    type Output = Orientation;

    fn index(&self, pos: Piece) -> &Self::Output {
        &self.0[pos.z_idx()][pos.y_idx()][pos.x_idx()]
    }
}
impl IndexMut<Piece> for Rubiks33 {
    fn index_mut(&mut self, pos: Piece) -> &mut Self::Output {
        &mut self.0[pos.z_idx()][pos.y_idx()][pos.x_idx()]
    }
}
impl PuzzleState for Rubiks33 {
    type Piece = Piece;
    type Sticker = Sticker;
    type Face = Face;
    type Twist = Twist;
    type Orientation = Orientation;

    const NAME: &'static str = "3x3x3";
    const TYPE: PuzzleType = PuzzleType::Rubiks33;
    const NDIM: usize = 3;
    const LAYER_COUNT: usize = 3;

    const PIECE_TYPE_NAMES: &'static [&'static str] = &["center", "edge", "corner"];

    const SCRAMBLE_MOVES_COUNT: usize = 30; // pulled from thin air

    fn get_sticker_color(&self, pos: Sticker) -> Face {
        self[pos.piece()][pos.axis()] * pos.sign()
    }

    lazy_static_array_methods! {
        fn pieces() -> &'static [Piece] {

            itertools::iproduct!(Sign::iter(), Sign::iter(), Sign::iter())
                .map(|(z, y, x)| Piece([x, y, z]))
                .filter(|&p| p != Piece::core())
        }
        fn stickers() -> &'static [Sticker] {
            Rubiks33::pieces().iter().copied().flat_map(Piece::stickers)
        }
    }
    fn faces() -> &'static [Face] {
        const RET: &[Face] = &[
            Face::new(Axis::Z, Sign::Neg), // Back
            Face::new(Axis::Y, Sign::Neg), // Down
            Face::new(Axis::X, Sign::Neg), // Left
            Face::new(Axis::X, Sign::Pos), // Right
            Face::new(Axis::Y, Sign::Pos), // Up
            Face::new(Axis::Z, Sign::Pos), // Front
        ];
        RET
    }

    lazy_static_generic_array_methods! {}

    fn face_symbols() -> &'static [&'static str] {
        &["B", "D", "L", "R", "U", "F"]
    }
    fn face_names() -> &'static [&'static str] {
        &["Back", "Down", "Left", "Right", "Up", "Front"]
    }

    fn twist_direction_symbols() -> &'static [&'static str] {
        &["", "'"]
    }
    fn twist_direction_names() -> &'static [&'static str] {
        &["CW", "CCW"]
    }
}
impl Rubiks33 {
    fn transform_point(point: Point3<f32>, p: StickerGeometryParams) -> Point3<f32> {
        let point = p.model_transform.transform_point(point);
        p.view_transform.transform_point(point / PUZZLE_RADIUS)
    }
}

/// Piece location in a 3x3x3 Rubik's cube.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Piece(pub [Sign; 3]);
impl FacetTrait for Piece {
    impl_facet_trait_id_methods!(Piece, Rubiks33::pieces());

    fn projection_center(self, p: StickerGeometryParams) -> Option<Point3<f32>> {
        Some(Rubiks33::transform_point(self.center_3d(p), p))
    }
}
impl PieceTrait<Rubiks33> for Piece {
    fn piece_type(self) -> PieceType {
        PieceType {
            ty: Rubiks33::TYPE,
            id: self.sticker_count() - 1,
        }
    }

    fn layer(self, face: Face) -> Option<usize> {
        match self[face.axis()] * face.sign() {
            Sign::Neg => Some(2),
            Sign::Zero => Some(1),
            Sign::Pos => Some(0),
        }
    }

    fn sticker_count(self) -> usize {
        self.x().abs() + self.y().abs() + self.z().abs()
    }
    fn stickers(self) -> Vec<Sticker> {
        Axis::iter()
            .filter(move |&axis| self[axis].is_nonzero())
            .map(move |axis| Sticker::new(self, axis))
            .collect()
    }
}
impl Index<Axis> for Piece {
    type Output = Sign;
    fn index(&self, axis: Axis) -> &Sign {
        &self.0[axis as usize]
    }
}
impl IndexMut<Axis> for Piece {
    fn index_mut(&mut self, axis: Axis) -> &mut Sign {
        &mut self.0[axis as usize]
    }
}
// impl Add<Face> for Piece {
//     type Output = Piece;
//     fn add(mut self, rhs: Face) -> Self {
//         self[rhs.axis] = self[rhs.axis] + rhs.sign;
//         self
//     }
// }
impl Piece {
    // /// Returns the piece at the center of the puzzle, which has no stickers.
    // pub fn core() -> Self {
    //     Self([Sign::Zero; 3])
    // }
    // /// Returns the X coordinate of this piece.
    // pub fn x(self) -> Sign {
    //     self[Axis::X]
    // }
    // /// Returns the Y coordinate of this piece.
    // pub fn y(self) -> Sign {
    //     self[Axis::Y]
    // }
    // /// Returns the Z coordinate of the piece.
    // pub fn z(self) -> Sign {
    //     self[Axis::Z]
    // }
    // /// Returns the X coordinate of this piece, in the range 0..=2.
    // fn x_idx(self) -> usize {
    //     (self.x().int() + 1) as usize
    // }
    // /// Returns the Y coordinate of this piece, in the range 0..=2.
    // fn y_idx(self) -> usize {
    //     (self.y().int() + 1) as usize
    // }
    // /// Returns the Z coordinate of this piece, in the range 0..=2.
    // fn z_idx(self) -> usize {
    //     (self.z().int() + 1) as usize
    // }

    fn center_3d(self, p: StickerGeometryParams) -> Point3<f32> {
        let mut ret = Point3::origin();
        for axis in Axis::iter() {
            ret[axis as usize] = p.face_scale() * self[axis].float();
        }
        ret
    }
}

/// Sticker location on a 3x3x3 Rubik's cube.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sticker {
    piece: Piece,
    axis: Axis,
}
impl FacetTrait for Sticker {
    impl_facet_trait_id_methods!(Sticker, Rubiks33::stickers());

    fn projection_center(self, p: StickerGeometryParams) -> Option<Point3<f32>> {
        Some(Rubiks33::transform_point(self.center_3d(p), p))
    }
}
impl StickerTrait<Rubiks33> for Sticker {
    fn piece(self) -> Piece {
        self.piece
    }
    fn face(self) -> Face {
        let axis = self.axis;
        let sign = self.piece[axis];
        Face::new(axis, sign)
    }

    fn geometry(self, p: StickerGeometryParams) -> Option<StickerGeometry> {
        let (ax1, ax2) = self.face().parallel_axes();

        // Compute the center of the sticker.
        let center = self.center_3d(p);

        // Add a radius to the sticker along each axis.
        let sticker_radius = p.face_scale() * p.sticker_scale() / 2.0;
        let get_corner = |v, u| {
            let mut vert = center;
            vert[ax1 as usize] += u * sticker_radius;
            vert[ax2 as usize] += v * sticker_radius;
            Rubiks33::transform_point(vert, p)
        };

        Some(StickerGeometry::new_double_quad([
            get_corner(-1.0, -1.0),
            get_corner(-1.0, 1.0),
            get_corner(1.0, -1.0),
            get_corner(1.0, 1.0),
        ]))
    }
}
impl Sticker {
    /// Returns the sticker on the given piece along the given axis. Panics if
    /// the given sticker does not exist.
    pub fn new(piece: Piece, axis: Axis) -> Self {
        assert!(
            piece[axis].is_nonzero(),
            "{:?} does not have sticker on {:?}",
            piece,
            axis
        );
        Self { piece, axis }
    }
    /// Returns the axis perpendicular to this sticker.
    pub fn axis(self) -> Axis {
        self.axis
    }
    /// Returns the sign of the normal of the sticker. (E.g. if the sticker is
    /// facing the positive direction along its axis, then this returns
    /// Sign::Pos; if it is facing the negative direction, this returns
    /// Sign::Neg).
    pub fn sign(self) -> Sign {
        self.piece()[self.axis()]
    }

    fn center_3d(self, p: StickerGeometryParams) -> Point3<f32> {
        let mut ret = self.piece().center_3d(p);
        ret[self.axis() as usize] = 1.5 * self.sign().float();
        ret
    }
}

/// Face of a 3D cube/cuboid.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Face {
    axis: Axis,
    sign: Sign,
}
impl FacetTrait for Face {
    impl_facet_trait_id_methods!(Face, Rubiks33::faces());

    fn projection_center(self, p: StickerGeometryParams) -> Option<Point3<f32>> {
        self.center_sticker().projection_center(p)
    }
}
impl FaceTrait<Rubiks33> for Face {
    fn pieces(self, layer: usize) -> Vec<Piece> {
        let mut piece = self.center();
        for _ in 0..layer {
            piece = piece + self.opposite();
        }
        let (ax1, ax2) = self.axis.perpendiculars();
        itertools::iproduct!(Sign::iter(), Sign::iter())
            .map(move |(v, u)| {
                piece[ax1] = u;
                piece[ax2] = v;
                piece
            })
            .collect()
    }
    fn stickers(self) -> Vec<Sticker> {
        self.pieces(0)
            .into_iter()
            .map(move |piece| Sticker::new(piece, self.axis))
            .collect()
    }
}
impl Neg for Face {
    type Output = Face;
    fn neg(self) -> Self {
        Self {
            sign: -self.sign,
            ..self
        }
    }
}
impl Mul<Sign> for Face {
    type Output = Face;
    fn mul(self, rhs: Sign) -> Self {
        Self {
            sign: self.sign * rhs,
            ..self
        }
    }
}
impl Face {
    /// Right face.
    pub const R: Face = Face::new(Axis::X, Sign::Pos);
    /// Left face.
    pub const L: Face = Face::new(Axis::X, Sign::Neg);
    /// Top face.
    pub const U: Face = Face::new(Axis::Y, Sign::Pos);
    /// Bottom face.
    pub const D: Face = Face::new(Axis::Y, Sign::Neg);
    /// Front face.
    pub const F: Face = Face::new(Axis::Z, Sign::Pos);
    /// Back face.
    pub const B: Face = Face::new(Axis::Z, Sign::Neg);

    /// Returns the face on the given axis with the given sign. Panics if given
    /// Sign::Zero.
    pub const fn new(axis: Axis, sign: Sign) -> Self {
        assert!(sign.is_nonzero(), "invalid sign for face");
        Self { axis, sign }
    }
    /// Returns the axis perpendicular to this face.
    pub const fn axis(self) -> Axis {
        self.axis
    }
    /// Returns the sign of this face along its perpendicular axis.
    pub const fn sign(self) -> Sign {
        self.sign
    }
    /// Returns the opposite face.
    #[must_use]
    pub fn opposite(self) -> Self {
        Self {
            axis: self.axis,
            sign: -self.sign,
        }
    }
    /// Returns the piece at the center of this face.
    pub fn center(self) -> Piece {
        let mut ret = Piece::core();
        ret[self.axis] = self.sign;
        ret
    }
    /// Returns the sticker at the center of this face.
    pub fn center_sticker(self) -> Sticker {
        Sticker::new(self.center(), self.axis)
    }
    /// Returns the axes parallel to this face (all except the perpendicular
    /// axis).
    pub fn parallel_axes(self) -> (Axis, Axis) {
        let (ax1, ax2) = self.axis.perpendiculars();
        match self.sign {
            Sign::Neg => (ax2, ax1),
            Sign::Zero => panic!("invalid face"),
            Sign::Pos => (ax1, ax2),
        }
    }
}

/// Twist of a single face on a 3x3x3 Rubik's cube.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Twist {
    /// Face to twist.
    pub face: Face,
    /// Direction to twist the face.
    pub direction: TwistDirection2D,
    /// Layer mask.
    pub layers: LayerMask,
}
impl fmt::Display for Twist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.layers.0 {
            // Simple moves.
            0b001 => write!(f, "{}{}", self.face.symbol(), self.direction),

            // Wide moves.
            0b011 => write!(f, "{}w{}", self.face.symbol(), self.direction),

            // Slice moves.
            0b010 => match self.face.axis() {
                Axis::X => write!(f, "M{}", self.direction * -self.face.sign()),
                Axis::Y => write!(f, "E{}", self.direction * -self.face.sign()),
                Axis::Z => write!(f, "S{}", self.direction * self.face.sign()),
            },

            // Whole cube rotations.
            0b111 => match self.face.axis() {
                Axis::X => write!(f, "x{}", self.direction * self.face.sign()),
                Axis::Y => write!(f, "y{}", self.direction * self.face.sign()),
                Axis::Z => write!(f, "z{}", self.direction * self.face.sign()),
            },

            // Anything else has a bad layer mask.
            _ => write!(f, "<unknown twist>"),
        }
    }
}
impl TwistTrait<Rubiks33> for Twist {
    fn from_face_with_layers(
        face: Face,
        direction: &str,
        layers: LayerMask,
    ) -> Result<Twist, &'static str> {
        let direction = match direction {
            "CW" => TwistDirection2D::CW,
            "CCW" => TwistDirection2D::CCW,
            _ => return Err("invalid direction"),
        };
        layers.validate::<Rubiks33>()?;
        Ok(Self {
            face,
            direction,
            layers,
        })
    }
    fn from_face_recenter(face: Face) -> Result<Twist, &'static str> {
        match face {
            Face::R => Ok(*twists::Y),
            Face::L => Ok(twists::Y.rev()),
            Face::U => Ok(twists::X.rev()),
            Face::D => Ok(*twists::X),
            Face::F => Err("cannot recenter near face"),
            Face::B => Err("cannot recenter far face"),
            _ => Err("invalid face"),
        }
    }
    fn from_sticker(
        sticker: Sticker,
        direction: TwistDirection2D,
        layers: LayerMask,
    ) -> Result<Twist, &'static str> {
        layers.validate::<Rubiks33>()?;
        Ok(Self {
            face: sticker.face(),
            direction,
            layers,
        })
    }
    fn from_rng() -> Self {
        let mut rng = rand::thread_rng();
        Self {
            face: Rubiks33::faces()[rng.gen_range(0..Rubiks33::faces().len())],
            direction: if rng.gen() {
                TwistDirection2D::CW
            } else {
                TwistDirection2D::CCW
            },
            layers: LayerMask(rng.gen_range(1..((1 << Rubiks33::LAYER_COUNT) - 1))),
        }
    }

    fn model_transform(self, t: f32) -> cgmath::Matrix4<f32> {
        let mut axis = Vector3::zero();
        axis[self.face.axis() as usize] = self.face.sign().float();
        let angle = Deg(t * 90.0 * self.direction.sign().float());
        Matrix4::from_axis_angle(axis, angle)
    }

    fn rotation(self) -> Orientation {
        // Get the axes of the plane of rotation.
        let (ax1, ax2) = self.face.parallel_axes();
        let rot = Orientation::rot90(ax1, ax2);
        // Reverse orientation if counterclockwise.
        match self.direction {
            TwistDirection2D::CW => rot,
            TwistDirection2D::CCW => rot.rev(),
        }
    }
    fn rev(self) -> Self {
        Self {
            face: self.face,
            direction: self.direction.rev(),
            layers: self.layers,
        }
    }
    fn affects_piece(self, piece: Piece) -> bool {
        match piece[self.face.axis()] * self.face.sign() {
            Sign::Neg => self.layers[2],
            Sign::Zero => self.layers[1],
            Sign::Pos => self.layers[0],
        }
    }

    fn can_combine(self, previous: Option<Self>, metric: TwistMetric) -> bool {
        if self.is_whole_puzzle_rotation() {
            match metric {
                TwistMetric::Qstm | TwistMetric::Ftm | TwistMetric::Stm => true,
                TwistMetric::Etm => false,
            }
        } else if let Some(prev) = previous {
            match metric {
                TwistMetric::Qstm => false,
                TwistMetric::Ftm => self.face == prev.face && self.layers == prev.layers,
                TwistMetric::Stm => self.face == prev.face,
                TwistMetric::Etm => false,
            }
        } else {
            false
        }
    }
    fn is_whole_puzzle_rotation(self) -> bool {
        self.layers == LayerMask::all::<Rubiks33>()
    }
}
impl Twist {
    /// Make a wide (2-layer) move from this move.
    pub const fn wide(mut self) -> Self {
        self.layers = LayerMask(0b011);
        self
    }
    /// Make a slice move from this move.
    pub const fn slice(mut self) -> Self {
        self.layers = LayerMask(0b010);
        self
    }
    /// Make a whole cube rotation from this move.
    pub const fn whole_cube(mut self) -> Self {
        self.layers = LayerMask::all::<Rubiks33>();
        self
    }
    /// Twist different layers.
    pub fn layers(mut self, layers: LayerMask) -> Result<Self, &'static str> {
        layers.validate::<Rubiks33>()?;
        self.layers = layers;
        Ok(self)
    }
}

/// 3-dimensional axis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Axis {
    /// X axis (right).
    X = 0,
    /// Y axis (up).
    Y = 1,
    /// Z axis (towards the camera).
    Z = 2,
}
impl Axis {
    /// Returns the perpendicular axes from this one, using the left-hand rule.
    /// (The cross product of the returned axes is the opposite of the input.)
    /// This is more convenient for twisty puzzles, where clockwise rotations
    /// are the default.
    pub fn perpendiculars(self) -> (Axis, Axis) {
        use Axis::*;
        match self {
            X => (Z, Y), // X+ => rotate from Z+ to Y+.
            Y => (X, Z), // Y+ => rotate from X+ to Z+.
            Z => (Y, X), // Z+ => rotate from Y+ to X+.
        }
    }

    /// Returns an iterator over all axes.
    pub fn iter() -> impl Iterator<Item = Axis> {
        [Axis::X, Axis::Y, Axis::Z].into_iter()
    }
}

/// Orientation of a 3D cube (i.e. a single piece of a 3D cube/cuboid).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Orientation([Face; 3]);
impl OrientationTrait<Rubiks33> for Orientation {
    fn rev(self) -> Self {
        let mut ret = Self::default();
        for axis in Axis::iter() {
            ret[self[axis].axis] = Face::new(axis, self[axis].sign);
        }
        ret
    }
}
impl Default for Orientation {
    fn default() -> Self {
        Self([
            // Face color on the right side.
            Face::new(Axis::X, Sign::Pos),
            // Face color on the top side.
            Face::new(Axis::Y, Sign::Pos),
            // Face color on the front side.
            Face::new(Axis::Z, Sign::Pos),
        ])
    }
}
impl Index<Axis> for Orientation {
    type Output = Face;
    fn index(&self, axis: Axis) -> &Face {
        &self.0[axis as usize]
    }
}
impl IndexMut<Axis> for Orientation {
    fn index_mut(&mut self, axis: Axis) -> &mut Face {
        &mut self.0[axis as usize]
    }
}
impl Mul<Orientation> for Orientation {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        let mut ret = Self::default();
        for axis in Axis::iter() {
            ret[axis] = rhs[self[axis].axis] * self[axis].sign;
        }
        ret
    }
}
impl Mul<Piece> for Orientation {
    type Output = Piece;
    fn mul(self, rhs: Piece) -> Piece {
        let mut ret = Piece::core();
        for axis in Axis::iter() {
            ret[axis] = rhs[self[axis].axis] * self[axis].sign;
        }
        ret
    }
}
impl Mul<Sticker> for Orientation {
    type Output = Sticker;
    fn mul(self, rhs: Sticker) -> Self::Output {
        let mut ret = rhs;
        ret.piece = self * rhs.piece;
        ret.axis = self[rhs.axis].axis;
        ret
    }
}
impl Orientation {
    /// Returns an orientation representing a 90-degree rotation from one axis
    /// to another.
    #[must_use]
    pub fn rot90(axis1: Axis, axis2: Axis) -> Self {
        let mut ret = Self::default();
        ret[axis2] = Face::new(axis1, Sign::Pos);
        ret[axis1] = Face::new(axis2, Sign::Neg);
        ret
    }
}


 */

/// 3-dimensional axis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Axis {
    /// X axis (right).
    X = 0,
    /// Y axis (up).
    Y = 1,
    /// Z axis (towards the camera).
    Z = 2,
}
impl Axis {
    /// Returns the perpendicular axes from this one, using the left-hand rule.
    /// (The cross product of the returned axes is the opposite of the input.)
    /// This is more convenient for twisty puzzles, where clockwise rotations
    /// are the default.
    const fn perpendiculars(self) -> (Axis, Axis) {
        use Axis::*;
        match self {
            X => (Z, Y), // X+ => rotate from Z+ to Y+.
            Y => (X, Z), // Y+ => rotate from X+ to Z+.
            Z => (Y, X), // Z+ => rotate from Y+ to X+.
        }
    }

    /// Returns an iterator over all axes.
    fn iter() -> impl Iterator<Item = Axis> {
        [Axis::X, Axis::Y, Axis::Z].into_iter()
    }
}
