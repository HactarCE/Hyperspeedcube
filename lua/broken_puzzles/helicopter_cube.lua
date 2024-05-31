print("Skipping " .. FILENAME) if true then return end
-- This puzzle is broken because HSC2 doesn't handle jumbling stops properly

local cubic = require('symmetries/cubic')
local utils = require('utils')

puzzles:add('helicopter_cube', {
  name = "Helicopter Cube",
  ndim = 3,
  build = function(p)
    local sym = cd'bc3'
    local oox = sym.oox.unit
    local oox_orbit = sym:orbit(oox)

    -- Build cube shape
    p:carve(sym:orbit(oox):with(cubic.FACE_NAMES))
    p.colors:set_defaults(cubic.FACE_COLORS)

    -- Define axes and slices
    p:add_axes(sym:orbit(sym.oxo.unit), {layers = {1/sqrt(2)}})

    -- Define twists
    for _, axis in sym:chiral():orbit(p.axes[sym.oxo.unit]) do
      p.twists:add{ axis = axis, transform = rot{ fix = axis, angle = PI } }
      p.twists:add{ axis = axis, transform = rot{ fix = axis, angle = acos(1/3) }, inverse = true, multipliers = false }
    end
  end,
})
