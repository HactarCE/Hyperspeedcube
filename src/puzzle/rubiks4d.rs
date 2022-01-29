//! 3x3x3x3 Rubik's cube.

use cgmath::{Deg, InnerSpace, Matrix3, Matrix4, SquareMatrix, Vector3, Vector4, Zero};
use itertools::Itertools;
use std::fmt;
use std::ops::{Add, Index, IndexMut, Mul, Neg};
use std::str::FromStr;

use super::{traits::*, FaceId, LayerMask, PuzzleType, Sign, TwistDirection};
use crate::render::WireframeVertex;

/// Maximum extent of any single coordinate along the X, Y, Z, or W axes.
const PUZZLE_RADIUS: f32 = 1.5;

/// Some pre-baked twists that can be applied to a 3x3x3x3 Rubik's cube.
pub mod twists {
    use super::*;

    lazy_static! {
        /// Turn the whole cube 90 degrees up.
        pub static ref X: Twist = by_3d_view(Face::I, Axis::X, TwistDirection::CW).whole_cube();
        /// Turn the whole cube 90 degrees to the left.
        pub static ref Y: Twist = by_3d_view(Face::I, Axis::Y, TwistDirection::CW).whole_cube();
        /// Turn the whole cube 90 degrees clockwise.
        pub static ref Z: Twist = by_3d_view(Face::I, Axis::Z, TwistDirection::CW).whole_cube();

    }

    /// Constructs a twist of `face` along `axis`
    pub fn by_3d_view(face: Face, axis: Axis, direction: TwistDirection) -> Twist {
        let mut sticker = face.center_sticker();
        if face.axis() == axis {
            sticker.piece[Axis::W] = face.sign();
        } else if face == Face::O {
            sticker.piece[axis] = Sign::Neg;
        } else {
            sticker.piece[axis] = Sign::Pos;
        }
        Twist::new(sticker, direction)
    }
}

/// State of a 3x3x3x3 Rubik's cube.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Rubiks4D([[[[Orientation; 3]; 3]; 3]; 3]);
impl Index<Piece> for Rubiks4D {
    type Output = Orientation;

    fn index(&self, pos: Piece) -> &Self::Output {
        &self.0[pos.w_idx()][pos.z_idx()][pos.y_idx()][pos.x_idx()]
    }
}
impl IndexMut<Piece> for Rubiks4D {
    fn index_mut(&mut self, pos: Piece) -> &mut Self::Output {
        &mut self.0[pos.w_idx()][pos.z_idx()][pos.y_idx()][pos.x_idx()]
    }
}
impl PuzzleState for Rubiks4D {
    type Piece = Piece;
    type Sticker = Sticker;
    type Face = Face;
    type Twist = Twist;
    type Orientation = Orientation;

    const NAME: &'static str = "Rubik's 4D";
    const TYPE: PuzzleType = PuzzleType::Rubiks4D;
    const NDIM: usize = 4;
    const LAYER_COUNT: usize = 3;

