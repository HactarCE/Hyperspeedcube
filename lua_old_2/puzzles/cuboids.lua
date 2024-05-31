--HSC v2.1.10

local utils = require('utils')
local cuboid_shapes = require('shapes/cuboids')

function make_cuboid_puzzle(p, q, r)
  local function size(v)
    if abs(v.x) > 0.1 then return p end
    if abs(v.y) > 0.1 then return q end
    if abs(v.z) > 0.1 then return r end
  end

  local twists = {
    ndim = 3,
    axes = 'cubic',
    symmetry = cd{2, 2},
    build = function(twists)
      local size = vec(p, q, r)

      local R, U, F = twists.axes.R, twists.axes.U, twists.axes.F
      local twist_rot = rot{from = F, to = U}
      for t, axis, twist_rot in cd{4, 3}:chiral():orbit(R, twist_rot) do
        local s = t.rev:transform(size)
        local y = round(abs(s.y))
        local z = round(abs(s.z))
        local allow_double_turns = y % 2 == z % 2
        if not allow_double_turns then
          twist_rot = twist_rot * twist_rot
        end
        twists:add(utils.twist3d(axis, twist_rot))
      end
    end,
  }

  local name = p .. 'x' .. q .. 'x' .. r
  puzzles:add(name, {
    name = name,
    ndim = 3,
    symmetry = cd{2, 2},

    shape = cuboid_shapes.make_cuboid_shape(p, q, r),
    twists = twists,

    build = function(puz)
      local size = vec(p, q, r)

      for t, axis in cd{4, 3}:orbit(puz.axes.R) do
        local s = abs(round(t.rev:transform(size).x))
        for i = 1, s-1 do
          local v = axis.vector
          puz.shape:slice(plane{normal = v, distance = s/2 - i})
          axis.layers:add(plane{normal = v, distance = s/2 - i})
        end
      end
    end,
  })
end

make_cuboid_puzzle(2, 3, 4)
make_cuboid_puzzle(3, 1, 3)
make_cuboid_puzzle(2, 3, 6)
