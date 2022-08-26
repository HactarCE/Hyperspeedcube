//! 4D Rubik's cube.

use cgmath::*;
use itertools::Itertools;
use num_enum::FromPrimitive;
use serde::{de::Error, Deserialize, Deserializer};
use smallvec::smallvec;
use std::collections::HashMap;
use std::ops::{Index, IndexMut, RangeInclusive};
use std::sync::Mutex;
use strum::IntoEnumIterator;

use super::*;

pub const DEFAULT_LAYER_COUNT: u8 = 3;
pub const MIN_LAYER_COUNT: u8 = 1;
pub const MAX_LAYER_COUNT: u8 = 9;
pub const LAYER_COUNT_RANGE: RangeInclusive<u8> = MIN_LAYER_COUNT..=MAX_LAYER_COUNT;

pub(super) fn deserialize_layer_count<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    let layer_count: u8 = Deserialize::deserialize(deserializer)?;
    if !LAYER_COUNT_RANGE.contains(&layer_count) {
        return Err(D::Error::custom(format!(
            "invalid layer count {layer_count}"
        )));
    }
    Ok(layer_count)
}

pub(super) fn puzzle_type(layer_count: u8) -> &'static dyn PuzzleType {
    puzzle_description(layer_count)
}

fn puzzle_description(layer_count: u8) -> &'static Rubiks4DDescription {
    lazy_static! {
        static ref CACHE: Mutex<HashMap<u8, &'static Rubiks4DDescription>> =
            Mutex::new(HashMap::new());
    }

    assert!(LAYER_COUNT_RANGE.contains(&layer_count));

    CACHE.lock().unwrap().entry(layer_count).or_insert_with(|| {
        let mut pieces = vec![];
        let mut stickers = vec![];

        let full_range = (0..layer_count).collect_vec();
        let ends = [0, layer_count - 1];

        let center_coord = (layer_count % 2 == 0) as u8;
        let mut piece_types = (center_coord..=layer_count / 2)
            .flat_map(|z| {
                (center_coord..=z).flat_map(move |y| {
                    (center_coord..=y)
                        .map(move |x| PieceTypeEnum::from_offset([x, y, z, layer_count / 2]))
                })
            })
            .collect_vec();
        piece_types.sort();

        let mut piece_locations = vec![];
        for w in 0..layer_count {
            let w_min = w == 0;
            let w_max = w == layer_count - 1;

            for z in 0..layer_count {
                let z_min = z == 0;
                let z_max = z == layer_count - 1;

                for y in 0..layer_count {
                    let y_min = y == 0;
                    let y_max = y == layer_count - 1;

                    let x_range = if w_min || w_max || z_min || z_max || y_min || y_max {
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
                        push_sticker_if(w_max, FaceEnum::O.into());
                        push_sticker_if(w_min, FaceEnum::I.into());

                        let piece_type = {
                            // Compute the distance of each coordinate from the
                            // center. 0 = centered along axis (only exists for odd
                            // puzzles).
                            let center = (layer_count - 1) as f32 / 2.0;
                            let x = (x as f32 - center).abs().ceil() as u8;
                            let y = (y as f32 - center).abs().ceil() as u8;
                            let z = (z as f32 - center).abs().ceil() as u8;
                            let w = (w as f32 - center).abs().ceil() as u8;
                            PieceType(
                                piece_types
                                    .iter()
                                    .find_position(|&&p| {
                                        p == PieceTypeEnum::from_offset([x, y, z, w])
                                    })
                                    .map(|(i, _)| i)
                                    .unwrap_or(0) as _, // shouldn't ever happen
                            )
                        };

                        piece_locations.push([x, y, z, w]);
                        pieces.push(PieceInfo {
                            stickers: piece_stickers,
                            piece_type,
                        })
                    }
                }
            }
        }

        let mut aliases = vec![];

        // Add slice twist aliases.
        if let Some(slice_layers) = LayerMask::slice_layers(layer_count) {
            use FaceEnum::*;

            aliases.push(("M".to_string(), Alias::AxisLayers(L.into(), slice_layers)));
            aliases.push(("E".to_string(), Alias::AxisLayers(D.into(), slice_layers)));
            aliases.push(("S".to_string(), Alias::AxisLayers(F.into(), slice_layers)));
            aliases.push(("P".to_string(), Alias::AxisLayers(O.into(), slice_layers)));
        }

        // Add 90-degree full-puzzle rotation aliases.
        let all_layers = LayerMask::all_layers(layer_count);
        for (ax1, ax2) in itertools::iproduct!(Axis::iter(), Axis::iter()) {
            if let Some((dir, face)) = TwistDirectionEnum::from_face_twist_plane(ax1, ax2) {
                let alias_string = format!("{}{}", ax1.symbol_lower(), ax2.symbol_lower());

                let mut twist = Twist {
                    axis: face.into(),
                    direction: dir.into(),
                    layers: all_layers,
                };
                aliases.push((alias_string.clone(), Alias::EntireTwist(twist)));

                twist.direction = dir.double().unwrap().into();
                aliases.push((alias_string + "2", Alias::EntireTwist(twist)));
            }
        }
        // Try to match longer aliases first.
        aliases.sort_by_key(|(s, _)| -(s.len() as isize));

        let notation = NotationScheme {
            axis_names: FaceEnum::iter()
                .map(|f| f.symbol_upper().to_string())
                .collect(),
            direction_names: TwistDirectionEnum::iter()
                .map(|dir| {
                    TwistDirectionName::PerAxis(
                        FaceEnum::iter().map(|f| dir.symbol_on_face(f)).collect(),
                    )
                })
                .collect(),
            block_suffix: None,
            aliases,
        };

        // It's not like we'll ever clear the cache anyway, so just leak it
        // and let us have the 'static lifetimes.
        Box::leak(Box::new(Rubiks4DDescription {
            name: format!("{0}x{0}x{0}x{0}", layer_count),

            layer_count,

            faces: FaceEnum::iter().map(|f| f.info()).collect(),
            pieces,
            stickers,
            twist_axes: FaceEnum::iter().map(|f| f.twist_axis_info()).collect(),
            twist_directions: TwistDirectionEnum::iter().map(|dir| dir.info()).collect(),
            piece_types: piece_types
                .into_iter()
                .map(|piece_type| PieceTypeInfo::new(piece_type.to_string()))
                .collect(),
            notation,

            piece_locations,
        }))
    })
}