    const PIECE_TYPE_NAMES: &'static [&'static str] = &["1c", "2c", "3c", "4c"];

    const STICKER_MODEL_VERTEX_COUNT: u16 = 8;
    const STICKER_MODEL_SURFACE_INDICES: &'static [u16] = &[
        1, 2, 3, 2, 1, 0, // Z-
        7, 6, 5, 4, 5, 6, // Z+
        0, 1, 4, 5, 4, 1, // Y-
        6, 3, 2, 3, 6, 7, // Y+
        2, 4, 6, 4, 2, 0, // X-
        7, 5, 3, 1, 3, 5, // X+
    ];
    const STICKER_MODEL_OUTLINE_INDICES: &'static [u16] = &[
        0, 1, 1, 3, 3, 2, 2, 0, // Z-
        7, 6, 6, 4, 4, 5, 5, 7, // Z+
        0, 1, 1, 5, 5, 4, 4, 0, // Y-
        7, 6, 6, 2, 2, 3, 3, 7, // Y+
        0, 2, 2, 6, 6, 4, 4, 0, // X-
        7, 5, 5, 1, 1, 3, 3, 7, // X+
    ];

    fn get_sticker_color(&self, pos: Sticker) -> Face {
        self[pos.piece()][pos.axis()] * pos.sign()
    }

    lazy_static_array_methods! {
        fn pieces() -> &'static [Piece] {
            itertools::iproduct!(Sign::iter(), Sign::iter(), Sign::iter(), Sign::iter())
                .map(|(w, z, y, x)| Piece([x, y, z, w]))
                .filter(|&p| p != Piece::core())
        }
        fn stickers() -> &'static [Sticker] {
            Rubiks4D::faces().iter().flat_map(|&face| {
                let mut stickers = face.stickers();
                // Sort in the same order that MC4D uses (decreasing order of
                // piece type).
                stickers.sort_by_key(|s| -(s.piece.sticker_count() as i32));
                stickers
            })
        }
    }

    fn faces() -> &'static [Face] {
        const RET: &[Face] = &[
            Face::new(Axis::W, Sign::Neg), // In
            Face::new(Axis::Z, Sign::Neg), // Back
            Face::new(Axis::Y, Sign::Neg), // Down
            Face::new(Axis::X, Sign::Neg), // Left
            Face::new(Axis::X, Sign::Pos), // Right
            Face::new(Axis::Y, Sign::Pos), // Up
            Face::new(Axis::Z, Sign::Pos), // Front
            Face::new(Axis::W, Sign::Pos), // Out
        ];
        RET
    }

    fn face_symbols() -> &'static [&'static str] {
        &["I", "B", "D", "L", "R", "U", "F", "O"]
    }
    fn face_names() -> &'static [&'static str] {
        &["In", "Back", "Down", "Left", "Right", "Up", "Front", "Out"]
    }
    fn default_face_colors() -> &'static [[f32; 3]] {
        &[
            crate::colors::PURPLE, // In
            crate::colors::YELLOW, // Back
            crate::colors::GREEN,  // Down
            crate::colors::ORANGE, // Left
            crate::colors::RED,    // Right
            crate::colors::BLUE,   // Up
            crate::colors::WHITE,  // Front
            crate::colors::PINK,   // Out
        ]
    }

    fn twist_direction_names() -> &'static [&'static str] {
        &["x", "x'", "y", "y'", "z", "z'"]
    }
}
impl Rubiks4D {
    fn transform_point(mut point: Vector4<f32>, p: GeometryParams) -> Vector3<f32> {
        // Compute the maximum extent along any axis from the origin in the 3D
        // projection of the puzzle. We will divide all XYZ coordinates by this
        // to normalize the puzzle size.
        let projection_radius = p.project_4d(cgmath::vec4(1.0, 0.0, 0.0, 1.0)).magnitude();

        point = p.model_transform * point;
        point /= PUZZLE_RADIUS;
        point.w /= 1.0 - p.face_spacing;
        p.view_transform * (p.project_4d(point) / projection_radius)
    }
}

/// Piece location in a 3x3x3x3 Rubik's cube.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Piece(pub [Sign; 4]);
impl FacetTrait for Piece {
    impl_facet_trait_id_methods!(Piece, Rubiks4D::pieces());

    fn projection_center(self, p: GeometryParams) -> Vector3<f32> {
        Rubiks4D::transform_point(self.center_4d(p), p)
    }
}
impl PieceTrait<Rubiks4D> for Piece {
    fn piece_type_id(self) -> usize {
        self.sticker_count() - 1
    }

    fn layer(self, face: Face) -> Option<usize> {
        match self[face.axis()] * face.sign() {
            Sign::Neg => Some(2),
            Sign::Zero => Some(1),
            Sign::Pos => Some(0),
        }
    }

