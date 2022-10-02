//! 3D Rubik's cube.

use cgmath::*;
use itertools::Itertools;
use ndpuzzle::math::{Matrix, Rotor};
use num_enum::FromPrimitive;
use smallvec::smallvec;
use std::ops::{Index, IndexMut, RangeInclusive};
use std::sync::Arc;
use strum::IntoEnumIterator;

use crate::util::{from_pt3, from_vec3};

use super::*;

pub const DEFAULT_LAYER_COUNT: u8 = 3;
pub const MIN_LAYER_COUNT: u8 = 1;
pub const MAX_LAYER_COUNT: u8 = 9;
pub const LAYER_COUNT_RANGE: RangeInclusive<u8> = MIN_LAYER_COUNT..=MAX_LAYER_COUNT;

#[derive(Debug)]
struct Rubiks3DInfo {
    piece_locations: Vec<[u8; 3]>,
}

pub fn puzzle_type(layer_count: u8) -> Arc<PuzzleType> {
    assert!(LAYER_COUNT_RANGE.contains(&layer_count));

    let mut pieces = vec![];
    let mut stickers = vec![];

    let full_range = (0..layer_count).collect_vec();
    let ends = [0, layer_count - 1];

    let center_coord = (layer_count % 2 == 0) as u8;
    let mut piece_types = (center_coord..=layer_count / 2)
        .flat_map(|y| {
            (center_coord..=y).map(move |x| PieceTypeEnum::from_offset([x, y, layer_count / 2]))
        })
        .collect_vec();
    piece_types.sort();

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
                        stickers.push(StickerInfo { piece, color: face });
                    }
                };
                push_sticker_if(x_max, FaceEnum::R.into());
                push_sticker_if(x_min, FaceEnum::L.into());
                push_sticker_if(y_max, FaceEnum::U.into());
                push_sticker_if(y_min, FaceEnum::D.into());
                push_sticker_if(z_max, FaceEnum::F.into());
                push_sticker_if(z_min, FaceEnum::B.into());

                let piece_type = {
                    // Compute the distance of each coordinate from the
                    // center. 0 = centered along axis (only exists for odd
                    // puzzles).
                    let center = (layer_count - 1) as f32 / 2.0;
                    let x = (x as f32 - center).abs().ceil() as u8;
                    let y = (y as f32 - center).abs().ceil() as u8;
                    let z = (z as f32 - center).abs().ceil() as u8;
                    PieceType(
                        piece_types
                            .iter()
                            .find_position(|&&p| p == PieceTypeEnum::from_offset([x, y, z]))
                            .map(|(i, _)| i)
                            .unwrap_or(0) as _, // shouldn't ever happen
                    )
                };

                piece_locations.push([x, y, z]);
                pieces.push(PieceInfo {
                    stickers: piece_stickers,
                    piece_type,
                })
            }
        }
    }

    let mut aliases = vec![];
    {
        use FaceEnum::*;
        let all_layers = LayerMask::all_layers(layer_count);
        aliases.push(("x".to_string(), Alias::AxisLayers(R.into(), all_layers)));
        aliases.push(("y".to_string(), Alias::AxisLayers(U.into(), all_layers)));
        aliases.push(("z".to_string(), Alias::AxisLayers(F.into(), all_layers)));

        if let Some(slice_layers) = LayerMask::slice_layers(layer_count) {
            aliases.push(("M".to_string(), Alias::AxisLayers(L.into(), slice_layers)));
            aliases.push(("E".to_string(), Alias::AxisLayers(D.into(), slice_layers)));
            aliases.push(("S".to_string(), Alias::AxisLayers(F.into(), slice_layers)));
        }

        if layer_count >= 4 {
            for f in FaceEnum::iter() {
                aliases.push((
                    f.symbol_lower().to_string(),
                    Alias::AxisLayers(f.into(), LayerMask(2)),
                ))
            }
        }
    }
    // Try to match longer aliases first.
    aliases.sort_by_key(|(s, _)| -(s.len() as isize));

    let notation = NotationScheme {
        axis_names: FaceEnum::iter()
            .map(|f| f.symbol_upper().to_string())
            .collect(),
        direction_names: TwistDirectionEnum::iter()
            .map(|dir| TwistDirectionName::Same(dir.symbol().to_string()))
            .collect(),
        block_suffix: Some("w".to_string()),
        aliases,
    };

    let shape = Arc::new(PuzzleShape {
        name: "Cube".to_string(),
        ndim: 3,
        faces: FaceEnum::iter().map(|f| f.info()).collect(),
    });

    let orientations = {
        use FaceEnum::{F, R, U};
        use TwistDirectionEnum::{CCW90, CW180, CW90};

        // Primitive rotations
        let x = (R, CW90);
        let x2 = (R, CW180);
        let xi = (R, CCW90);
        let y = (U, CW90);
        let y2 = (U, CW180);
        let yi = (U, CCW90);
        let z = (F, CW90);
        let z2 = (F, CW180);
        let zi = (F, CCW90);

        vec![
            // 90-degree rotations
            vec![x],
            vec![xi],
            vec![y],
            vec![yi],
            vec![z],
            vec![zi],
            // 180-degree face rotations
            vec![x2],
            vec![y2],
            vec![z2],
            // 180-degree edge rotations
            vec![x, y2],
            vec![z, x2],
            vec![y, z2],
            vec![x, z2],
            vec![z, y2],
            vec![y, x2],
            // 120-degree vertex rotations
            vec![x, y],
            vec![xi, yi],
            vec![z, y],
            vec![x, yi],
            vec![x, z],
            vec![y, zi],
            vec![y, x],
            vec![z, xi],
        ]
        .into_iter()
        .map(|rotations| {
            let rotor = rotations
                .into_iter()
                .fold(Rotor::identity(), |r, (face, dir)| {
                    face.twist_rotation(dir) * r
                });
            rotor
        })
        .collect_vec()
    };

    let twists = Arc::new(PuzzleTwists {
        name: "3-layer cubic".to_string(),
        axes: FaceEnum::iter()
            .map(|f| f.twist_axis_info(layer_count))
            .collect(),
        directions: TwistDirectionEnum::iter().map(|dir| dir.info()).collect(),
        orientations,
    });

    let info = Arc::new(Rubiks3DInfo { piece_locations });

    // It's not like we'll ever clear the cache anyway, so just leak it
    // and let us have the 'static lifetimes.
    Arc::new_cyclic(|this| PuzzleType {
        this: this.clone(),
        name: format!("{0}x{0}x{0}", layer_count),
        ndim: 3,
        shape,
        twists,

        family_name: "Rubiks3D".to_string(),
        projection_type: ProjectionType::_3D,
        radius: 3.0_f32.sqrt(),
        layer_count,

        pieces,
        stickers,
        piece_types: piece_types
            .into_iter()
            .map(|piece_type| PieceTypeInfo::new(piece_type.to_string()))
            .collect(),
        scramble_moves_count: 10 * layer_count as usize, // TODO pulled from thin air; probably insufficient for big cubes
        notation,
        new: Box::new(move |ty| {
            let piece_states = vec![PieceState::default(); ty.pieces.len()].into_boxed_slice();
            Box::new(Rubiks3D {
                ty,
                info: info.clone(),
                piece_states,
            })
        }),
    })
}