#[derive(Debug, Clone)]
struct Rubiks4DDescription {
    name: String,

    layer_count: u8,

    faces: Vec<FaceInfo>,
    pieces: Vec<PieceInfo>,
    stickers: Vec<StickerInfo>,
    twist_axes: Vec<TwistAxisInfo>,
    twist_directions: Vec<TwistDirectionInfo>,
    piece_types: Vec<PieceTypeInfo>,
    notation: NotationScheme,

    piece_locations: Vec<[u8; 4]>,
}
impl PuzzleType for Rubiks4DDescription {
    fn ty(&self) -> PuzzleTypeEnum {
        PuzzleTypeEnum::Rubiks4D {
            layer_count: self.layer_count,
        }
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn family_display_name(&self) -> &'static str {
        "Rubik's 4D"
    }
    fn family_internal_name(&self) -> &'static str {
        "Rubiks4D"
    }
    fn projection_type(&self) -> ProjectionType {
        ProjectionType::_4D
    }

    fn layer_count(&self) -> u8 {
        self.layer_count
    }
    fn family_max_layer_count(&self) -> u8 {
        MAX_LAYER_COUNT
    }
    fn projection_radius_3d(&self, p: StickerGeometryParams) -> f32 {
        let r = 1.0 - p.face_spacing;
        let farthest_point = cgmath::vec4(1.0, r, r, r);
        match p.project_4d(farthest_point) {
            Some(farthest_point) => p
                .view_transform
                .transform_point(farthest_point)
                .distance(Point3::origin()),
            None => 3.0_f32.sqrt(), // shouldn't ever happen
        }
    }
    fn scramble_moves_count(&self) -> usize {
        15 * self.layer_count as usize // TODO pulled from thin air; probably insufficient for big cubes
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
    fn piece_types(&self) -> &[PieceTypeInfo] {
        &self.piece_types
    }

    fn opposite_twist_axis(&self, twist_axis: TwistAxis) -> Option<TwistAxis> {
        Some(FaceEnum::from(twist_axis).opposite().into())
    }
    fn count_quarter_turns(&self, twist: Twist) -> usize {
        use TwistDirectionEnum::*;

        match twist.direction.into() {
            R | L | U | D | F | B => 1,
            R2 | L2 | U2 | D2 | F2 | B2 => 2,
            UF | DB | UR | DL | FR | BL | DF | UB | UL | DR | BR | FL => 3,
            UFR | DBL | UFL | DBR | DFR | UBL | UBR | DFL => 2,
        }
    }

    fn make_recenter_twist(&self, axis: TwistAxis) -> Result<Twist, String> {
        use FaceEnum::*;
        use TwistDirectionEnum as Dir;

        let (axis, direction) = match axis.into() {
            R => (U, Dir::F),
            L => (U, Dir::B),
            U => (R, Dir::B),
            D => (R, Dir::F),
            F => (R, Dir::U),
            B => (R, Dir::D),
            O => return Err("cannot recenter near face".to_string()),
            I => return Err("cannot recenter far face".to_string()),
        };

        Ok(Twist {
            axis: axis.into(),
            direction: direction.into(),
            layers: self.all_layers(),
        })
    }

    fn canonicalize_twist(&self, twist: Twist) -> Twist {
        let mut face: FaceEnum = twist.axis.into();
        let mut direction: TwistDirectionEnum = twist.direction.into();
        let mut layers = twist.layers;

        let rev_layers = self.reverse_layers(twist.layers);
        let should_reverse = if Some(layers) == self.slice_layers() {
            use FaceEnum::*;
            // These are the faces that correspond to MESP slice twists.
            !matches!(face, L | D | F | O)
        } else {
            twist.layers.0 > rev_layers.0 || twist.layers == rev_layers && face.sign() == Sign::Neg
        };
        if should_reverse {
            face = face.opposite();
            direction = direction.mirror(face.axis());
            layers = rev_layers;
        }

        // Canonicalize full-puzzle rotations.
        if twist.layers == self.all_layers() {
            if let Some([ax1, ax2]) = direction.twist_plane_for_face(face) {
                if let Some((new_direction, new_face)) =
                    TwistDirectionEnum::from_face_twist_plane(ax1, ax2)
                {
                    let is_face_180 = direction.is_face_180();

                    face = new_face;
                    direction = new_direction;
                    if is_face_180 {
                        direction = direction.double().unwrap();
                    }
                }
            }
        }

        Twist {
            axis: face.into(),
            direction: direction.into(),
            layers,
        }
    }

    fn reverse_twist_direction(&self, mut direction: TwistDirection) -> TwistDirection {
        direction.0 ^= 1;
        direction
    }
    fn chain_twist_directions(&self, dirs: &[TwistDirection]) -> Option<TwistDirection> {
        match dirs {
            [] => None,
            [dir] => Some(*dir),
            _ => {
                // Apply all of `dirs` to a single hypothetical piece and see
                // which twist direction it ends up looking like at the end. If
                // it doesn't match any twist direction, it should match the
                // initial state.
                let face = FaceEnum::default();
                let final_state = dirs.iter().fold(PieceState::default(), |state, &dir| {
                    state.twist(face, dir.into())
                });

                match TwistDirectionEnum::from_piece_state_on_face(final_state, face) {
                    Some(dir) => Some(dir.into()),
                    None => {
                        debug_assert_eq!(final_state, PieceState::default());
                        None
                    }
                }
            }
        }
    }

    fn notation_scheme(&self) -> &NotationScheme {
        &self.notation
    }
}