    fn sticker_count(self) -> usize {
        self.x().abs() + self.y().abs() + self.z().abs() + self.w().abs()
    }
    fn stickers(self) -> Vec<Sticker> {
        Axis::iter()
            .filter(move |&axis| self[axis].is_nonzero())
            .map(move |axis| Sticker { piece: self, axis })
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
impl Add<Face> for Piece {
    type Output = Piece;
    fn add(mut self, rhs: Face) -> Self {
        self[rhs.axis] = self[rhs.axis] + rhs.sign;
        self
    }
}
impl Piece {
    /// Returns the piece at the center of the puzzle, which has no stickers.
    pub fn core() -> Self {
        Self([Sign::Zero; 4])
    }
    /// Returns the X coordinate of this piece.
    pub fn x(self) -> Sign {
        self[Axis::X]
    }
    /// Returns the Y coordinate of this piece.
    pub fn y(self) -> Sign {
        self[Axis::Y]
    }
    /// Returns the Z coordinate of the piece.
    pub fn z(self) -> Sign {
        self[Axis::Z]
    }
    /// Returns the W coordinate of the piece.
    pub fn w(self) -> Sign {
        self[Axis::W]
    }
    /// Returns the X coordinate of this piece, in the range 0..=2.
    fn x_idx(self) -> usize {
        (self.x().int() + 1) as usize
    }
    /// Returns the Y coordinate of this piece, in the range 0..=2.
    fn y_idx(self) -> usize {
        (self.y().int() + 1) as usize
    }
    /// Returns the Z coordinate of this piece, in the range 0..=2.
    fn z_idx(self) -> usize {
        (self.z().int() + 1) as usize
    }
    /// Returns the W coordinate of this piece, in the range 0..=2.
    fn w_idx(self) -> usize {
        (self.w().int() + 1) as usize
    }

    fn center_4d(self, p: GeometryParams) -> Vector4<f32> {
        let mut ret = Vector4::zero();
        for axis in Axis::iter() {
            ret[axis as usize] = p.face_scale() * self[axis].float();
        }
        ret
    }
}

/// Sticker location on a 3x3x3x3 Rubik's cube.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sticker {
    piece: Piece,
    axis: Axis,
}
impl FacetTrait for Sticker {
    impl_facet_trait_id_methods!(Sticker, Rubiks4D::stickers());

    fn projection_center(self, p: GeometryParams) -> Vector3<f32> {
        Rubiks4D::transform_point(self.center_4d(p), p)
    }
}
impl StickerTrait<Rubiks4D> for Sticker {
    fn piece(self) -> Piece {
        self.piece
    }
    fn face(self) -> Face {
        Face::new(self.axis(), self.sign())
    }

