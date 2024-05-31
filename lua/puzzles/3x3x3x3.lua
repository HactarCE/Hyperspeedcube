local hypercubic = require('symmetries/hypercubic')

puzzles:add('3x3x3x3', {
  name = "3x3x3x3",
  ndim = 4,
  build = function(p)
    local sym = cd'bc4'
    local ooox = sym.ooox.unit

    -- Build shape
    p:carve(sym:orbit(ooox):with(hypercubic.FACE_NAMES))
    -- p.colors:set_defaults(hypercubic.FACE_COLORS)

    -- Define axes and slices
    p:add_axes(sym:orbit(ooox):with(hypercubic.AXIS_NAMES), {1/3, -1/3})

    -- Define twists
    for _, axis, twist_transform in sym:chiral():orbit(p.axes[ooox], sym:thru(1, 2)) do
      p.twists:add(axis, twist_transform, {})
    end
  end,
})