#[derive(Debug, Clone)]
pub struct Rubiks3D {
    ty: Arc<PuzzleType>,
    info: Arc<Rubiks3DInfo>,
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
    fn ty(&self) -> &Arc<PuzzleType> {
        &self.ty
    }
    fn clone_boxed(&self) -> Box<dyn PuzzleState> {
        Box::new(self.clone())
    }

    fn twist(&mut self, twist: Twist) -> Result<(), &'static str> {
        for piece in self.pieces_affected_by_twist(twist) {
            self[piece] = self[piece].twist(twist.axis.into(), twist.direction.into());
        }
        Ok(())
    }
    fn layer_from_twist_axis(&self, twist_axis: TwistAxis, piece: Piece) -> u8 {
        let face: FaceEnum = twist_axis.into();
        let face_coord = match face.sign() {
            Sign::Pos => self.ty.layer_count - 1,
            Sign::Neg => 0,
        };
        let piece_coord = self.piece_location(piece)[face.axis() as usize];
        u8::abs_diff(face_coord, piece_coord)
    }

    fn sticker_geometry(
        &self,
        sticker: Sticker,
        p: &StickerGeometryParams,
    ) -> Option<StickerGeometry> {
        let piece = self.ty.info(sticker).piece;
        let face = self.sticker_face(sticker);

        let mut transform = Matrix::EMPTY_IDENT;
        if let Some((twist, progress)) = p.twist_animation {
            if self.is_piece_affected_by_twist(twist, piece) {
                let twist_axis: FaceEnum = twist.axis.into();
                let twist_transform = twist_axis.twist_matrix(twist.direction.into(), progress);
                transform = twist_transform;
            }
        }

        // Compute the center of the sticker.
        let center = &transform * from_pt3(self.sticker_center_3d(sticker, p));

        // Compute the vectors that span the plane of the sticker.
        let [u_span_axis, v_span_axis] = face.parallel_axes();
        let u = &transform * from_vec3(u_span_axis.unit_vec3() * p.sticker_scale);
        let v = &transform * from_vec3(v_span_axis.unit_vec3() * p.sticker_scale);

        // Decide what twists should happen when the sticker is clicked.
        let cw_twist = Twist {
            axis: face.into(),
            direction: TwistDirectionEnum::CW90.into(),
            layers: LayerMask::default(),
        };
        let ccw_twist = self.ty.reverse_twist(cw_twist);
        let recenter = None;

        Some(StickerGeometry::new_double_quad(
            [
                p.project_4d(&(&center - &u) - &v)?,
                p.project_4d(&(&center - &u) + &v)?,
                p.project_4d(&(&center + &u) - &v)?,
                p.project_4d(&(&center + &u) + &v)?,
            ],
            ClickTwists {
                cw: Some(cw_twist),
                ccw: Some(ccw_twist),
                recenter,
            },
            p.show_frontfaces,
            p.show_backfaces,
        ))
    }

    fn is_solved(&self) -> bool {
        self.ty
            .stickers
            .iter()
            .enumerate()
            .all(|(i, sticker)| self.sticker_face(Sticker(i as _)) == sticker.color.into())
    }
}
impl Rubiks3D {
    fn piece_location(&self, piece: Piece) -> [u8; 3] {
        let piece_state = self[piece];
        let initial_location = self.info.piece_locations[piece.0 as usize];
        let mut ret = [0_u8; 3];
        for (i, axis) in Axis::iter().enumerate() {
            let r = piece_state[axis].axis() as usize;
            ret[r] = initial_location[i];
            if piece_state[axis].sign() == Sign::Neg {
                ret[r] = self.ty.layer_count - 1 - ret[r];
            }
        }
        ret
    }
    fn sticker_face(&self, sticker: Sticker) -> FaceEnum {
        let sticker_info = self.ty.info(sticker);
        let original_face: FaceEnum = sticker_info.color.into();
        let current_face = self[sticker_info.piece][original_face.axis()];
        match original_face.sign() {
            Sign::Pos => current_face,
            Sign::Neg => current_face.opposite(),
        }
    }