#[derive(Debug, Clone)]
pub struct Rubiks4D {
    desc: &'static Rubiks4DDescription,
    piece_states: Box<[PieceState]>,
}
impl Eq for Rubiks4D {}
impl PartialEq for Rubiks4D {
    fn eq(&self, other: &Self) -> bool {
        self.piece_states == other.piece_states
    }
}
impl Index<Piece> for Rubiks4D {
    type Output = PieceState;

    fn index(&self, piece: Piece) -> &Self::Output {
        &self.piece_states[piece.0 as usize]
    }
}
impl IndexMut<Piece> for Rubiks4D {
    fn index_mut(&mut self, piece: Piece) -> &mut Self::Output {
        &mut self.piece_states[piece.0 as usize]
    }
}
impl PuzzleState for Rubiks4D {
    fn twist(&mut self, twist: Twist) -> Result<(), &'static str> {
        for piece in self.pieces_affected_by_twist(twist) {
            self[piece] = self[piece].twist(twist.axis.into(), twist.direction.into());
        }
        Ok(())
    }
    fn layer_from_twist_axis(&self, twist_axis: TwistAxis, piece: Piece) -> u8 {
        let face: FaceEnum = twist_axis.into();
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
        let face = self.sticker_face(sticker);

        let mut model_transform = Matrix4::identity();
        if let Some((twist, progress)) = p.twist_animation {
            if self.is_piece_affected_by_twist(twist, piece) {
                let twist_axis: FaceEnum = twist.axis.into();
                model_transform = twist_axis.twist_matrix(twist.direction.into(), progress);
            }
        }

        // Compute the center of the sticker.
        let center = model_transform * self.sticker_center_4d(sticker, p);

        // Compute the vectors that span the volume of the sticker.
        let Matrix4 { x, y, z, w: _ } = model_transform
            * face.basis_matrix()
            * p.sticker_scale
            // Invert outer face.
            * if face == FaceEnum::O { -1.0 } else { 1.0 };

        let project = |point_4d| Some(p.view_transform.transform_point(p.project_4d(point_4d)?));

        // Decide what twists should happen when the sticker is clicked.
        let mut twists: [ClickTwists; 6];
        {
            let sticker_signs = self.sticker_signs_within_face(sticker);
            let cw =
                TwistDirectionEnum::from_signs_within_face(sticker_signs).map(|twist_direction| {
                    Twist {
                        axis: face.into(),
                        direction: twist_direction.into(),
                        layers: LayerMask::default(),
                    }
                });
            let ccw = cw.map(|t| self.reverse_twist(t));
            let recenter = self.make_recenter_twist(face.into()).ok();
            twists = [ClickTwists { cw, ccw, recenter }; 6];
            // Replace corner twists with face twists on centermost pieces.
            if self.is_centermost_piece(piece) {
                let mut i = 0;
                for ax in [Axis::X, Axis::Y, Axis::Z] {
                    for sign in [-1, 1] {
                        if sticker_signs[ax as usize] == sign || self.layer_count() % 2 == 1 {
                            if let Some(new_dir) = TwistDirectionEnum::from_signs_within_face(
                                ax.unit_vec4().truncate() * sign,
                            ) {
                                twists[i].ccw = Some(Twist {
                                    axis: face.into(),
                                    direction: new_dir.rev().into(),
                                    layers: LayerMask::default(),
                                });
                                twists[i].cw = Some(Twist {
                                    axis: face.into(),
                                    direction: new_dir.into(),
                                    layers: LayerMask::default(),
                                });
                            }
                        }
                        i += 1;
                    }
                }
            }
        }

        StickerGeometry::new_cube(
            [
                project(center + -x + -y + -z)?,
                project(center + -x + -y + z)?,
                project(center + -x + y + -z)?,
                project(center + -x + y + z)?,
                project(center + x + -y + -z)?,
                project(center + x + -y + z)?,
                project(center + x + y + -z)?,
                project(center + x + y + z)?,
            ],
            twists,
        )
    }

    fn is_solved(&self) -> bool {
        self.stickers()
            .iter()
            .enumerate()
            .all(|(i, sticker)| self.sticker_face(Sticker(i as _)) == sticker.color.into())
    }
}
#[delegate_to_methods]
#[delegate(PuzzleType, target_ref = "desc")]
impl Rubiks4D {
    pub fn new(layer_count: u8) -> Self {
        let desc = puzzle_description(layer_count);
        let piece_states = vec![PieceState::default(); desc.pieces().len()].into_boxed_slice();
        Self { desc, piece_states }
    }

    fn desc(&self) -> &Rubiks4DDescription {
        self.desc
    }

