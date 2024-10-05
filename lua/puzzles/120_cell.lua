SHALLOW_CUT_DEPTH = 3/2 * 1/phi
EVIL_CUT_DEPTH = 1/phi

GIZMO_FACET_SIZE = tan(pi/10)
GIZMO_EDGE_FACTOR = 0.8

function build_120_cell(self, depth)
  local sym = cd'h4'
  local facet_poles = sym:orbit(sym.ooox.unit)

  self:carve(facet_poles)
  if depth ~= nil then
    local axes = self.axes:add(facet_poles, {depth})

    self.axes:autoname()



    -- Define twists
    local a1 = self.axes[sym.ooox.unit]
    local a2 = sym:thru(4):transform(a1)
    local a3 = sym:thru(3):transform(a2)
    local a4 = sym:thru(2):transform(a3)
    local t = sym:thru(2, 1)
    for _, axis1, axis2, twist_transform in sym.chiral:orbit(a1, a2, t) do
      self.twists:add(axis1, twist_transform, {
        name = axis2.name,
        gizmo_pole_distance = GIZMO_FACET_SIZE,
      })
    end

    -- local ridge = a2.vector + a3.vector -- ridge orthogonal to `a1`
    -- local init_transform = sym:thru(3, 1) -- rot{fix = a1.vector ^ ridge, angle = PI}
    -- for t, axis1, _ridge, twist_transform in sym.chiral:orbit(a1, ridge, init_transform) do
    --   self.twists:add(axis1, twist_transform, {
    --     name = t:transform(a2).name .. t:transform(a3).name,
    --     gizmo_pole_distance = GIZMO_FACET_SIZE * ((5+3*sqrt(5))/10 * (1-GIZMO_EDGE_FACTOR) + phi * GIZMO_EDGE_FACTOR) / (2*(3+sqrt(5))/5),
    --   })
    -- end

    -- local edge = ridge + a4.vector -- edge orthogonal to `a1`
    -- local init_transform = sym:thru(3, 2)
    -- for t, axis1, _edge, twist_transform in sym.chiral:orbit(a1, edge, init_transform) do
    --   self.twists:add(axis1, twist_transform, {
    --     name = t:transform(a2).name .. t:transform(a3).name .. t:transform(a4).name,
    --     gizmo_pole_distance = GIZMO_FACET_SIZE * (sqrt(3/5 + 4/(3*sqrt(5))) * (1-GIZMO_EDGE_FACTOR) + sqrt(3) * GIZMO_EDGE_FACTOR) / (2*(3+sqrt(5))/5),
    --   })
    -- end
  end
end

puzzles:add{
  id = '120_cell',
  version = '0.1.0',
  name = "120-Cell",
  meta = {
    author = {"Andrew Farkas", "Milo Jacquet"},
  },
  ndim = 4,
  build = function(self) build_120_cell(self) end,
}

puzzles:add{
  id = '120_cell_shallow',
  version = '0.1.0',
  name = "Facet-Turning 120-Cell (Shallow)",
  meta = {
    author = {"Andrew Farkas", "Milo Jacquet"},
  },
  ndim = 4,
  build = function(self) build_120_cell(self, SHALLOW_CUT_DEPTH) end,
}

puzzles:add{
  id = '120_cell_evil',
  version = '0.1.0',
  name = "Facet-Turning 120-Cell (Evil)",
  meta = {
    author = {"Andrew Farkas", "Milo Jacquet"},
  },
  ndim = 4,
  build = function(self) build_120_cell(self, EVIL_CUT_DEPTH) end,
}
