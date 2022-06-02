//! 3x3x3x3 Rubik's cube.

use cgmath::{
    Deg, EuclideanSpace, InnerSpace, Matrix3, Matrix4, MetricSpace, Point3, SquareMatrix,
    Transform, Vector4, Zero,
};
use itertools::Itertools;
use rand::Rng;
use std::fmt;
use std::ops::{Add, Index, IndexMut, Mul, Neg};
use std::str::FromStr;

use super::{
    common_4d::Axis, rubiks34, traits::*, LayerMask, PieceType, PuzzleType, Sign, StickerGeometry,
    StickerGeometryParams, TwistDirection2D, TwistMetric,
};

/// Maximum extent of any single coordinate along the X, Y, Z, or W axes.
const PUZZLE_RADIUS: f32 = 1.0;

/// Some pre-baked twists that can be applied to a 3x3x3x3 Rubik's cube.
pub mod twists {
    use super::*;
    use TwistDirection2D::*;

    lazy_static! {
        static ref LAYERS: LayerMask = LayerMask::all::<Rubiks24>();

        /// Turn the whole cube 90 degrees up.
        pub static ref X: Twist = Twist::by_3d_view(Face::I, Axis::X, CW, *LAYERS).unwrap();
        /// Turn the whole cube 90 degrees to the left.
        pub static ref Y: Twist = Twist::by_3d_view(Face::I, Axis::Y, CW, *LAYERS).unwrap();
        /// Turn the whole cube 90 degrees clockwise.
        pub static ref Z: Twist = Twist::by_3d_view(Face::I, Axis::Z, CW, *LAYERS).unwrap();
    }
}

/// State of a 3x3x3x3 Rubik's cube.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Rubiks24([[[[Orientation; 2]; 2]; 2]; 2]);
impl Index<Piece> for Rubiks24 {
    type Output = Orientation;

    fn index(&self, pos: Piece) -> &Self::Output {
        &self.0[pos.w_idx()][pos.z_idx()][pos.y_idx()][pos.x_idx()]
    }
}
impl IndexMut<Piece> for Rubiks24 {
    fn index_mut(&mut self, pos: Piece) -> &mut Self::Output {
        &mut self.0[pos.w_idx()][pos.z_idx()][pos.y_idx()][pos.x_idx()]
    }
}
impl PuzzleState for Rubiks24 {
    type Piece = Piece;
    type Sticker = Sticker;
    type Face = Face;
    type Twist = Twist;
    type Orientation = Orientation;

    const NAME: &'static str = "2x2x2x2";
    const TYPE: PuzzleType = PuzzleType::Rubiks24;
    const NDIM: usize = 4;
    const LAYER_COUNT: usize = 2;

    const PIECE_TYPE_NAMES: &'static [&'static str] = &["4c"];

    const SCRAMBLE_MOVES_COUNT: usize = 20; // based on what MC4D does

    fn get_sticker_color(&self, pos: Sticker) -> Face {
        self[pos.piece()][pos.axis()] * pos.sign()
    }

    lazy_static_array_methods! {
        fn pieces() -> &'static [Piece] {
            let signs = [Sign::Neg, Sign::Pos];
            itertools::iproduct!(signs, signs, signs, signs)
                .map(|(w, z, y, x)| Piece([x, y, z, w]))
        }
        fn stickers() -> &'static [Sticker] {
            Rubiks24::faces().iter().flat_map(|&face| face.stickers())
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

    lazy_static_generic_array_methods! {}

    fn face_symbols() -> &'static [&'static str] {
        &["I", "B", "D", "L", "R", "U", "F", "O"]
    }
    fn face_names() -> &'static [&'static str] {
        &["In", "Back", "Down", "Left", "Right", "Up", "Front", "Out"]
    }

    fn twist_direction_symbols() -> &'static [&'static str] {
        &["x", "x'", "y", "y'", "z", "z'"]
    }
    fn twist_direction_names() -> &'static [&'static str] {
        Self::twist_direction_symbols()
    }
}
impl Rubiks24 {
    fn transform_point(mut point: Vector4<f32>, p: StickerGeometryParams) -> Option<Point3<f32>> {
        // Compute the maximum extent along any axis from the origin in the 3D
        // projection of the puzzle. We will divide all XYZ coordinates by this
        // to normalize the puzzle size.
        let projection_radius = p
            .project_4d(cgmath::vec4(1.0, 0.0, 0.0, 1.0))?
            .distance(Point3::origin());

        point = p.model_transform * point;
        point /= PUZZLE_RADIUS;
        point.w /= 1.0 - p.face_spacing;
        Some(
            p.view_transform
                .transform_point(p.project_4d(point)? / projection_radius),
        )
    }
}

