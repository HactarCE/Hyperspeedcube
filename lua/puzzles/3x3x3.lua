local shapes = require('shapes')
local symmetries = require('symmetries')

function carve_cube(p, radius)
  local sym = cd'bc3'
  p:carve(sym:orbit(sym.oox.unit * (radius or 1)):with(cubic.FACE_NAMES))
  p.colors:set_defaults(cubic.FACE_COLORS)
end

puzzles:add('3x3x3', {
  name = "3x3x3",
  ndim = 3,
  -- colors_id = '3d/cube',
  -- twists_id = '3d/cubic/ft',
  build = function(p)
    local shape = symmetries.cubic.cube()
    local sym = shape.sym
    local oox = sym.oox.unit

    -- Build shape
    shape:carve_into(p)

    -- Define axes and slices
    p.axes:add(sym:orbit(oox):with(symmetries.cubic.FACE_NAMES), {1/3, -1/3})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(p.axes[oox], sym:thru(2, 1)) do
      p.twists:add(axis, twist_transform)
    end
  end,
})
