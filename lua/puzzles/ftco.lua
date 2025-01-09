puzzles:add{
  id = 'ftco',
  version = '0.1.0',

  name = "Face-Turning Cuboctahedron (Shallow)",

  tags = {
    author = "SuperSnowman16",
    experimental = true,
  },

  ndim = 3,
  build = function(self)
    local sym = cd'bc3'
    local oox = sym:vec('oox').unit*sqrt(3)/2
    local xoo = sym:vec('xoo').unit
    self:carve(sym:orbit(oox))
    self:carve(sym:orbit(xoo))

    -- Define axes and slices
    self.axes:add(sym:orbit(oox), {INF, sqrt(3)/2 * 3/4})
    self.axes:add(sym:orbit(xoo), {INF, 3/4})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox.unit], sym:thru(2, 1)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = sqrt(3)/2})
    end
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end
  end,
}