/// Piece location in a 2x2x2x2 Rubik's cube.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Piece(pub [Sign; 4]);
impl FacetTrait for Piece {
    impl_facet_trait_id_methods!(Piece, Rubiks24::pieces());

    fn projection_center(self, p: StickerGeometryParams) -> Option<Point3<f32>> {
        Rubiks24::transform_point(self.center_4d(p), p)
    }
}
impl PieceTrait<Rubiks24> for Piece {
    fn piece_type(self) -> PieceType {
        PieceType {
            ty: Rubiks24::TYPE,
            id: self.sticker_count() - 1,
        }
    }

    fn layer(self, face: Face) -> Option<usize> {
        match self[face.axis()] * face.sign() {
            Sign::Neg => Some(1),
            Sign::Zero => panic!("invalid piece"),
            Sign::Pos => Some(0),
        }
    }

    fn sticker_count(self) -> usize {
        4
    }
    fn stickers(self) -> Vec<Sticker> {
        Axis::iter()
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
        self[rhs.axis] = self[rhs.axis] + rhs.sign + rhs.sign;
        self
    }
}
impl Piece {
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
    /// Returns the X coordinate of this piece, in the range 0..=1.
    fn x_idx(self) -> usize {
        (self.x().int() + 1) as usize / 2
    }
    /// Returns the Y coordinate of this piece, in the range 0..=1.
    fn y_idx(self) -> usize {
        (self.y().int() + 1) as usize / 2
    }
    /// Returns the Z coordinate of this piece, in the range 0..=1.
    fn z_idx(self) -> usize {
        (self.z().int() + 1) as usize / 2
    }
    /// Returns the W coordinate of this piece, in the range 0..=1.
    fn w_idx(self) -> usize {
        (self.w().int() + 1) as usize / 2
    }

