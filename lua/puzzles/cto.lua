local utils = require('utils')
local symmetries = require('symmetries')

puzzles:add('cto', {
  ndim = 3,
  name = "Corner-Turning Octahedron",
  build = function(self)
    local sym = cd'bc3'
    self:carve(sym:orbit(sym.xoo.unit))

    -- Define axes and slices
    self.axes:add(sym:orbit(sym.oox.unit), utils.layers_exclusive(sqrt(2), 0, 3))

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox.unit], sym:thru(2, 1)) do
      self.twists:add(axis, twist_transform)
    end
  end,
})
