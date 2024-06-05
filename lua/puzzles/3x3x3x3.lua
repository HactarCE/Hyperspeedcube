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
    local a1 = p.axes[ooox]
    local a2 = sym:thru(4):transform(a1)
    local t = sym:thru(2, 1)
    for _, axis1, axis2, twist_transform in sym:chiral():orbit(a1, a2, t) do
      p.twists:add(axis1, twist_transform, {
        name = axis1.name .. axis2.name,
        -- gizmo_pole_distance = axis2.vector,
      })
    end
  end,
})
