use hypermath::{Point, Vector};

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
        let centroid = self.centroid.as_vector();
        Self {
            ndim: ndim_below + self.ndim + ndim_above,
            centroid: crate::lift_vector_by_ndim(centroid, ndim_below, self.ndim, ndim_above),
            normal: crate::lift_vector_by_ndim(&self.normal, ndim_below, self.ndim, ndim_above),
        }
    }
}
