
/// Non-imaginary shape represneted by a CGA blade.
pub enum BladeShape {
    Degenerate,
    Scalar(f32),
    Point(CgaPoint),
    FlatPoint(CgaPoint),
    PointPair([CgaPoint; 2]),
    Hyperplane {
        rank: u8,
        normal: Vector,
        distance: f32,
        span: Vec<Vector>,
        antispan: Vec<Vector>,
    },
    Hypersphere {
        rank: u8,
        center: Vector,
        radius: f32,
        span: Vec<Vector>,
        antispan: Vec<Vector>,
    },
}


    /// Returns information about the shape represented by the OPNS blade, or
    /// `None` if the operation failed for an unknown reason.
    pub fn opns_shape(&self, ndim: u8) -> Option<BladeShape> {
        if approx_eq(&self.mag2(), &0.0) {
            return Some(BladeShape::Degenerate);
        }
        Some(match self.grade() {
            0 => BladeShape::Scalar(self.mv()[Axes::SCALAR]),
            1 => BladeShape::Point(self.to_point()),
            2 => BladeShape::PointPair(self.point_pair_to_points()?),
            _ if self.grade() == ndim + 1 => {
                let ipns = self.opns_to_ipns(ndim);
                let span = self.opns_flat_span()
                if ipns.ipns_is_flat() {
                    BladeShape::Hyperplane {
                        normal: ipns.ipns_plane_normal()?,
                        distance: ipns.ipns_plane_distance()?,
                    }
                } else {
                        let radius = Term::scalar(self.mag()?);
                        let multiplier = (Blade::NI << self).inverse()?;
                        Some([
                            Blade((self.mv() - radius) * multiplier.mv()).to_point(),
                            Blade((self.mv() + radius) * multiplier.mv()).to_point(),
                        ])


                    BladeShape::Hypersphere {
                        center: ipns.ipns_sphere_center().to_option()?,
                        radius: ipns.ipns_radius()?,
                    }
                }
            }
            _ => BladeShape::Unknown {
                rank: self.grade() - 2,
                is_flat: self.opns_is_flat(),
            },
        })


/// Returns the spanning vectors for the smallest flat subspace containing
/// the OPNS blade. For an N-sphere, this returns N+1 vectors. For a flat
/// N-dimensional subspace, this returns N vectors. All returned vectors are
/// normalized and mutually orthogonal.
///
/// Returns `None` if the blade is not invertible.
pub fn opns_flat_span(&self) -> Vec<Vector> {
    if !self.opns_is_flat() {
        return (self ^ Blade::NI).opns_flat_span();
    }
    let mut remaining = self;
    for axis in self.ndim() {
        if remaining.grade()
        let v =  Vector::unit(axis);
        // Project `a` onto `remaining`.
        (v << remaining.inverse())
    }
}
