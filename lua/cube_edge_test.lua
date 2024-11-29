puzzles:add{
  id = 'cube_edge_test',
  ndim = 3,
  build = function(self)
    self:carve(lib.symmetries.cubic.cube():iter_poles())
    self:carve(lib.symmetries.cubic.cube(1/sqrt(2)):iter_edge_poles())
  end,
}