    fn verts(self, p: GeometryParams) -> Option<Vec<WireframeVertex>> {
        let [ax1, ax2, ax3] = self.face().parallel_axes();

        // Compute the center of the sticker.
        let center = self.center_4d(p);

        // Add a radius to the sticker along each axis.
        let sticker_radius = p.face_scale() * p.sticker_scale() / 2.0;
        let get_corner = |v, u, t| {
            let mut vert = center;
            vert[ax1 as usize] += t * sticker_radius;
            vert[ax2 as usize] += u * sticker_radius;
            vert[ax3 as usize] += v * sticker_radius;
            Rubiks4D::transform_point(vert, p)
        };
        let corners = [
            get_corner(-1.0, -1.0, -1.0),
            get_corner(-1.0, -1.0, 1.0),
            get_corner(-1.0, 1.0, -1.0),
            get_corner(-1.0, 1.0, 1.0),
            get_corner(1.0, -1.0, -1.0),
            get_corner(1.0, -1.0, 1.0),
            get_corner(1.0, 1.0, -1.0),
            get_corner(1.0, 1.0, 1.0),
        ];
        // Only show this sticker if the 3D volume is positive. (Cull it if its
        // 3D volume is negative.)
        Matrix3::from_cols(
            corners[1] - corners[0],
            corners[2] - corners[0],
            corners[4] - corners[0],
        )
        .determinant()
        .is_sign_positive()
        .then(|| WireframeVertex::cube(corners, p.fill_color, p.line_color).collect())
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
    /// Returns the faces adjacent to the sticker, in order, not including its
    /// own face.
    fn adj_faces(self) -> StickerAdjFaces {
        let mut a = vec![];
        let mut c = vec![];
        let mut parity = Sign::Pos;
        for axis in self.face().parallel_axes() {
            let sign = self.piece()[axis];
            match sign {
                Sign::Neg | Sign::Pos => {
                    parity = parity * sign;
                    if c.len() % 2 == 1 {
                        parity = -parity;
                    }
                    a.push(Face { axis, sign })
                }
                Sign::Zero => c.push(axis),
            }
        }

        assert_eq!(a.len() + c.len(), 3);
        if parity == Sign::Neg {
            // This always performs a single swap.
            a.reverse();
            c.reverse();
        }

        match a.len() {
            0 => StickerAdjFaces::_0 {
                centered: [c[0], c[1], c[2]],
            },
            1 => StickerAdjFaces::_1 {
                adjacent: a[0],
                centered: [c[1], c[0]],
            },
            2 => StickerAdjFaces::_2 {
                adjacent: [a[0], a[1]],
                centered: c[0],
            },
            3 => StickerAdjFaces::_3 {
                adjacent: [a[0], a[1], a[2]],
            },
            _ => unreachable!(),
        }
    }
    /// Returns the 3D vector to the sticker from the center of its face.
    pub fn vec3_within_face(self) -> [Sign; 3] {
        let [ax1, ax2, ax3] = self.axis().sticker_order_perpendiculars();
        [self.piece()[ax1], self.piece()[ax2], self.piece()[ax3]]
    }

    fn center_4d(self, p: GeometryParams) -> Vector4<f32> {
        let mut ret = self.piece().center_4d(p);
        ret[self.axis() as usize] = 1.5 * self.sign().float();
        ret
    }
}

/// Faces that a sticker is adjacent to and axes that it is centered along.
///
/// Order is only guaranteed for stickers with 3 adjacent faces.
#[derive(Debug, Copy, Clone)]
enum StickerAdjFaces {
    _0 { centered: [Axis; 3] },
    _1 { adjacent: Face, centered: [Axis; 2] },
    _2 { adjacent: [Face; 2], centered: Axis },
    _3 { adjacent: [Face; 3] },
}

/// Face of a 3D cube/cuboid.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Face {
    axis: Axis,
    sign: Sign,
}
impl FacetTrait for Face {
    impl_facet_trait_id_methods!(Face, Rubiks4D::faces());

