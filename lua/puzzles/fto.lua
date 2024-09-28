local utils = require('utils')
local symmetries = require('symmetries')

function define_fto(size, name)
  local name_suffix
  if name == '' then
    name_suffix = ''
  else
    name_suffix = ' (' .. name .. ')'
  end

  puzzles:add{
    id = 'fto_' .. size,
    name = "FT Octahedron " .. size .. name_suffix,
    version = '0.1.0',
    ndim = 3,
    colors = 'octahedron',
    meta = {
      authors = {'Andrew Farkas', 'Milo Jacquet'},
    },
    build = function(self)
      local sym = cd'bc3'
      local shape = symmetries.octahedral.octahedron()
      self:carve(shape:iter_poles())

      -- Define axes and slices
      self.axes:add(shape:iter_poles(), utils.layers_exclusive(1, -1, size-1))

      -- Define twists
      for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
        self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
      end
    end,
  }
end

define_fto(1, '')
define_fto(2, 'Skewb Diamond')
define_fto(3, '')
define_fto(4, 'Master FTO')
define_fto(5, '')
define_fto(6, '')
define_fto(7, '')
