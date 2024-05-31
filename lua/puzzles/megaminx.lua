local dodecahedral = require('symmetries/dodecahedral')
local utils = require('utils')

puzzles:add('megaminx', {
  name = "Megaminx",
  ndim = 3,
  build = function(p)
    local sym = cd'h3'
    local oox = sym:vec('oox').unit;

    -- Build shape
    p:carve(sym:orbit(oox):with(dodecahedral.FACE_NAMES))
    p.colors:set_defaults(dodecahedral.FACE_COLORS)

    -- Define axes and slices
    p:add_axes(sym:orbit(oox):with(dodecahedral.AXIS_NAMES), {1/PHI})

    -- Define twists
    for _, axis, twist_transform in sym:chiral():orbit(p.axes[oox], sym:thru(2, 1)) do
      p.twists:add(axis, twist_transform)
    end
  end,
})