    fn piece_location(&self, piece: Piece) -> [u8; 4] {
        let piece_state = self[piece];
        let initial_location = self.desc.piece_locations[piece.0 as usize];
        let mut ret = [0_u8; 4];
        for (i, axis) in Axis::iter().enumerate() {
            let r = piece_state[axis].axis() as usize;
            ret[r] = initial_location[i];
            if piece_state[axis].sign() == Sign::Neg {
                ret[r] = self.layer_count() - 1 - ret[r];
            }
        }
        ret
    }
    fn piece_location_from_center(&self, piece: Piece) -> [i8; 4] {
        let center = (self.layer_count() - 1) as f32 / 2.0;
        self.piece_location(piece)
            .map(|x| (x as f32 - center).round() as i8)
    }
    fn piece_location_signs(&self, piece: Piece) -> Vector4<i8> {
        let mut coords = self.piece_location_from_center(piece);
        let [x, y, z, w] = coords;
        coords.sort_by_key(|x| x.abs());
        let hi_coord = coords[2].abs();

        if hi_coord == 0 {
            Vector4::zero() // don't divide by zero
        } else {
            cgmath::vec4(x, y, z, w) / hi_coord
        }
    }
    fn is_centermost_piece(&self, piece: Piece) -> bool {
        let mut coords = self.piece_location_from_center(piece);
        coords.sort_by_key(|x| x.abs());
        let hi = coords[2];
        if self.layer_count() % 2 == 0 {
            hi.abs() == 1
        } else {
            hi == 0
        }
    }
    fn sticker_signs_within_face(&self, sticker: Sticker) -> Vector3<i8> {
        let face = self.sticker_face(sticker);
        let piece_loc_signs = self.piece_location_signs(self.info(sticker).piece);
        Self::signs_within_face(face, piece_loc_signs)
    }
    fn signs_within_face(face: FaceEnum, piece_loc_signs: Vector4<i8>) -> Vector3<i8> {
        let [basis1, basis2, basis3] = face.basis();
        cgmath::vec3(
            piece_loc_signs.dot(basis1.cast().unwrap()),
            piece_loc_signs.dot(basis2.cast().unwrap()),
            piece_loc_signs.dot(basis3.cast().unwrap()),
        )
    }
    fn sticker_face(&self, sticker: Sticker) -> FaceEnum {
        let sticker_info = self.info(sticker);
        let original_face: FaceEnum = sticker_info.color.into();
        let current_face = self[sticker_info.piece][original_face.axis()];
        match original_face.sign() {
            Sign::Pos => current_face,
            Sign::Neg => current_face.opposite(),
        }
    }

    fn piece_center_4d(&self, piece: Piece, p: StickerGeometryParams) -> Vector4<f32> {
        let pos = self.piece_location(piece);
        cgmath::vec4(
            self.piece_center_coordinate(pos[0], p),
            self.piece_center_coordinate(pos[1], p),
            self.piece_center_coordinate(pos[2], p),
            self.piece_center_coordinate(pos[3], p),
        )
    }
    fn sticker_center_4d(&self, sticker: Sticker, p: StickerGeometryParams) -> Vector4<f32> {
        let sticker_info = self.info(sticker);
        let piece = sticker_info.piece;
        let mut ret = self.piece_center_4d(piece, p);

        let sticker_face = self.sticker_face(sticker);
        ret[sticker_face.axis() as usize] = sticker_face.sign().float();
        ret
    }

    fn piece_center_coordinate(&self, x: u8, p: StickerGeometryParams) -> f32 {
        (2.0 * x as f32 - (self.layer_count() - 1) as f32) * p.sticker_grid_scale
    }

    pub fn to_mc4d_twist_string(mut twist: Twist) -> String {
        lazy_static! {
            static ref MC4D_TWIST_IDS: HashMap<(TwistAxis, TwistDirection), usize> =
                Rubiks4D::mc4d_twist_order()
                    .enumerate()
                    .filter_map(|(i, twist)| Some((twist?, i)))
                    .collect();
        }

        let dir: TwistDirectionEnum = twist.direction.into();
        if let Some(quarter_turn) = dir.half() {
            twist.direction = quarter_turn.into();
            return format!("{0} {0}", Self::to_mc4d_twist_string(twist));
        }
        let sticker_id = MC4D_TWIST_IDS[&(twist.axis, twist.direction)];
        let direction_id = 1;
        let layer_mask = twist.layers.0;
        format!("{sticker_id},{direction_id},{layer_mask}")
    }
    pub fn from_mc4d_twist_string(s: &str) -> Option<Twist> {
        lazy_static! {
            static ref MC4D_TWISTS: Vec<Option<(TwistAxis, TwistDirection)>> =
                Rubiks4D::mc4d_twist_order().collect();
        }

        let mut segments = s.split(',');

        let (axis, direction) = (*MC4D_TWISTS.get(segments.next()?.parse::<usize>().ok()?)?)?;
        let direction: TwistDirectionEnum = direction.into();
        let direction = match segments.next()?.parse::<i8>().ok()? {
            1 => direction.rev(),
            2 => direction.rev().double()?,
            -1 => direction,
            -2 => direction.double()?,
            _ => return None,
        };
        let layers = LayerMask(segments.next()?.parse().ok()?);
        if segments.next().is_some() {
            return None;
        }
        Some(Twist {
            axis,
            direction: direction.into(),
            layers,
        })
    }
    fn mc4d_twist_order() -> impl Iterator<Item = Option<(TwistAxis, TwistDirection)>> {
        use FaceEnum::*;

        [I, B, D, L, R, U, F, O].into_iter().flat_map(|face| {
            let mut basis = face.basis_faces();
            basis.sort_by_key(|f| f.axis()); // order: X, Y, Z, W
            basis.reverse(); // order: W, Z, Y, X
            let mc4d_basis_1 = basis[0].axis().unit_vec4();
            let mc4d_basis_2 = basis[1].axis().unit_vec4();
            let mc4d_basis_3 = basis[2].axis().unit_vec4();

            let piece_locations = itertools::iproduct!([-1, 0, 1], [-1, 0, 1], [-1, 0, 1])
                .map(|(x, y, z)| Vector3::new(x, y, z));
            let corners = piece_locations.clone().filter(|v| v.magnitude2() == 3);
            let edges = piece_locations.clone().filter(|v| v.magnitude2() == 2);
            let centers = piece_locations.filter(|v| v.magnitude2() == 1);
            let core = std::iter::once(Vector3::zero());
            let mc4d_order_piece_locations = corners.chain(edges).chain(centers).chain(core);

            mc4d_order_piece_locations
                .map(move |mc4d_coords_of_sticker_within_face: Vector3<i8>| {
                    let offset = Vector4::zero()
                        + mc4d_basis_1 * mc4d_coords_of_sticker_within_face.x
                        + mc4d_basis_2 * mc4d_coords_of_sticker_within_face.y
                        + mc4d_basis_3 * mc4d_coords_of_sticker_within_face.z;
                    TwistDirectionEnum::from_signs_within_face(Self::signs_within_face(
                        face,
                        match face {
                            O => -offset, // not sure why this is necessary, but it is
                            _ => offset,
                        },
                    ))
                })
                .map(move |twist_dir| Some((face.into(), twist_dir?.into())))
        })
    }
}

