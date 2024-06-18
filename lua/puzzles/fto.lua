local octahedral = require('symmetries/octahedral')

puzzles:add('fto', {
  name = "Face-Turning Octahedron",
  ndim = 3,
  build = function(p)
    local sym = cd'bc3'
    local xoo = sym:vec('xoo').unit

    -- Build shape
    p:carve(sym:orbit(xoo):with(octahedral.FACE_NAMES))
    p.colors:set_defaults(octahedral.FACE_COLORS)

    -- Define axes and slices
    p.axes:add(sym:orbit(xoo):with(octahedral.AXIS_NAMES), {1/3, -1/3})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(p.axes[xoo], sym:thru(3, 2)) do
      p.twists:add(axis, twist_transform)
    end
  end,
})
