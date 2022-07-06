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
            format!("{}{face_upper}w{fwd}", twist.layers.count())
        } else if let Some(layer) = twist.layers.get_single_layer() {
            let layer = layer + 1;
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
        let sticker_radius = p.face_scale(self.layer_count()) * p.sticker_scale() / 2.0;

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
        let pos = self.piece_location(piece);
        for axis in Axis::iter() {
            ret[axis as usize] = p.face_scale(self.layer_count())
                * (pos[axis as usize] as f32 - (self.layer_count() as f32 - 1.0) / 2.0);
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
