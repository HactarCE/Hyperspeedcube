local utils = require('utils')

local sym = cd{5, 3, 3}
local cut_depth = 15/16

puzzles:add('120_cell', {
  name = "120-Cell",
  ndim = 4,
  build = function(p)
    cd{4, 3}:foreach(function(s)
      s:vec('oox')
    end)
    sym:with(function(s)

    end)
    for _, v in sym:orbit('oox') do

    end
    -- axes
    for _, v in sym:orbit('ooox') do
      p.shape:carve(v.unit)
      p.shape:slice(v.unit * cut_depth)
    end

    carve(sym:orbit('ooox'))

    sym:foreach('oox', function(v) carve(v.unit) end)

  end,
})
