local hypercubic = require('symmetries/hypercubic')

puzzles:add('rt_hypercube', {
  name = "Ridge-turning hypercube",
  ndim = 4,
  build = function(p)
    local sym = cd'bc4'
    local ooox = sym.ooox.unit
    local ooxo = sym.ooxo.unit

    -- Build shape
    p:carve(sym:orbit(ooox):with(hypercubic.FACE_NAMES))
    p.colors:set_defaults(hypercubic.FACE_COLORS)

    -- Define axes and slices
    p.axes:add(sym:orbit(ooxo), {1})
    p.axes:autoname()

    -- Define twists
    local f = ooox
    local a1 = p.axes[ooxo]
    local t = sym:thru(2, 1)
    for _, axis, twist_transform in sym.chiral:orbit(a1, t) do
      p.twists:add(axis, twist_transform, {
        gizmo_pole_distance = 0.5,
      })
    end
    local t = sym:thru(4, 1)
    for _, axis, twist_transform in sym.chiral:orbit(a1, t) do
      p.twists:add(axis, twist_transform, {
        gizmo_pole_distance = 1,
      })
    end
    local t = sym:thru(4, 2)
    for _, axis, twist_transform in sym.chiral:orbit(a1, t) do
      p.twists:add(axis, twist_transform, {
        gizmo_pole_distance = 1.1,
      })
    end
  end,
})
