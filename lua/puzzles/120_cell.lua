local shapes = require('shapes')
local symmetries = require('symmetries')

puzzles:add('120_cell', {
  name = "120-cell",
  ndim = 4,
  build = function(p)
    local sym = cd'h4'
    local ooox = sym.ooox.unit
    print(ooox)
    p:carve(sym:orbit(ooox))
    -- p:slice(ooox * 0.96)

    p.axes:add(sym:orbit(ooox), {3/2 * 1/phi})

    local a1 = p.axes[ooox]
    local a2 = sym:thru(4):transform(a1)
    local t = sym:thru(2, 1)
    print(sym.chiral)
    for _, axis1, axis2, twist_transform in sym.chiral:orbit(a1, a2, t) do
      p.twists:add(axis1, twist_transform, {
        gizmo_pole_distance = tan(pi/10) * 1.1,
      })
    end
  end,
})
