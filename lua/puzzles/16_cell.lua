-- TODO: 0.5 to delete center
-- TODO: half-cut
puzzles:add{
  id = '16_cell',
  version = '0.1.0',
  ndim = 4,
  build = function(self)
    local sym = cd{3,3,4}
    self:carve(sym:orbit(sym.ooox.unit))
    self.axes:add(sym:orbit(sym.ooox.unit), {1,3/5})
  end,

  tags = {
    'experimental',
  }
}
