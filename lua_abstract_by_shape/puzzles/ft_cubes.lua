local utils = require('utils')

function define_ft_cube(size)
  local id = size .. 'x' .. size .. 'x' .. size
  puzzles:add(id, {
    shape = 'cube', -- defines `self.shape` and color schemes; does not actually carve
    build = function(self)
      local sym = self.shape.sym
      self.shape:build()

      -- Define axes and slices
      self.axes:add(self.shape:iter_poles(), utils.layers_exclusive(-1, 1, size))

      -- Define twists
      for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox], sym:thru(2, 1)) do
        self.twists:add(axis, twist_transform)
      end
    end,
  })
end

for size = 1,10 do
  define_ft_cube(size)
end