/// The facing directions of the X+, Y+, Z+, and W+ stickers on this piece
/// (assuming it has those stickers).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PieceState([FaceEnum; 4]);
impl Default for PieceState {
    fn default() -> Self {
        use FaceEnum::*;

        Self([R, U, F, O])
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
    fn rotate_by_faces(self, from: FaceEnum, to: FaceEnum) -> Self {
        if from.sign() == to.sign() {
            self.rotate(from.axis(), to.axis())
        } else {
            self.rotate(to.axis(), from.axis())
        }
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
    fn twist(mut self, face: FaceEnum, direction: TwistDirectionEnum) -> Self {
        let [basis_x, basis_y, basis_z] = face.basis_faces();

        let mut chars = direction.symbol_xyz().chars().peekable();

        loop {
            let [mut a, mut b] = match chars.next() {
                None => return self,
                Some('x') => [basis_z, basis_y],
                Some('y') => [basis_x, basis_z],
                Some('z') => [basis_y, basis_x],
                _ => unreachable!(),
            };
            let double = chars.next_if_eq(&'2').is_some();
            let inverse = chars.next_if_eq(&'\'').is_some();
            if inverse {
                std::mem::swap(&mut a, &mut b);
            }
            self = self.rotate_by_faces(a, b);
            if double {
                self = self.rotate_by_faces(a, b);
            }
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
    O = 6,
    I = 7,
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
            symbol: self.symbol_upper_str(),
            name: self.name(),
        }
    }
    fn twist_axis_info(self) -> TwistAxisInfo {
        TwistAxisInfo {
            name: self.symbol_upper_str(),
        }
    }

    fn axis(self) -> Axis {
        use FaceEnum::*;

        match self {
            R | L => Axis::X,
            U | D => Axis::Y,
            F | B => Axis::Z,
            O | I => Axis::W,
        }
    }
    fn sign(self) -> Sign {
        use FaceEnum::*;

        match self {
            R | U | F | O => Sign::Pos,
            L | D | B | I => Sign::Neg,
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
            O => I,
            I => O,
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
            O => "O",
            I => "I",
        }
    }
    fn symbol_upper(self) -> char {
        self.symbol_upper_str().chars().next().unwrap()
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
            O => "Out",
            I => "In",
        }
    }

    fn vector(self) -> Vector4<f32> {
        (match self.axis() {
            Axis::X => Vector4::unit_x(),
            Axis::Y => Vector4::unit_y(),
            Axis::Z => Vector4::unit_z(),
            Axis::W => Vector4::unit_w(),
        } * self.sign().float())
    }

    fn basis_faces(self) -> [FaceEnum; 3] {
        use Axis::*;
        use FaceEnum::*;

        let w = match self.sign() {
            Sign::Pos => O,
            Sign::Neg => I,
        };

        [
            if self.axis() == X { w } else { R },
            if self.axis() == Y { w } else { U },
            if self.axis() == Z { w } else { F },
        ]
    }
    fn basis(self) -> [Vector4<f32>; 3] {
        self.basis_faces().map(|f| f.vector())
    }
    fn basis_matrix(self) -> Matrix4<f32> {
        let [x, y, z] = self.basis();
        let w = Vector4::zero();
        // This should be a 4x3 matrix, not 4x4.
        Matrix4 { x, y, z, w }
    }

    fn twist_matrix(self, direction: TwistDirectionEnum, progress: f32) -> Matrix4<f32> {
        let angle = Rad::full_turn() / direction.period() as f32 * progress;
        let mat3 = Matrix3::from_axis_angle(direction.vector3_f32().normalize(), -angle);
        let mut ret = Matrix4::identity();
        let basis = self.basis_faces();
        for i in 0..3 {
            for j in 0..3 {
                ret[basis[i].axis() as usize][basis[j].axis() as usize] =
                    mat3[i][j] * basis[i].sign().float() * basis[j].sign().float();
            }
        }
        ret
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(EnumIter, FromPrimitive, Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
enum TwistDirectionEnum {
    /// 90-degree face (2c) twist clockwise around `R`
    #[default]
    R,
    /// 90-degree face (2c) twist clockwise around `L`
    L,
    /// 90-degree face (2c) twist clockwise around `U`
    U,
    /// 90-degree face (2c) twist clockwise around `D`
    D,
    /// 90-degree face (2c) twist clockwise around `F`
    F,
    /// 90-degree face (2c) twist clockwise around `B`
    B,

    /// 180-degree face (2c) twist clockwise around `R`
    R2,
    /// 180-degree face (2c) twist clockwise around `L`
    L2,
    /// 180-degree face (2c) twist clockwise around `U`
    U2,
    /// 180-degree face (2c) twist clockwise around `D`
    D2,
    /// 180-degree face (2c) twist clockwise around `F`
    F2,
    /// 180-degree face (2c) twist clockwise around `B`
    B2,

    /// 180-degree edge (3c) twist clockwise around `UF`
    UF,
    /// 180-degree edge (3c) twist clockwise around `DB`
    DB,
    /// 180-degree edge (3c) twist clockwise around `UR`
    UR,
    /// 180-degree edge (3c) twist clockwise around `DL`
    DL,
    /// 180-degree edge (3c) twist clockwise around `FR`
    FR,
    /// 180-degree edge (3c) twist clockwise around `BL`
    BL,
    /// 180-degree edge (3c) twist clockwise around `DF`
    DF,
    /// 180-degree edge (3c) twist clockwise around `UB`
    UB,
    /// 180-degree edge (3c) twist clockwise around `UL`
    UL,
    /// 180-degree edge (3c) twist clockwise around `DR`
    DR,
    /// 180-degree edge (3c) twist clockwise around `BR`
    BR,
    /// 180-degree edge (3c) twist clockwise around `FL`
    FL,

    /// 120-degree corner (4c) twist clockwise around `UFR`
    UFR,
    /// 120-degree corner (4c) twist clockwise around `DBL`
    DBL,
    /// 120-degree corner (4c) twist clockwise around `UFL`
    UFL,
    /// 120-degree corner (4c) twist clockwise around `DBR` (equivalent: z'x)
    DBR,
    /// 120-degree corner (4c) twist clockwise around `DFR`
    DFR,
    /// 120-degree corner (4c) twist clockwise around `UBL` (equivalent: z'y)
    UBL,
    /// 120-degree corner (4c) twist clockwise around `UBR`
    UBR,
    /// 120-degree corner (4c) twist clockwise around `DFL` (equivalent: y'z)
    DFL,
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
        TwistDirectionInfo {
            symbol: self.symbol_xyz(),
            name: self.name(),
        }
    }

    fn symbol_xyz(self) -> &'static str {
        use TwistDirectionEnum::*;

        match self {
            R => "x",
            L => "x'",
            U => "y",
            D => "y'",
            F => "z",
            B => "z'",

            R2 => "x2",
            L2 => "x2'",
            U2 => "y2",
            D2 => "y2'",
            F2 => "z2",
            B2 => "z2'",

            UF => "xy2",
            DB => "xy2'",
            UR => "zx2",
            DL => "zx2'",
            FR => "yz2",
            BL => "yz2'",
            DF => "xz2",
            UB => "xz2'",
            UL => "zy2",
            DR => "zy2'",
            BR => "yx2",
            FL => "yx2'",

            UFR => "xy",
            DBL => "y'x'",
            UFL => "zy",
            DBR => "xy'", // (equivalent: z'x)
            DFR => "xz",
            UBL => "yz'", // (equivalent: z'y)
            UBR => "yx",
            DFL => "zx'", // (equivalent: y'z)
        }
    }
    fn name(self) -> &'static str {
        self.symbol_xyz()
    }
    fn symbol_on_face(self, face: FaceEnum) -> String {
        if face == FaceEnum::O {
            return self.rev().symbol_on_face(FaceEnum::I);
        }

        let vector4 = (face.basis_matrix() * self.vector3_f32().extend(0.0))
            .cast::<i8>()
            .unwrap();
        fn select_face_char(x: i8, char_pos: &'static str, char_neg: &'static str) -> &'static str {
            match x {
                1 => char_pos,
                -1 => char_neg,
                _ => "",
            }
        }

        // "UFRO" is the most natural-sounding order IMO.
        String::new()
            + select_face_char(vector4.y, "U", "D")
            + select_face_char(vector4.z, "F", "B")
            + select_face_char(vector4.x, "R", "L")
            + select_face_char(vector4.w, "O", "I")
            + if self.is_face_180() { "2" } else { "" }
    }