    fn center_4d(self, p: StickerGeometryParams) -> Vector4<f32> {
        let mut ret = Vector4::zero();
        for axis in Axis::iter() {
            ret[axis as usize] = p.face_scale(2.0) * self[axis].float() * 0.5;
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
    impl_facet_trait_id_methods!(Sticker, Rubiks24::stickers());

    fn projection_center(self, p: StickerGeometryParams) -> Option<Point3<f32>> {
        Rubiks24::transform_point(self.center_4d(p), p)
    }
}
impl StickerTrait<Rubiks24> for Sticker {
    fn piece(self) -> Piece {
        self.piece
    }
    fn face(self) -> Face {
        Face::new(self.axis(), self.sign())
    }

    fn geometry(self, p: StickerGeometryParams) -> Option<StickerGeometry> {
        let [ax1, ax2, ax3] = self.face().parallel_axes();

        // Compute the center of the sticker.
        let center = self.center_4d(p);

        // Add a radius to the sticker along each axis.
        let sticker_radius = p.face_scale(2.0) * p.sticker_scale() / 2.0;
        let get_corner = |v, u, t| {
            let mut vert = center;
            vert[ax1 as usize] += t * sticker_radius;
            vert[ax2 as usize] += u * sticker_radius;
            vert[ax3 as usize] += v * sticker_radius;
            Rubiks24::transform_point(vert, p)
        };
        let verts = [
            get_corner(-1.0, -1.0, -1.0)?,
            get_corner(-1.0, -1.0, 1.0)?,
            get_corner(-1.0, 1.0, -1.0)?,
            get_corner(-1.0, 1.0, 1.0)?,
            get_corner(1.0, -1.0, -1.0)?,
            get_corner(1.0, -1.0, 1.0)?,
            get_corner(1.0, 1.0, -1.0)?,
            get_corner(1.0, 1.0, 1.0)?,
        ];
        // Only show this sticker if the 3D volume is positive. (Cull it if its
        // 3D volume is negative.)
        Matrix3::from_cols(
            verts[1] - verts[0],
            verts[2] - verts[0],
            verts[4] - verts[0],
        )
        .determinant()
        .is_sign_positive()
        .then(|| StickerGeometry::new_cube(verts))
    }
}
impl Sticker {
    /// Returns the sticker on the given piece along the given axis. Panics if
    /// the given sticker does not exist.
    pub fn new(piece: Piece, axis: Axis) -> Self {
        for axis in Axis::iter() {
            assert!(piece[axis].is_nonzero(), "invalid piece");
        }
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
    fn adj_faces(self) -> [Face; 3] {
        let mut parity = Sign::Pos;
        let mut a = self.face().parallel_axes().map(|axis| {
            let sign = self.piece()[axis];
            parity = parity * sign;
            Face { axis, sign }
        });

        if parity == Sign::Neg {
            // This always performs a single swap.
            a.reverse();
        }

        a.try_into().unwrap()
    }
    /// Returns the 3D vector to the sticker from the center of its face.
    pub fn vec3_within_face(self) -> [Sign; 3] {
        let [ax1, ax2, ax3] = self.axis().sticker_order_perpendiculars();
        [self.piece()[ax1], self.piece()[ax2], self.piece()[ax3]]
    }

    fn center_4d(self, p: StickerGeometryParams) -> Vector4<f32> {
        let mut ret = self.piece().center_4d(p);
        ret[self.axis() as usize] = self.sign().float();
        ret
    }
}

/// Face of a 4D cube/cuboid.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Face {
    axis: Axis,
    sign: Sign,
}
impl From<rubiks34::Face> for Face {
    fn from(f: rubiks34::Face) -> Self {
        Self {
            axis: f.axis(),
            sign: f.sign(),
        }
    }
}
impl FacetTrait for Face {
    impl_facet_trait_id_methods!(Face, Rubiks24::faces());

    fn projection_center(self, p: StickerGeometryParams) -> Option<Point3<f32>> {
        let mut ret = Vector4::zero();
        ret[self.axis() as usize] = self.sign().float();
        Rubiks24::transform_point(ret, p)
    }
}
impl FaceTrait<Rubiks24> for Face {
    fn pieces(self, layer: usize) -> Vec<Piece> {
        let sign = match layer {
            0 => self.sign(),
            1 => -self.sign(),
            _ => return vec![],
        };
        let mut piece = Piece([sign; 4]);
        let [ax1, ax2, ax3] = self.axis.sticker_order_perpendiculars();

        let signs = [Sign::Neg, Sign::Pos];
        itertools::iproduct!(signs, signs, signs)
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
    pub sticker: rubiks34::Sticker,
    /// Direction to twist the face.
    pub direction: TwistDirection2D,
    /// Layer mask.
    pub layers: LayerMask,
}
impl fmt::Display for Twist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sticker_id = self.sticker.id();
        let direction = self.direction.sign().int();
        let layers =
            (self.layers[0] as u8) | ((self.layers[1] as u8) << 1) | ((self.layers[2] as u8) << 2);
        write!(f, "{},{},{}", sticker_id, direction, layers)
    }
}
impl FromStr for Twist {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let [s1, s2, s3]: [&str; 3] = s
            .split(',')
            .collect_vec()
            .try_into()
            .map_err(|_| "invalid twist")?;
        let sticker_id: usize = s1.parse().map_err(|_| "invalid sticker ID")?;
        let direction: isize = s2.parse().map_err(|_| "invalid direction")?;
        let layers: u32 = s3.parse().map_err(|_| "invalid layer mask")?;

        let sticker = rubiks34::Sticker::from_id(sticker_id).ok_or("invalid sticker ID")?;
        let direction = match direction {
            -1 => TwistDirection2D::CW,
            1 => TwistDirection2D::CCW,
            _ => return Err("invalid direction ID"),
        };
        let layers = LayerMask(layers);
        layers.validate::<Rubiks24>()?;

        Ok(Self {
            sticker,
            direction,
            layers,
        })
    }
}
impl TwistTrait<Rubiks24> for Twist {
    fn from_face_with_layers(
        face: Face,
        direction: &str,
        layers: LayerMask,
    ) -> Result<Twist, &'static str> {
        use Axis::*;
        use TwistDirection2D::*;