    fn piece_center_3d(&self, piece: Piece, p: &StickerGeometryParams) -> Point3<f32> {
        let pos = self.piece_location(piece);
        cgmath::point3(
            self.piece_center_coordinate(pos[0], p),
            self.piece_center_coordinate(pos[1], p),
            self.piece_center_coordinate(pos[2], p),
        )
    }
    fn sticker_center_3d(&self, sticker: Sticker, p: &StickerGeometryParams) -> Point3<f32> {
        let sticker_info = self.ty.info(sticker);
        let piece = sticker_info.piece;
        let mut ret = self.piece_center_3d(piece, p);

        let sticker_face = self.sticker_face(sticker);
        ret[sticker_face.axis() as usize] = sticker_face.sign().float();
        ret
    }

    fn piece_center_coordinate(&self, x: u8, p: &StickerGeometryParams) -> f32 {
        (2.0 * x as f32 - (self.ty.layer_count - 1) as f32) * p.sticker_grid_scale
    }
}

/// The facing directions of the X+, Y+, and Z+ stickers on this piece (assuming
/// it has those stickers).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PieceState([FaceEnum; 3]);
impl Default for PieceState {
    fn default() -> Self {
        use FaceEnum::*;

        Self([R, U, F])
    }
}
impl Index<Axis> for PieceState {
    type Output = FaceEnum;

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
                *face = ((*face as u8) ^ diff).into(); // Swap axes
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
    fn twist(self, face: FaceEnum, mut direction: TwistDirectionEnum) -> Self {
        use TwistDirectionEnum::*;

        if face.sign() == Sign::Neg {
            direction = direction.rev();
        }
        let [a, b] = face.axis().perpendiculars();
        match direction {
            CW90 => self.rotate(a, b),
            CCW90 => self.rotate(b, a),
            CW180 => self.mirror(a).mirror(b),
            CCW180 => self.mirror(a).mirror(b),
        }
    }
}

