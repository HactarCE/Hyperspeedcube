local cubic = require('symmetries/cubic')

puzzles:add('3x3x3', {
  name = "3x3x3",
  ndim = 3,
  build = function(p)
    local sym = cd'bc3'
    local oox = sym.oox.unit

    -- Build shape
    p:carve(sym:orbit(oox):with(cubic.FACE_NAMES))
    p.colors:set_defaults(cubic.FACE_COLORS)

    -- Define axes and slices
    p:add_axes(sym:orbit(oox):with(cubic.AXIS_NAMES), {1/3, -1/3})

    -- Define twists
    for _, axis, twist_transform in sym:chiral():orbit(p.axes[oox], sym:thru(1, 2)) do
      p.twists:add(axis, twist_transform)
    end
  end,
})
