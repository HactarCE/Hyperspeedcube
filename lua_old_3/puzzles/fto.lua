local octahedral = require('symmetries/octahedral')

puzzles:add('fto', {
  name = "Face-Turning Octahedron",
  ndim = 3,
  build = function(p)
    local shape = octahedral.octahedron()
    local sym = shape.symmetry
    local xoo = sym.xoo.unit

    -- Build shape
    shape:carve_into(p)

    -- Define axes and slices
    p.axes:add(sym:orbit(xoo):with(octahedral.FACE_NAMES), {1/3, -1/3})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(p.axes[xoo], sym:thru(3, 2)) do
      p.twists:add(axis, twist_transform)
    end
  end,
})