#[derive(EnumIter, FromPrimitive, Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
enum FaceEnum {
    #[default]
    R = 0,
    L = 1,
    U = 2,
    D = 3,
    F = 4,
    B = 5,
}
impl From<Face> for FaceEnum {
    fn from(Face(i): Face) -> Self {
        Self::from(i)
    }
}
impl From<FaceEnum> for Face {
    fn from(face: FaceEnum) -> Self {
        Self(face as _)
    }
}
impl From<TwistAxis> for FaceEnum {
    fn from(TwistAxis(i): TwistAxis) -> Self {
        Self::from(i)
    }
}
impl From<FaceEnum> for TwistAxis {
    fn from(face: FaceEnum) -> Self {
        Self(face as _)
    }
}
impl FaceEnum {
    fn info(self) -> FaceInfo {
        FaceInfo {
            name: self.name().to_owned(),
        }
    }
    fn twist_axis_info(self, layer_count: u8) -> TwistAxisInfo {
        TwistAxisInfo {
            symbol: self.symbol_upper_str().to_string(),
            layer_count,
            opposite: Some((
                self.opposite().into(),
                TwistDirectionEnum::iter()
                    .map(|dir| dir.rev().into())
                    .collect(),
            )),
        }
    }

    fn axis(self) -> Axis {
        use FaceEnum::*;

        match self {
            R | L => Axis::X,
            U | D => Axis::Y,
            F | B => Axis::Z,
        }
    }
    fn sign(self) -> Sign {
        use FaceEnum::*;

        match self {
            R | U | F => Sign::Pos,
            L | D | B => Sign::Neg,
        }
    }
    #[must_use]
    fn opposite(self) -> Self {
        use FaceEnum::*;

        match self {
            R => L,
            L => R,
            U => D,
            D => U,
            F => B,
            B => F,
        }
    }

    fn symbol_upper_str(self) -> &'static str {
        use FaceEnum::*;

        match self {
            R => "R",
            L => "L",
            U => "U",
            D => "D",
            F => "F",
            B => "B",
        }
    }
    fn symbol_upper(self) -> char {
        self.symbol_upper_str().chars().next().unwrap()
    }
    fn symbol_lower(self) -> char {
        self.symbol_upper().to_ascii_lowercase()
    }
    fn name(self) -> &'static str {
        use FaceEnum::*;

        match self {
            R => "Right",
            L => "Left",
            U => "Up",
            D => "Down",
            F => "Front",
            B => "Back",
        }
    }

    /// Returns the axes parallel to this face (all except the perpendicular
    /// axis).
    fn parallel_axes(self) -> [Axis; 2] {
        let [ax1, ax2] = self.axis().perpendiculars();
        match self.sign() {
            Sign::Neg => [ax2, ax1],
            Sign::Pos => [ax1, ax2],
        }
    }

    fn twist_rotation(self, direction: TwistDirectionEnum) -> Rotor {
        const X: u8 = 0;
        const Y: u8 = 1;
        const Z: u8 = 2;

        let angle = Rad::full_turn() * direction.sign().float() / direction.period() as f32;
        let plane = match self {
            FaceEnum::R => (Y, Z),
            FaceEnum::L => (Z, Y),
            FaceEnum::U => (Z, X),
            FaceEnum::D => (X, Z),
            FaceEnum::F => (X, Y),
            FaceEnum::B => (Y, X),
        };
        Rotor::from_angle_in_axis_plane(plane.0, plane.1, angle.0)
    }
    fn twist_matrix(self, direction: TwistDirectionEnum, progress: f32) -> Matrix {
        Rotor::identity()
            .slerp(&self.twist_rotation(direction), progress)
            .matrix()
    }
}