    fn period(self) -> usize {
        use TwistDirectionEnum::*;

        match self {
            // 90-degree face (2c) twists.
            R | L | U | D | F | B => 4,
            // 180-degree face (2c) twists.
            R2 | L2 | U2 | D2 | F2 | B2 => 2,
            // 180-degree edge (3c) twists.
            UF | DB | UR | DL | FR | BL | DF | UB | UL | DR | BR | FL => 2,
            // 120-degree corner (4c) twists.
            UFR | DBL | UFL | DBR | DFR | UBL | UBR | DFL => 3,
        }
    }
    fn rev(self) -> Self {
        Self::from(self as u8 ^ 1)
    }

    fn mirror(self, axis: Axis) -> Self {
        if axis == Axis::W {
            return self;
        }
        let mut v = self.vector3();
        v *= -1;
        v[axis as usize] *= -1;
        let ret = Self::from_signs_within_face(v).unwrap();
        if self.is_face_180() {
            ret.double().unwrap()
        } else {
            ret
        }
    }

    fn half(self) -> Option<Self> {
        use TwistDirectionEnum::*;

        match self {
            R2 | L2 | U2 | D2 | F2 | B2 => Some(Self::from(self as u8 - 6)),
            _ => None,
        }
    }
    fn is_face_180(self) -> bool {
        use TwistDirectionEnum::*;

        matches!(self, R2 | L2 | U2 | D2 | F2 | B2)
    }
    fn double(self) -> Option<Self> {
        use TwistDirectionEnum::*;

        match self {
            R | L | U | D | F | B => Some(Self::from(self as u8 + 6)),
            R2 | L2 | U2 | D2 | F2 | B2 => None,
            UF | DB | UR | DL | FR | BL | DF | UB | UL | DR | BR | FL => None,
            UFR | DBL | UFL | DBR | DFR | UBL | UBR | DFL => Some(self.rev()),
        }
    }

