    pub fn sandwich_multivector(&self, multivector: &Multivector) -> Multivector {
        self.0
            .iter()
            .flat_map(|&lhs| {
                multivector
                    .0
                    .iter()
                    .flat_map(move |&mid| self.0.iter().map(move |&rhs| lhs * mid * rhs.reverse()))
            })
            .sum()
    }
    /// Returns the matrix equivalent to a sandwich product with the
    /// multivector.
    ///
    /// The matix is more expensive to compute initially than any one sandwich
    /// product, but cheaper to apply.
    pub fn matrix(&self) -> Matrix {
        Matrix::from_cols((0..self.ndim()).map(|axis| self.sandwich_axis_vector(axis, 1.0)))
    }
    /// Returns the sandwich product with an axis-aligned vector: `M * v
    /// * M_rev`.
    fn sandwich_axis_vector(&self, axis: u8, mag: f32) -> Vector {
        let ndim = std::cmp::max(self.ndim(), axis + 1);
        let mid = Term {
            coef: mag,
            axes: Axes::euclidean(axis),
        };

        let mut ret = Vector::zero(ndim);
        for &lhs in &self.0 {
            for &rhs in &self.0 {
                let term = lhs * mid * rhs.reverse();
                if term.axes.count() == 1 {
                    let euclidean_axis = term.axes.min_euclidean_ndim() - 1;
                    ret[euclidean_axis] += term.coef;
                }
            }
        }
        ret
