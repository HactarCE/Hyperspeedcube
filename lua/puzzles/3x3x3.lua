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
  build = function(p)
    local sym = shapes.cube.sym
    local oox = sym.oox.unit

    -- Build shape
    -- p:view(my_transform):carve(shapes.cube, 1)

    -- Build shape
    shapes.cube:carve_into(p)

    -- Define axes and slices
    p:add_axes(sym:orbit(oox):with(symmetries.cubic.FACE_NAMES_SHORT), {1/3, -1/3})

    -- Define twists
    for _, axis, twist_transform in sym:chiral():orbit(p.axes[oox], sym:thru(2, 1)) do
      p.twists:add(axis, twist_transform)
    end
  end,
})