    fn projection_center(self, p: GeometryParams) -> Vector3<f32> {
        self.center_sticker().projection_center(p)
    }
}
impl FaceTrait<Rubiks4D> for Face {
    fn pieces(self, layer: usize) -> Vec<Piece> {
        let mut piece = self.center();
        for _ in 0..layer {
            piece = piece + self.opposite();
        }
        let [ax1, ax2, ax3] = self.axis.sticker_order_perpendiculars();

        itertools::iproduct!(Sign::iter(), Sign::iter(), Sign::iter())
            .map(move |(v, u, t)| {
                piece[ax1] = t;
                piece[ax2] = u;
                piece[ax3] = v;
                piece
            })
            .collect()
    }
    fn stickers(self) -> Vec<Sticker> {
        let axis = self.axis;
        self.pieces(0)
            .into_iter()
            .map(move |piece| Sticker::new(piece, axis))
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
    /// Outer face.
    pub const O: Face = Face::new(Axis::W, Sign::Pos);
    /// Inner face.
    pub const I: Face = Face::new(Axis::W, Sign::Neg);

    /// Returns the face on the given axis with the given sign. Panics if given
    /// Sign::Zero.
    pub const fn new(axis: Axis, sign: Sign) -> Self {
        // assert!(sign.is_nonzero(), "invalid sign for face"); // TODO: panicking in const functions is unstable
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
        Sticker {
            piece: self.center(),
            axis: self.axis,
        }
    }
    /// Returns the axes parallel to this face (all except the perpendicular
    /// axis).
    pub fn parallel_axes(self) -> [Axis; 3] {
        use Axis::*;
        let [ax1, ax2, ax3] = match self.axis {
            X => [Y, Z, W],
            Y => [X, W, Z],
            Z => [W, X, Y],
            W => [Z, Y, X],
        };
        match self.sign {
            Sign::Neg => [ax2, ax1, ax3],
            Sign::Zero => panic!("invalid face"),
            Sign::Pos => [ax1, ax2, ax3],
        }
    }
}

/// Twist of a single face on a 3x3x3x3 Rubik's cube.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Twist {
    /// Sticker to twist around.
    pub sticker: Sticker,
    /// Direction to twist the face.
    pub direction: TwistDirection,
    /// Layer mask.
    pub layers: [bool; 3],
}
impl fmt::Display for Twist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sticker_id = self.sticker.id();
        let direction = self.direction.sign().int();
        let layer_mask =
            (self.layers[0] as u8) | ((self.layers[1] as u8) << 1) | ((self.layers[2] as u8) << 2);
        write!(f, "{},{},{}", sticker_id, direction, layer_mask)
    }
}
impl FromStr for Twist {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let [s1, s2, s3]: [&str; 3] = s.split(',').collect_vec().try_into().map_err(|_| ())?;
        let sticker_id: usize = s1.parse().map_err(|_| ())?;
        let direction: isize = s2.parse().map_err(|_| ())?;
        let layer_mask: usize = s3.parse().map_err(|_| ())?;

        let sticker = Sticker::from_id(sticker_id).ok_or(())?;
        let direction = match direction {
            -1 => TwistDirection::CW,
            1 => TwistDirection::CCW,
            _ => return Err(()),
        };
        let layers = [
            layer_mask & 1 != 0,
            layer_mask & 2 != 0,
            layer_mask & 4 != 0,
        ];

