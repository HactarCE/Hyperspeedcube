local tribonacci_constant = (1 + 4*cosh(1/3 * acosh(2 + 3/8))) / 3

puzzles:add{
  id = "pentagonal_icositetrahedron",
  version = '0.1.0',
  name = "Pentagonal Icositetrahedron",
  tags = {
    "type/shape",
    "experimental",
  },
  ndim = 3,
  build = function(self)
    local sym = cd'bc3'.chiral
    self:carve(sym:orbit(vec(1, 1/tribonacci_constant, tribonacci_constant)))
    self.axes:add(sym:orbit(vec(1, 1/tribonacci_constant, tribonacci_constant)), {INF, 1.85})
  end,
}

puzzles:add{
  id = 'snub_cube',
  version = '0.1.0',
  name = "Snub Cube",
  tags = {
    "type/shape",
    experimental = true,
  },
  ndim = 3,
  build = function(self)
    local sym = cd'bc3'.chiral

    local v1 = vec(1, 2*tribonacci_constant + 1, tribonacci_constant^2)
    local v2 = vec(tribonacci_constant^3)
    local v3 = vec(1,1,1) * tribonacci_constant^2

    local scale = v2.mag

    local function dual_vertex_to_pole(v)
      return v / v.mag2 * scale
    end

    self:carve(sym:orbit(dual_vertex_to_pole(v1)))
    self:carve(sym:orbit(dual_vertex_to_pole(v2))) -- square faces
    self:carve(sym:orbit(dual_vertex_to_pole(v3)))
  end
}
