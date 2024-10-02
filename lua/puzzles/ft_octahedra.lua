local utils = require('utils')
local symmetries = require('symmetries')

puzzle_generators:add{
  id = 'ft_octahedron',
  version = '0.1.0',

  name = "N-Layer Face-Turning Octahedron",
  meta = {
    authors = { "Andrew Farkas", "Milo Jacquet" },
  },

  params = {
    { name = "Layers", type = 'int', default = 1, min = 0, max = 7 },
  },

  examples = {
    { params = {0}, name = "Octahedron" },
    { params = {2}, name = "Skewb Diamond" },
    { params = {3}, name = "Face-Turning Octahedron" },
    { params = {4}, name = "Master Face-Turning Octahedron" },
  },

  gen = function(params)
    local size = params[1]

    return {
      name = size .. "-Layer Face-Turning Octahedron ",

      colors = 'octahedron',

      ndim = 3,
      build = function(self)
        local sym = cd'bc3'
        local shape = symmetries.octahedral.octahedron()
        self:carve(shape:iter_poles())

        -- Define axes and slices
        self.axes:add(shape:iter_poles(), utils.layers.exclusive(1, -1, size-1))

        -- Define twists
        for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
          self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
        end
      end,
    }
  end
}
