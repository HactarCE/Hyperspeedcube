puzzles:add('3x3x3_min', {
  name = "Minimal 3x3x3 definition",
  ndim = 3,
  build = function(p)
    local sym = cd'bc3'
    local v = sym.oox.unit
    p:carve(sym:orbit(v))
    p:add_axes(sym:orbit(v), {1/3, -1/3})
    for _, ax, tr in sym:chiral():orbit(p.axes[v], sym:thru(1, 2)) do
      p.twists:add(ax, tr)
    end
  end,
})
