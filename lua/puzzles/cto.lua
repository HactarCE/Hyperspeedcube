local d = sqrt(3)
local depths = lib.utils.layers.inclusive(d, 0, 3)
depths[#depths] = nil -- delete central cut
for i = #depths, 1, -1 do table.insert(depths, -depths[i]) end -- add opposite cuts
CUT_DEPTHS = depths

puzzles:add{
  id = 'cto',
  name = "Corner-Turning Octahedron",
  version = '0.1.0',
  ndim = 3,
  colors = 'octahedron',
  build = function(self)
    local sym = cd'bc3'
    local shape = lib.symmetries.octahedral.octahedron()
    self:carve(shape:iter_poles())

    -- Define axes and slices
    self.axes:add(sym:orbit(sym.oox.unit), CUT_DEPTHS)

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox.unit], sym:thru(2, 1)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = sqrt(3)/3})
    end
  end,

  tags = {
    author = "Andrew Farkas",
    experimental = true,
  }
}
