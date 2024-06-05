local shapes = require('shapes')
local symmetries = require('symmetries')

puzzles:add('cuboctahedron', {
  name = "Cuboctahedron",
  ndim = 3,
  build = function(p)
    local sym = cd'bc3'

    -- Build shape
    shapes.cuboctahedron:carve_into(p)

    -- -- Define axes and slices
    -- p:add_axes(sym:orbit(oox):with(cubic.AXIS_NAMES), {1/3, -1/3})

    -- -- Define twists
    -- for _, axis, twist_transform in sym:chiral():orbit(p.axes[oox], sym:thru(2, 1)) do
    --   p.twists:add(axis, twist_transform)
    -- end
  end,
})