#[derive(EnumIter, FromPrimitive, Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
enum TwistDirectionEnum {
    #[default]
    CW90 = 0,
    CCW90 = 1,
    CW180 = 2,
    CCW180 = 3,
}
impl From<TwistDirectionEnum> for TwistDirection {
    fn from(direction: TwistDirectionEnum) -> Self {
        Self(direction as _)
    }
}
impl From<TwistDirection> for TwistDirectionEnum {
    fn from(TwistDirection(i): TwistDirection) -> Self {
        Self::from(i)
    }
}
impl TwistDirectionEnum {
    fn info(self) -> TwistDirectionInfo {
        use TwistDirectionEnum::*;

        TwistDirectionInfo {
            symbol: self.symbol().to_owned(),
            name: self.name().to_owned(),
            qtm: match self {
                CW90 | CCW90 => 1,
                CW180 | CCW180 => 2,
            },
            rev: match self {
                CW90 => CCW90.into(),
                CCW90 => CW90.into(),
                CW180 => CCW180.into(),
                CCW180 => CW180.into(),
            },
        }
    }

    fn symbol(self) -> &'static str {
        use TwistDirectionEnum::*;

        match self {
            CW90 => "",
            CCW90 => "'",
            CW180 => "2",
            CCW180 => "2'",
        }
    }
    fn name(self) -> &'static str {
        use TwistDirectionEnum::*;

        match self {
            CW90 => "CW",
            CCW90 => "CCW",
            CW180 => "180 CW",
            CCW180 => "180 CCW",
        }
    }

    fn period(self) -> usize {
        use TwistDirectionEnum::*;

        match self {
            CW90 | CCW90 => 4,
            CW180 | CCW180 => 2,
        }
    }
    fn sign(self) -> Sign {
        use TwistDirectionEnum::*;

        match self {
            CW90 | CW180 => Sign::Neg,
            CCW90 | CCW180 => Sign::Pos,
        }
    }
    fn rev(self) -> Self {
        Self::from(self as u8 ^ 1)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum PieceTypeEnum {
    Piece,
    Corner,
    Edge,
    Wing(u8),
    Center,
    TCenter(u8),
    XCenter(u8),
    Oblique(u8, u8),
}
impl ToString for PieceTypeEnum {
    fn to_string(&self) -> String {
        match self {
            Self::Piece => format!("piece"),
            Self::Corner => format!("corner"),
            Self::Edge => format!("edge"),
            Self::Wing(0) => format!("wing"),
            Self::Wing(x) => format!("wing ({x})"),
            Self::Center => format!("center"),
            Self::TCenter(0) => format!("T-center"),
            Self::TCenter(x) => format!("T-center ({x})"),
            Self::XCenter(0) => format!("X-center"),
            Self::XCenter(x) => format!("X-center ({x})"),
            Self::Oblique(0, 0) => format!("oblique"),
            Self::Oblique(x, y) => format!("oblique ({x},{y})"),
        }
    }
}
impl PieceTypeEnum {
    fn from_offset(mut coords: [u8; 3]) -> Self {
        coords.sort();
        let [min, med, max] = coords;
        if max == 0 {
            Self::Piece
        } else if min == max {
            Self::Corner
        } else if med == max {
            if min == 0 {
                Self::Edge
            } else {
                Self::Wing(if max < 3 { 0 } else { min })
            }
        } else if med == 0 {
            Self::Center
        } else if min == 0 {
            Self::TCenter(if max < 3 { 0 } else { med })
        } else if min == med {
            Self::XCenter(if max < 3 { 0 } else { med })
        } else {
            Self::Oblique(if max < 4 { 0 } else { min }, if max < 4 { 0 } else { med })
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
    fn perpendiculars(self) -> [Axis; 2] {
        use Axis::*;
        match self {
            X => [Z, Y], // X+ => rotate from Z+ to Y+.
            Y => [X, Z], // Y+ => rotate from X+ to Z+.
            Z => [Y, X], // Z+ => rotate from Y+ to X+.
        }
    }

    /// Returns an iterator over all axes.
    fn iter() -> impl Iterator<Item = Axis> {
        [Axis::X, Axis::Y, Axis::Z].into_iter()
    }

    /// Returns the unit vector along this axis.
    fn unit_vec3(self) -> Vector3<f32> {
        match self {
            Axis::X => Vector3::unit_x(),
            Axis::Y => Vector3::unit_y(),
            Axis::Z => Vector3::unit_z(),
        }
    }
}
