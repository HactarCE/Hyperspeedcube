puzzles:add{
  id = '2x2x2x2x2',
  ndim = 5,
  build = function(self)
    local sym = cd'bc5'
    self:carve(sym:orbit(sym.oooox.unit))
    self:slice(sym:orbit(plane(sym.oooox.unit, 0)))
  end,
}