        let (axis, direction) = match direction {
            "x" => (X, CW),
            "x'" => (X, CCW),
            "y" => (Y, CW),
            "y'" => (Y, CCW),
            "z" => (Z, CW),
            "z'" => (Z, CCW),
            _ => return Err("invalid direction"),
        };
        layers.validate::<Rubiks24>()?;
        Self::by_3d_view(face, axis, direction, layers)
    }
    fn from_face_recenter(face: Face) -> Result<Twist, &'static str> {
        let twist34 =
            rubiks34::Twist::from_face_recenter(rubiks34::Face::new(face.axis(), face.sign()))?;
        Ok(Self {
            sticker: twist34.sticker,
            direction: twist34.direction,
            layers: LayerMask::default(),
        }
        .whole_cube())
    }
    fn from_sticker(
        sticker: Sticker,
        direction: TwistDirection2D,
        layers: LayerMask,
    ) -> Result<Twist, &'static str> {
        let sticker = rubiks34::Sticker::new(rubiks34::Piece(sticker.piece.0), sticker.axis);
        Ok(Self {
            sticker,
            direction,
            layers,
        })
    }
    fn from_rng() -> Self {
        let mut rng = rand::thread_rng();
        loop {
            if let Ok(ret) = Self::from_sticker(
                Rubiks24::stickers()[rng.gen_range(0..Rubiks24::stickers().len())],
                if rng.gen() {
                    TwistDirection2D::CW
                } else {
                    TwistDirection2D::CCW
                },
                LayerMask(rng.gen_range(1..((1 << Rubiks24::LAYER_COUNT) - 1))),
            ) {
                return ret;
            }
        }
    }

    fn model_transform(self, t: f32) -> Matrix4<f32> {
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
            rubiks34::StickerAdjFaces::_0 { .. } => Orientation::default(),

            rubiks34::StickerAdjFaces::_1 {
                adjacent: _,
                centered: [axis1, axis2],
            } => Orientation::rot90(axis1, axis2),

            rubiks34::StickerAdjFaces::_2 {
                adjacent: [face1, face2],
                centered,
            } => Orientation::rot180(face1.into(), face2.into(), centered),

            rubiks34::StickerAdjFaces::_3 {
                adjacent: [face1, face2, face3],
            } => Orientation::rot120(face3.into(), face2.into(), face1.into()),
        };

        // Reverse orientation if counterclockwise.
        match self.direction {
            TwistDirection2D::CW => rot,
            TwistDirection2D::CCW => rot.rev(),
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

    fn can_combine(self, previous: Option<Self>, metric: TwistMetric) -> bool {
        if self.is_whole_puzzle_rotation() {
            match metric {
                TwistMetric::Qstm | TwistMetric::Ftm | TwistMetric::Stm => true,
                TwistMetric::Etm => false,
            }
        } else if let Some(prev) = previous {
            match metric {
                TwistMetric::Qstm => false,
                TwistMetric::Ftm => {
                    self.sticker.face() == prev.sticker.face() && self.layers == prev.layers
                }
                TwistMetric::Stm => self.sticker.face() == prev.sticker.face(),
                TwistMetric::Etm => false,
            }
        } else {
            false
        }
    }
    fn is_whole_puzzle_rotation(self) -> bool {
        self.layers == LayerMask::all::<Rubiks24>()
    }
}
impl Twist {
    /// Constructs a twist of `face` around `axis`
    pub fn by_3d_view(
        face: Face,
        axis: Axis,
        direction: TwistDirection2D,
        layers: LayerMask,
    ) -> Result<Self, &'static str> {
        let twist_34 = rubiks34::Twist::by_3d_view(
            rubiks34::Face::new(face.axis(), face.sign()),
            axis,
            direction,
            layers,
        )?;
        twist_34.layers.validate::<Rubiks24>()?;
        Ok(Self {
            sticker: twist_34.sticker,
            direction: twist_34.direction,
            layers: twist_34.layers,
        })
    }

    /// Returns the number of repetitions of this twist required before the
    /// puzzle returns to the original state.
    fn symmetry_order(self) -> usize {
        match self.sticker.adj_faces() {
            rubiks34::StickerAdjFaces::_0 { .. } => 1, // invalid
            rubiks34::StickerAdjFaces::_1 { .. } => 4, // 90-degree rotation
            rubiks34::StickerAdjFaces::_2 { .. } => 2, // 180-degree rotation
            rubiks34::StickerAdjFaces::_3 { .. } => 3, // 120-degree rotation
        }
    }
    /// Make a whole cube rotation from this move.
    pub const fn whole_cube(mut self) -> Self {
        self.layers = LayerMask::all::<Rubiks24>();
        self
    }
    /// Twist different layers.
    pub fn layers(mut self, layers: LayerMask) -> Result<Self, &'static str> {
        layers.validate::<Rubiks24>()?;
        self.layers = layers;
        Ok(self)
    }
}

/// Orientation of a 4D cube (i.e. a single piece of a 4D cube/cuboid).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Orientation([Face; 4]);
impl OrientationTrait<Rubiks24> for Orientation {
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
        let mut ret = rhs;
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
        for axis in Axis::iter() {
            if self[axis].axis == rhs.axis {
                ret.axis = axis;
            }
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
}
