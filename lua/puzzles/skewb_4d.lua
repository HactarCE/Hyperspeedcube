local octahedral = require('symmetries/octahedral')
local cubic = require('symmetries/cubic')

puzzles:add('skewb_prism', {
  name = "skewb prism",
  ndim = 4,
  build = function(p)
    local sym = cd{4,3,2}
    local xoo = sym.xooo.unit
    local oox = sym.ooxo.unit
    local ax = vec'w' * 0.5
    print(pole)
    p:carve(sym:orbit(oox):with(cubic.FACE_NAMES))
    p:carve(ax)
    p:carve(-ax)

    --p.axes:add(sym:orbit(pole):with(octahedral.AXIS_NAMES), {0})
    p.axes:add(sym:orbit(xoo), {0})

    for _, axis, twist_transform in sym.chiral:orbit(p.axes[xoo], sym:thru(3, 2)) do
      p.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end
    for _, axis, twist_transform in sym.chiral:orbit(p.axes[xoo], sym:thru(4, 2)) do
      p.twists:add(axis, twist_transform, {gizmo_pole_distance = 0.7})
    end
  end,
})
