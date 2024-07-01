puzzles:add('2^6', {
  name = "2^6",
  ndim = 6,
  build = function(p)
    local sym = cd'bc6'
    local ooooox = sym.ooooox.unit

    -- Build shape
    p:carve(sym:orbit(ooooox))

    -- Define axes and slices
    p.axes:add(sym:orbit(ooooox), {0})
  end,
})
