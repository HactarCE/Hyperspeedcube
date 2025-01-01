puzzles:add{
  id = 'pyra_test',
  ndim = 3,
  build = function(self)
    local sym = cd'a3'
    self:carve(sym:orbit(sym.oox.unit))
    self.axes:add(sym:orbit(sym.oox.unit), {1, 0.75, 0.5, 0.25, 0, -INF})
  end,
}
