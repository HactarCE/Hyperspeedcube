puzzles:add{
  id = 'zzz_test_puzzle',
  name = "Test Puzzle",
  version = '0.0.1',
  ndim = 3,
  build = function(p) p:carve(cd'bc3':orbit(cd'bc3'.oox.unit)) end,
  tags = { 'experimental' },
}

puzzles:add{
  id = 'zzz_opposite_colors_same_cube',
  name = "Opposite colors are the same",
  version = '0.1.0',
  ndim = 3,
  colors = 'hemicube',
  build = function(self)
    local sym = cd'bc3'
    local shape = lib.symmetries.bc3.cube()
    self:carve(shape:iter_poles(), {
      stickers = {
        R = 'X', L = 'X',
        U = 'Y', D = 'Y',
        F = 'Z', B = 'Z',
      },
    })

    -- Define axes and slices
    self.axes:add(shape:iter_poles(), lib.utils.layers.inclusive(1, -1, 3))

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox], sym:thru(2, 1)) do
      self.twists:add(axis, twist_transform)
    end
  end,

  tags = { 'experimental' },
}
