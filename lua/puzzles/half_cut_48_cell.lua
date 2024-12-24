GIZMO_EDGE_FACTOR = 0.8

function build_48_cell_puzzle(self, shape_name)
  local sym = cd'f4'

  local cubic_sym = symmetry{
    sym:thru(2, 3, 2),
    sym:thru(2),
    sym:thru(2, 1, 2),
    sym:thru(2, 4, 3, 2, 3, 4, 2),
  }


  if shape_name == 'hypercube' then
    self:carve(cubic_sym:orbit(sym.ooox.unit):named(lib.symmetries.hypercubic.HYPERCUBE_FACET_NAMES))
  elseif shape_name == 'orthoplex' then
    self:carve(cubic_sym:orbit(sym:thru(4):transform(sym.ooox.unit)))
  elseif shape_name == 'octaplex' then
    self:carve(sym:orbit(sym.ooox.unit))
  elseif shape_name == '48cell' then
    self:carve(sym:orbit(sym.ooox.unit))
    self:carve(sym:orbit(sym.xooo.unit))
  else
    error('unknown shape')
  end

  -- self:carve(cubic_sym:orbit(sym.ooxx.unit) )

  local axes1 = self.axes:add(sym:orbit(sym.ooox.unit), {INF, 0, -INF})
  for t in sym:orbit(sym.xoox) do
    self.twists:add(t:transform(axes1[1]), t:transform_oriented(sym:thru(3, 2)), {
      gizmo_pole_distance = 1,
    })
  end
  for t in sym:orbit(sym.oxox) do
    self.twists:add(t:transform(axes1[1]), t:transform_oriented(sym:thru(3, 1)), {
      gizmo_pole_distance = (1 + GIZMO_EDGE_FACTOR) / sqrt(2),
    })
  end
  for t in sym:orbit(sym.ooxx) do
    self.twists:add(t:transform(axes1[1]), t:transform_oriented(sym:thru(2, 1)), {
      gizmo_pole_distance = (1 + 2 * GIZMO_EDGE_FACTOR) / sqrt(3),
    })
  end

  local axes2 = self.axes:add(sym:orbit(sym.xooo.unit), {INF, 0, -INF})
  for t in sym:orbit(sym.xoox) do
    self.twists:add(t:transform(axes2[1]), t:transform_oriented(sym:thru(3, 2)), {
      gizmo_pole_distance = 1,
    })
  end
  for t in sym:orbit(sym.xoxo) do
    self.twists:add(t:transform(axes2[1]), t:transform_oriented(sym:thru(4, 2)), {
      gizmo_pole_distance = (1 + GIZMO_EDGE_FACTOR) / sqrt(2),
    })
  end
  for t in sym:orbit(sym.xxoo) do
    self.twists:add(t:transform(axes2[1]), t:transform_oriented(sym:thru(4, 3)), {
      gizmo_pole_distance = (1 + 2 * GIZMO_EDGE_FACTOR) / sqrt(3),
    })
  end
end

puzzles:add{
  id = 'half_cut_48_cell_hypercubic',
  version = '0.1.0',
  name = "Half-Cut 48-Cell (Hypercube)",
  tags = {
    author = {"Andrew Farkas", "Luna Harran", "Milo Jacquet"},
  },
  colors = 'hypercube',
  ndim = 4,
  build = function(self) build_48_cell_puzzle(self, 'hypercube') end,
}

puzzles:add{
  id = 'half_cut_48_cell_16_cell',
  version = '0.1.0',
  name = "Half-Cut 48-Cell (16-Cell)",
  tags = {
    author = {"Andrew Farkas", "Luna Harran", "Milo Jacquet"},
  },
  ndim = 4,
  build = function(self) build_48_cell_puzzle(self, 'orthoplex') end,
}

puzzles:add{
  id = 'half_cut_48_cell_24_cell',
  version = '0.1.0',
  name = "Half-Cut 48-Cell (24-Cell)",
  tags = {
    author = {"Andrew Farkas", "Luna Harran", "Milo Jacquet"},
  },
  ndim = 4,
  build = function(self) build_48_cell_puzzle(self, 'octaplex') end,
}

puzzles:add{
  id = 'half_cut_48_cell',
  version = '0.1.0',
  name = "Half-Cut 48-Cell",
  tags = {
    author = {"Andrew Farkas", "Luna Harran", "Milo Jacquet"},
  },
  ndim = 4,
  build = function(self) build_48_cell_puzzle(self, '48cell') end,
}