    fn vector3(self) -> Vector3<i8> {
        use TwistDirectionEnum::*;

        let x = match self {
            R | R2 | UR | FR | DR | BR | UFR | DBR | DFR | UBR => 1, // R
            L | L2 | UL | FL | DL | BL | UFL | DBL | DFL | UBL => -1, // L
            U | D | F | B | U2 | D2 | F2 | B2 | UF | DB | DF | UB => 0,
        };
        let y = match self {
            U | U2 | UF | UR | UB | UL | UFR | UFL | UBL | UBR => 1, // U
            D | D2 | DF | DR | DB | DL | DFR | DFL | DBL | DBR => -1, // D
            R | L | F | B | R2 | L2 | F2 | B2 | FR | BL | BR | FL => 0,
        };
        let z = match self {
            F | F2 | UF | FR | DF | FL | UFR | UFL | DFR | DFL => 1, // F
            B | B2 | UB | BR | DB | BL | UBR | UBL | DBR | DBL => -1, // B
            R | L | U | D | R2 | L2 | U2 | D2 | UR | DL | UL | DR => 0,
        };

        vec3(x, y, z)
    }
    fn vector3_f32(self) -> Vector3<f32> {
        self.vector3().cast().unwrap()
    }
    fn from_signs_within_face(v: Vector3<i8>) -> Option<Self> {
        use TwistDirectionEnum::*;

        match [v.x, v.y, v.z] {
            [1, 1, 1] => Some(UFR),
            [-1, 1, 1] => Some(UFL),
            [1, -1, 1] => Some(DFR),
            [-1, -1, 1] => Some(DFL),
            [1, 1, -1] => Some(UBR),
            [-1, 1, -1] => Some(UBL),
            [1, -1, -1] => Some(DBR),
            [-1, -1, -1] => Some(DBL),

            [1, 1, 0] => Some(UR),
            [-1, 1, 0] => Some(UL),
            [1, -1, 0] => Some(DR),
            [-1, -1, 0] => Some(DL),
            [1, 0, 1] => Some(FR),
            [-1, 0, 1] => Some(FL),
            [1, 0, -1] => Some(BR),
            [-1, 0, -1] => Some(BL),
            [0, 1, 1] => Some(UF),
            [0, -1, 1] => Some(DF),
            [0, 1, -1] => Some(UB),
            [0, -1, -1] => Some(DB),

            [1, 0, 0] => Some(R),
            [-1, 0, 0] => Some(L),
            [0, 1, 0] => Some(U),
            [0, -1, 0] => Some(D),
            [0, 0, 1] => Some(F),
            [0, 0, -1] => Some(B),

            _ => None,
        }
    }

    fn from_piece_state_on_face(piece_state: PieceState, face: FaceEnum) -> Option<Self> {
        lazy_static! {
            static ref RESULT_OF_SINGLE_TWIST: HashMap<(PieceState, FaceEnum), TwistDirectionEnum> =
                itertools::iproduct!(FaceEnum::iter(), TwistDirectionEnum::iter())
                    .map(|(face, dir)| {
                        let result = PieceState::default().twist(face, dir);
                        ((result, face), dir)
                    })
                    .collect();
        }

        RESULT_OF_SINGLE_TWIST.get(&(piece_state, face)).copied()
    }

    fn twist_plane_for_face(self, basis_face: FaceEnum) -> Option<[Axis; 2]> {
        use TwistDirectionEnum::*;

        let [x, y, z] = basis_face.basis_faces();
        let [face1, face2] = match self {
            R | R2 => [z, y],
            L | L2 => [y, z],
            U | U2 => [x, z],
            D | D2 => [z, x],
            F | F2 => [y, x],
            B | B2 => [x, y],
            _ => return None,
        };
        Some(match face1.sign() * face2.sign() {
            Sign::Pos => [face1.axis(), face2.axis()],
            Sign::Neg => [face2.axis(), face1.axis()],
        })
    }
    fn from_face_twist_plane(ax1: Axis, ax2: Axis) -> Option<(Self, FaceEnum)> {
        use TwistDirectionEnum::*;

        let basis_face = if ax1 != Axis::X && ax2 != Axis::X {
            FaceEnum::R
        } else if ax1 != Axis::Y && ax2 != Axis::Y {
            FaceEnum::U
        } else {
            FaceEnum::F
        };

        let direction = [R, L, U, D, F, B]
            .into_iter()
            .find(|dir| dir.twist_plane_for_face(basis_face) == Some([ax1, ax2]))?;

        Some((direction, basis_face))
    }

