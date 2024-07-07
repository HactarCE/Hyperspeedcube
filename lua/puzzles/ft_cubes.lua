local utils = require('utils')
local symmetries = require('symmetries')

function define_ft_cube_3d(size)
  local id = size .. 'x' .. size .. 'x' .. size
  puzzles:add(id, {
    ndim = 3,
    name = size .. '^3',
    colors = 'cube',
    build = function(self)
      local sym = cd'bc3'
      local shape = symmetries.cubic.cube()
      self:carve(shape:iter_poles())

      -- Define axes and slices
      self.axes:add(shape:iter_poles(), utils.layers_exclusive(1, -1, size))

      -- Define twists
      for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox], sym:thru(2, 1)) do
        self.twists:add(axis, twist_transform)
      end
    end,
  })
end

for size = 1, 9 do
  define_ft_cube_3d(size)
end

function define_ft_cube_4d(size)
  local gizmo_size = 1.2
  local alpha = 0.8

  local id = size .. 'x' .. size .. 'x' .. size .. 'x' .. size
  puzzles:add(id, {
    ndim = 4,
    name = size .. '^4',
    colors = 'hypercube',
    build = function(self)
      local sym = cd'bc4'
      local shape = symmetries.hypercubic.hypercube()
      self:carve(shape:iter_poles())

      -- Define axes and slices
      self.axes:add(shape:iter_poles(), utils.layers_exclusive(1, -1, size))

      -- Define twists
      local a1 = self.axes[sym.ooox.unit]
      local a2 = sym:thru(4):transform(a1)
      local a3 = sym:thru(3):transform(a2)
      local a4 = sym:thru(2):transform(a3)
      local t = sym:thru(2, 1)
      for _, axis1, axis2, twist_transform in sym.chiral:orbit(a1, a2, t) do
        self.twists:add(axis1, twist_transform, {
          name = axis1.name .. axis2.name,
          gizmo_pole_distance = gizmo_size,
        })
      end

      local edge = a2.vector + a3.vector -- ridge orthogonal to `a1`
      local init_transform = sym:thru(3, 1) -- rot{fix = a1.vector ^ edge, angle = PI}
      for t, axis1, _edge, twist_transform in sym.chiral:orbit(a1, edge, init_transform) do
        self.twists:add(axis1, twist_transform, {
          name = axis1.name .. t:transform(a2).name .. t:transform(a3).name,
          gizmo_pole_distance = (1+alpha)/sqrt(2) * gizmo_size,
        })
      end

      local vertex = edge + a4.vector -- edge orthogonal to `a1`
      local init_transform = sym:thru(3, 2)
      for t, axis1, _vertex, twist_transform in sym.chiral:orbit(a1, vertex, init_transform) do
        self.twists:add(axis1, twist_transform, {
          name = axis1.name .. t:transform(a2).name .. t:transform(a3).name .. t:transform(a4).name,
          gizmo_pole_distance = (1+2*alpha)/sqrt(3) * gizmo_size,
        })
      end
    end,
  })
end

for size = 1, 9 do
  define_ft_cube_4d(size)
end

puzzles:add('opposite_colors_same_cube', {
  ndim = 3,
  name = "Opposite colors are the same",
  colors = 'half_cube',
  build = function(self)
    local sym = cd'bc3'
    local shape = symmetries.cubic.cube()
    self:carve(shape:iter_poles(), {
      stickers = {
        R = 'X', L = 'X',
        U = 'Y', D = 'Y',
        F = 'Z', B = 'Z',
      },
    })

    -- Define axes and slices
    self.axes:add(shape:iter_poles(), utils.layers_exclusive(1, -1, size))

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox], sym:thru(2, 1)) do
      self.twists:add(axis, twist_transform)
    end
  end,
})
