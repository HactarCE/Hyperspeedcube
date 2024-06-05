local tetrahedral = require('symmetries/tetrahedral')
local utils = require('utils')

puzzles:add('hmt', {
  name = "Halpern-Meier Tetrahedron",
  ndim = 3,
  build = function(p)
    local sym = cd'a3'
    local xoo = sym:vec('xoo').unit;
    local oox = sym:vec('oox').unit;

    -- Build shape
    p:carve(sym:orbit(oox):with(tetrahedral.FACE_NAMES))
    p.colors:set_defaults(tetrahedral.FACE_COLORS)

    -- Define axes and slices
    p:add_axes(sym:orbit(oox):with(tetrahedral.FACE_AXIS_NAMES), {0})
    p:add_axes(sym:orbit(xoo):with(tetrahedral.VERTEX_AXIS_NAMES), {0})

    -- Define twists
    for _, axis, twist_transform in sym:chiral():orbit(p.axes[oox], sym:thru(2, 1)) do
      p.twists:add(axis, twist_transform)
    end
    for _, axis, twist_transform in sym:chiral():orbit(p.axes[xoo], sym:thru(3, 2)) do
      p.twists:add(axis, twist_transform)
    end
  end,
})