    fn from_xyz_chars(s: &str) -> Result<Option<Self>, String> {
        use TwistDirectionEnum::*;

        let face = FaceEnum::default();
        let mut chars = s.chars().peekable();
        let mut piece_state = PieceState::default();
        loop {
            let mut dir = match chars.next() {
                Some('x') => R,
                Some('y') => U,
                Some('z') => F,
                Some(c) => return Err(format!("unknown twist character: {c:?}")),
                None => break,
            };
            if chars.next_if_eq(&'2').is_some() {
                dir = dir.double().unwrap(); // never fails for R/U/F
            }
            if chars.next_if_eq(&'\'').is_some() {
                dir = dir.rev();
            }

            piece_state = piece_state.twist(face, dir);
        }

        Ok(Self::from_piece_state_on_face(piece_state, face))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum PieceTypeEnum {
    Piece,
    Corner,
    Edge,
    Wing(u8),
    Ridge,
    TRidge(u8),
    XRidge(u8),
    ObliqueRidge(u8, u8),
    Center,
    TCenter(u8),
    XCenter(u8),
    YCenter(u8, u8),
    SemiOblique(u8, u8, u8),
    Oblique(u8, u8, u8),
}
impl ToString for PieceTypeEnum {
    fn to_string(&self) -> String {
        match self {
            Self::Piece => format!("piece"),
            Self::Corner => format!("corner"),
            Self::Edge => format!("edge"),
            Self::Wing(0) => format!("wing"),
            Self::Wing(x) => format!("wing ({x})"),
            Self::Ridge => format!("ridge"),
            Self::TRidge(0) => format!("T-ridge"),
            Self::TRidge(x) => format!("T-ridge ({x})"),
            Self::XRidge(0) => format!("X-ridge"),
            Self::XRidge(x) => format!("X-ridge ({x})"),
            Self::ObliqueRidge(0, 0) => format!("oblique ridge"),
            Self::ObliqueRidge(x, y) => format!("oblique ridge ({x},{y})"),
            Self::Center => format!("center"),
            Self::TCenter(0) => format!("T-center"),
            Self::TCenter(x) => format!("T-center ({x})"),
            Self::XCenter(0) => format!("X-center"),
            Self::XCenter(x) => format!("X-center ({x})"),
            Self::YCenter(0, 0) => format!("Y-center"),
            Self::YCenter(x, y) => format!("Y-center ({x},{y})"),
            Self::SemiOblique(0, 0, 0) => format!("semi-oblique"),
            Self::SemiOblique(x, y, z) => format!("semi-oblique ({x},{y},{z})"),
            Self::Oblique(0, 0, 0) => format!("oblique"),
            Self::Oblique(x, y, z) => format!("oblique ({x},{y},{z})"),
        }
    }
}
impl PieceTypeEnum {
    fn from_offset(mut coords: [u8; 4]) -> Self {
        coords.sort();
        let [min, lo, hi, max] = coords;
        if max == 0 {
            Self::Piece
        } else if min == max {
            Self::Corner
        } else if lo == max {
            if min == 0 {
                Self::Edge
            } else if max < 3 {
                Self::Wing(0)
            } else {
                Self::Wing(min)
            }
        } else if hi == max {
            if lo == 0 {
                Self::Ridge
            } else if min == 0 {
                Self::TRidge(if max < 3 { 0 } else { lo })
            } else if min == lo {
                Self::XRidge(if max < 3 { 0 } else { lo })
            } else {
                if max < 4 {
                    Self::ObliqueRidge(0, 0)
                } else {
                    Self::ObliqueRidge(min, lo)
                }
            }
        } else if hi == 0 {
            Self::Center
        } else if lo == 0 {
            Self::TCenter(if max < 3 { 0 } else { hi })
        } else if min == 0 {
            if max < 3 {
                Self::YCenter(0, 0)
            } else {
                Self::YCenter(lo, hi)
            }
        } else if min == hi {
            Self::XCenter(if max < 3 { 0 } else { lo })
        } else if min == lo || lo == hi {
            Self::SemiOblique(min, lo, hi)
        } else {
            if max < 5 {
                Self::Oblique(0, 0, 0)
            } else {
                Self::Oblique(min, lo, hi)
            }
        }
    }
}

/// 4-dimensional axis.
#[derive(EnumIter, Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Axis {
    /// X axis (right).
    X = 0,
    /// Y axis (up).
    Y = 1,
    /// Z axis (towards the 3D camera).
    Z = 2,
    /// Z axis (towards the 4D camera).
    W = 3,
}
impl Axis {
    fn symbol_lower(self) -> char {
        match self {
            Axis::X => 'x',
            Axis::Y => 'y',
            Axis::Z => 'z',
            Axis::W => 'w',
        }
    }

    /// Returns the unit vector along this axis.
    fn unit_vec4<S: BaseNum>(self) -> Vector4<S> {
        match self {
            Axis::X => Vector4::unit_x(),
            Axis::Y => Vector4::unit_y(),
            Axis::Z => Vector4::unit_z(),
            Axis::W => Vector4::unit_w(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_rubiks_4d_twist_canonicalization() {
        for layer_count in 1..=4 {
            let p = Rubiks4D::new(layer_count);
            let are_twists_eq = |twist1, twist2| {
                twist_comparison_key(&p, twist1) == twist_comparison_key(&p, twist2)
            };
            crate::puzzle::tests::test_twist_canonicalization(&p, are_twists_eq);
        }
    }

    #[test]
    fn test_rubiks_4d_twist_serialization() {
        for layer_count in 1..=4 {
            let p = Rubiks4D::new(layer_count);
            crate::puzzle::tests::test_twist_serialization(&p);
        }

        for layer_count in 1..=7 {
            let p = Rubiks4D::new(layer_count);
            crate::puzzle::tests::test_layered_twist_serialization(&p);
        }
    }

    fn twist_comparison_key(p: &Rubiks4D, twist: Twist) -> impl PartialEq {
        const SOME_PROGRESS: f32 = 0.1;

        let face: FaceEnum = twist.axis.into();
        let matrix = face.twist_matrix(twist.direction.into(), SOME_PROGRESS);
        let pieces_affected = p.pieces_affected_by_twist(twist);
        (matrix, pieces_affected)
    }
}