        Ok(Self {
            sticker,
            direction,
            layers,
        })
    }
}
impl TwistTrait<Rubiks4D> for Twist {
    fn from_twist_command(
        face_id: FaceId,
        direction: &str,
        layer_mask: LayerMask,
    ) -> Result<Twist, &'static str> {
        let face = Face::from_id(face_id.0 as usize).ok_or("invalid face")?;
        let (axis, direction) = match direction {
            "x" => (Axis::X, TwistDirection::CW),
            "x'" => (Axis::X, TwistDirection::CCW),
            "y" => (Axis::Y, TwistDirection::CW),
            "y'" => (Axis::Y, TwistDirection::CCW),
            "z" => (Axis::Z, TwistDirection::CW),
            "z'" => (Axis::Z, TwistDirection::CCW),
            _ => return Err("invalid direction"),
        };
        if layer_mask.0 > 0b111 {
            return Err("invaild layer mask");
        }
        let layers = [
            layer_mask.0 & 0b001 != 0,
            layer_mask.0 & 0b010 != 0,
            layer_mask.0 & 0b100 != 0,
        ];
        Ok(twists::by_3d_view(face, axis, direction).layers(layers))
    }
    fn from_recenter_command(face_id: FaceId) -> Result<Twist, &'static str> {
        let face = Face::from_id(face_id.0 as usize).ok_or("invalid face")?;
        if face.axis() == Axis::W {
            return Err("cannot recenter near or far face");
        }
        let (axis1, axis2) = Axis::perpendicular_plane(face.axis(), Axis::W);
        let mut sticker = Face::new(axis1, face.sign()).center_sticker();
        sticker.piece[axis2] = match face.axis() {
            Axis::X => Sign::Pos,
            Axis::Y => Sign::Neg,
            Axis::Z => Sign::Pos,
            Axis::W => return Err("cannot recenter near or far face"),
        };
        Ok(Twist::new(sticker, TwistDirection::CW).whole_cube())
    }

    fn model_matrix(self, t: f32) -> Matrix4<f32> {
        let [s1, s2, s3] = self.sticker.vec3_within_face();
        let face = self.sticker.face();
        let mut axis = cgmath::vec3(s1.float(), s2.float(), s3.float()).normalize();
        if "LUBO".contains(face.symbol()) {
            axis *= -1.0;
        }
        let angle = Deg(t * 360.0 / self.symmetry_order() as f32 * self.direction.sign().float());
        let mat3 = Matrix3::from_axis_angle(axis, angle);

        // Rearrange rows and columns.
        let mut ret = Matrix4::identity();
        let axes = self.sticker.axis().sticker_order_perpendiculars();
        for (row1, row2) in axes.into_iter().enumerate() {
            for (col1, col2) in axes.into_iter().enumerate() {
                ret[col2 as usize][row2 as usize] = mat3[col1][row1];
            }
        }
        ret
    }

    fn rotation(self) -> Orientation {
        let rot = match self.sticker.adj_faces() {
            StickerAdjFaces::_0 { .. } => Orientation::default(),

            StickerAdjFaces::_1 {
                adjacent: _,
                centered: [axis1, axis2],
            } => Orientation::rot90(axis1, axis2),

            StickerAdjFaces::_2 {
                adjacent: [face1, face2],
                centered,
            } => Orientation::rot180(face1, face2, centered),

            StickerAdjFaces::_3 {
                adjacent: [face1, face2, face3],
            } => Orientation::rot120(face3, face2, face1),
        };

        // Reverse orientation if counterclockwise.
        match self.direction {
            TwistDirection::CW => rot,
            TwistDirection::CCW => rot.rev(),
        }
    }
    fn rev(self) -> Self {
        Self {
            sticker: self.sticker,
            direction: self.direction.rev(),
            layers: self.layers,
        }
    }
    fn affects_piece(self, piece: Piece) -> bool {
        let face = self.sticker.face();
        match piece[face.axis()] * face.sign() {
            Sign::Neg => self.layers[2],
            Sign::Zero => self.layers[1],
            Sign::Pos => self.layers[0],
        }
    }
}
impl From<Sticker> for Twist {
    fn from(sticker: Sticker) -> Self {
        Self {
            sticker,
            direction: TwistDirection::default(),
            layers: [true, false, false],
        }
    }
}
impl Twist {
    /// Returns the sticker with the given ID.
    pub fn from_sticker_idx(i: usize) -> Self {
        Self::from(Sticker::from_id(i).unwrap())
    }

    /// Returns the number of repetitions of this twist required before the
    /// puzzle returns to the original state.
    fn symmetry_order(self) -> usize {
        match self.sticker.adj_faces() {
            StickerAdjFaces::_0 { .. } => 1, // invalid
            StickerAdjFaces::_1 { .. } => 4, // 90-degree rotation
            StickerAdjFaces::_2 { .. } => 2, // 180-degree rotation
            StickerAdjFaces::_3 { .. } => 3, // 120-degree rotation
        }
    }
    /// Returns a twist of the face from the given sticker and in the given
    /// direction.
    pub const fn new(sticker: Sticker, direction: TwistDirection) -> Self {
        Self {
            sticker,
            direction,
            layers: [true, false, false],
        }
    }
    /// Make a fat (2-layer) move from this move.
    pub const fn fat(self) -> Self {
        self.layers([true, true, false])
    }
    /// Make a slice move from this move.
    pub const fn slice(self) -> Self {
        self.layers([false, true, false])
    }
    /// Make a whole cube rotation from this move.
    pub const fn whole_cube(self) -> Self {
        self.layers([true, true, true])
    }
    /// Twist different layers.
    pub const fn layers(mut self, layers: [bool; 3]) -> Self {
        self.layers = layers;
        self
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
    /// W axis (towards the 4D camera).
    W = 3,
}
impl Axis {
    /// Returns the perpendicular axes from this one, in the order used for
    /// calculating sticker indices.
    pub fn sticker_order_perpendiculars(self) -> [Axis; 3] {
        use Axis::*;
        // This ordering is necessary in order to maintain compatibility with
        // MC4D sticker indices.
        match self {
            X => [Y, Z, W],
            Y => [X, Z, W],
            Z => [X, Y, W],
            W => [X, Y, Z],
        }
    }
    /// Returns the axes of the oriented plane perpendicular to two other axes.
    pub fn perpendicular_plane(axis1: Axis, axis2: Axis) -> (Axis, Axis) {
        let [t, u, v] = axis1.sticker_order_perpendiculars();
        if axis2 == t {
            (u, v)
        } else if axis2 == u {
            (v, t)
        } else if axis2 == v {
            (t, u)
        } else {
            panic!("no perpendicular plane")
        }
    }
    /// Returns the axis perpendicular to three other axes.
    pub fn perpendicular_axis(axes: [Axis; 3]) -> Axis {
        Axis::iter().find(|ax| !axes.contains(ax)).unwrap()
    }

