use hypermath::{Point, Vector, VectorRef};

#[derive(Debug)]
pub struct SurfaceBuilder {
    /// Number of dimensions of the space containing the surface.
    ///
    /// The surface is always one dimension lower than this.
    pub ndim: u8,
    /// Centroid of the surface, used to compute facet shrink.
    ///
    /// It is acceptable for this to be slightly inaccurate.
    pub centroid: Point,
    /// Normal vector to the surface, used to cull 4D backfaces.
    pub normal: Vector,
}

impl SurfaceBuilder {
    pub fn lift_by_ndim(&self, ndim_below: u8, ndim_above: u8) -> Self {
        let below = std::iter::repeat_n(0.0, ndim_below as usize);
        let above = std::iter::repeat_n(0.0, ndim_above as usize);
        Self {
            ndim: ndim_below + self.ndim + ndim_above,
            centroid: itertools::chain!(
                below.clone(),
                self.centroid.as_vector().iter_ndim(self.ndim),
                above.clone()
            )
            .collect(),
            normal: itertools::chain!(
                below.clone(),
                self.normal.iter_ndim(self.ndim),
                above.clone()
            )
            .collect(),
        }
    }
}
