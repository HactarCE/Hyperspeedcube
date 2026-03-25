use super::*;

impl Space {
    /// Returns a primordial cube that has already been generated.
    pub fn get_primordial_cube(&self) -> Result<Polytope<'_>> {
        Ok(self.get(
            self.primordial_cube
                .lock()
                .ok_or_eyre("no primordial cube has been generated")?,
        ))
    }

    /// Adds a primordial cube to the space. When converting a shape to
    /// simplexes, any polytope flush with a facet of the primordial cube will
    /// produce an error.
    ///
    /// Use [`crate::PRIMORDIAL_CUBE_RADIUS`] for a sensible default.
    pub fn add_primordial_cube(&self, size: Float) -> Result<Polytope<'_>> {
        // Construct a 3^d array of polytope elements. Along each axis X, the
        // polytopes at X=0 and X=1 are on the boundary of X=2.
        let mut elements = Vec::<ElementId>::with_capacity(3_usize.pow(self.ndim() as _));
        let mut position = vec![0; self.ndim() as usize];
        'outer: loop {
            let zero_axes = position.iter().positions(|&x| x == 2).collect_vec();
            let centroid = Point::from_iter(position.iter().map(|&x| [-size, size, 0.0][x]));
            let element_rank = zero_axes.len() as u8;
            let polytope_data = if element_rank == 0 {
                self.add_vertex(centroid)?.into()
            } else {
                let stride = |i| 3_usize.pow(i as _);
                let base: usize = position
                    .iter()
                    .enumerate()
                    .map(|(i, &x)| stride(i) * x)
                    .sum();
                let boundary_indexes = position
                    .iter()
                    .positions(|&x| x == 2)
                    .flat_map(|i| [base - stride(i), base - stride(i) * 2])
                    .collect_vec();

                let boundary = boundary_indexes.iter().map(|&i| elements[i]).collect();
                let hyperplane = if element_rank + 1 == self.ndim() {
                    Some(
                        self.hyperplanes.lock().push(
                            Hyperplane::from_pole(centroid.as_vector())
                                .ok_or_eyre("error constructing primordial hyperplane")?,
                        )?,
                    )
                } else {
                    None
                };
                PolytopeData::Polytope {
                    rank: element_rank,
                    boundary,
                    hyperplane,

                    is_primordial: element_rank + 1 == self.ndim(),
                    seam: None,

                    patch: None,
                }
            };

            let new_id = self.add_polytope(polytope_data)?;
            if element_rank == self.ndim() {
                // We've constructed the whole cube!
                let whole_cube = self.get(new_id).as_polytope()?;
                *self.primordial_cube.lock() = Some(whole_cube.id);
                return Ok(whole_cube);
            }
            elements.push(new_id);

            // Move to the next element position.
            for component in &mut position {
                *component += 1;
                if *component > 2 {
                    *component = 0;
                } else {
                    continue 'outer;
                }
            }
        }
    }
}