    /// Returns an iterator over all axes.
    pub fn iter() -> impl Iterator<Item = Axis> {
        [Axis::X, Axis::Y, Axis::Z, Axis::W].into_iter()
    }
}

/// Orientation of a 4D cube (i.e. a single piece of a 4D cube/cuboid).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Orientation([Face; 4]);
impl OrientationTrait<Rubiks4D> for Orientation {
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
            // Face color on the right cell.
            Face::new(Axis::X, Sign::Pos),
            // Face color on the top cell.
            Face::new(Axis::Y, Sign::Pos),
            // Face color on the front cell.
            Face::new(Axis::Z, Sign::Pos),
            // Face color on the outer cell.
            Face::new(Axis::W, Sign::Pos),
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
impl Orientation {
    /// Returns an orientation representing a 90-degree rotation from one axis
    /// to another.
    #[must_use]
    pub fn rot90(axis1: Axis, axis2: Axis) -> Self {
        let face1 = Face::new(axis1, Sign::Pos);
        let face2 = Face::new(axis2, Sign::Pos);
        Self::rot90_faces(face1, face2)
    }
    /// Returns an orientation representing a 90-degree rotation from one face
    /// to another.
    #[must_use]
    pub fn rot90_faces(face1: Face, face2: Face) -> Self {
        let mut ret = Self::default();
        ret[face2.axis()] = face1 * face2.sign();
        ret[face1.axis()] = face2 * -face1.sign();
        ret
    }

    /// Returns an orientation representing a 180-degree rotation exchanging two
    /// adjacent faces and inverting a third axis.
    #[must_use]
    pub fn rot180(face1: Face, face2: Face, invert_axis: Axis) -> Self {
        let mut ret = Self::default();
        ret[face1.axis()] = face2 * face1.sign();
        ret[face2.axis()] = face1 * face2.sign();
        ret[invert_axis] = -ret[invert_axis];
        ret
    }

    /// Returns an orientation representing a 120-degree rotation exchanging
    /// three mutually adjacent faces.
    #[must_use]
    pub fn rot120(face1: Face, face2: Face, face3: Face) -> Self {
        Self::rot90_faces(face1, face2) * Self::rot90_faces(face2, face3)
    }

    /// Negates all axes.
    #[must_use]
    fn invert(mut self) -> Self {
        for axis in Axis::iter() {
            self[axis] = -self[axis];
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_4d_twist_serialization() {
        for &sticker in Rubiks4D::stickers() {
            for layer_mask in 0..8 {
                for direction in [TwistDirection::CCW, TwistDirection::CW] {
                    let twist = Twist {
                        sticker,
                        direction,
                        layers: [
                            (layer_mask & 1) != 0,
                            (layer_mask & 2) != 0,
                            (layer_mask & 4) != 0,
                        ],
                    };
                    assert_eq!(twist, twist.to_string().parse().unwrap());
                }
            }
        }
    }
}
