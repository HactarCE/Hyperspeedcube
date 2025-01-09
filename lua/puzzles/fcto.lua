puzzles:add{
  id = 'fcto',
  name = "Comboctahedron",
  aliases = { "Face-And-Corner-Turning Octahedron" },
  version = '0.1.0',
  ndim = 3,
  colors = 'octahedron',
  build = function(self)
    local sym = cd'bc3'
    local shape = lib.symmetries.octahedral.octahedron()
    self:carve(shape:iter_poles())

    -- Define axes and slices
    self.axes:add(shape:iter_poles(), lib.utils.layers.inclusive(1, -1, 3))
    self.axes:add(sym:orbit(sym.oox.unit), lib.puzzles.cto.CUT_DEPTHS)

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox.unit], sym:thru(2, 1)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 2/sqrt(3)})
    end
  end,

  -- TODO: tags. museum=1848
  tags = {
    author = "Andrew Farkas",
    experimental = true,
  }
}
