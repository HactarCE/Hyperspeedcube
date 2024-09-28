local utils = require('utils')

NGONAL_NAMES = {
  "Monogonal",
  "Digonal",
  "Triangular",
  "Square",
  "Pentagonal",
  "Hexagonal",
  "Heptagonal",
  "Octagonal",
  "Nonagonal",
  "Decagonal",
}

function ngonal_name(i)
  return NGONAL_NAMES[i] or i .. "-gonal"
end

function shallow_cut_ft_prism_name(n, width, height)
  local name = ngonal_name(n) .. " Prism"
  if width > 1 or height > 1 then
    name = "Face-Turning " .. name .. " (Shallow " .. width .. "x" .. height .. ")"
  end
  return name
end

function prism_face_order(n)
  local order = {'U', 'D'}
  for i = 1, n do order[i+2] = 'F' .. i end
  return order
end

-- Returns the length of an edge of an n-gonal polygon with inradius 1
function polygon_edge_length(n)
  return tan(pi/n)*2
end

function prism_base_poles(n)
  local sym = cd{n, 2}
  return sym:orbit(sym.oox.unit * polygon_edge_length(n)/2):named{
    U = {},
    D = {3},
  }
end

function prism_side_poles(n)
  local sym = cd{n, 2}
  local names = {
    F1 = {}
  }
  for i = 2, ceil(n/2) do
    names['F' .. i] = {1, 'F' .. n-i+2}
  end
  for i = 1, floor(n/2) do
    names['F' .. n-i+1] = {2, 'F' .. i}
  end
  return sym:orbit(sym.oxo.unit):named(names)
end

function carve_prism(self, n)
  self:carve(prism_base_poles(n))
  self:carve(prism_side_poles(n))

  -- Reorder colors
  self.colors:reorder(prism_face_order(n))

  -- Assign default color_systems
  self.colors.U.default = "Mono Dyad [1]"
  self.colors.D.default = "Mono Dyad [2]"
  for i = 1, n do
    self.colors['F' .. i].default = "Rainbow [" .. i .. "]"
  end
end

function add_prism_twists(self, n, base_axis, side_axis, h)
  local sym = cd{n, 2}
  for _, axis, twist_transform in sym.chiral:orbit(base_axis, sym:thru(2, 1)) do
    self.twists:add(axis, twist_transform, {gizmo_pole_distance = h})
  end
  for _, axis, twist_transform in sym.chiral:orbit(side_axis, sym:thru(3, 1)) do
    self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
  end
end

puzzle_generators:add{
  id = 'ft_prism_3',
  version = '0.1.0',

  meta = {
    author = { "Andrew Farkas", "Luna Harran" },
  },

  name = "Face-Turning Triangular Prism",

  params = {
    { name = "Width", type = 'int', default = 3, min = 1, max = 10 },
    { name = "Height", type = 'int', default = 3, min = 1, max = 5 },
  },

  examples = {
    { params = {3,3}, name = "3-Layer Pentahedron" },
    { params = {4,5}, name = "5-Layer Pentahedron" },
  },

  gen = function(params)
    local width = params[1]
    local height = params[2]

    return {
      name = shallow_cut_ft_prism_name(3, width, height),

      ndim = 3,
      build = function(self)
        local sym = cd{3, 2}

        carve_prism(self, 3)

        local side_layers = utils.even_odd_layers(1, 0, width)
        table.insert(side_layers, -2) -- allow edge turns

        local h = polygon_edge_length(3)/2
        local base_axes = self.axes:add(prism_base_poles(3), utils.layers_exclusive(h, -h, height-1))
        local side_axes = self.axes:add(prism_side_poles(3), side_layers)
        self.axes:reorder(prism_face_order(3))

        add_prism_twists(self, 3, base_axes[1], side_axes[1], h)
      end,
    }
  end,
}

puzzle_generators:add{
  id = 'ft_prism',
  version = '0.1.0',

  meta = {
    author = { "Andrew Farkas", "Luna Harran" },
  },

  name = "Face-Turning Polygonal Prism (Shallow, 5+)",

  params = {
    { name = "Polygon size", type = 'int', default = 5, min = 3, max = 24 },
    { name = "Width", type = 'int', default = 3, min = 1, max = 10 },
    { name = "Height", type = 'int', default = 3, min = 1, max = 5 },
  },

  examples = {},

  gen = function(params)
    local n = params[1]
    local width = params[2]
    local height = params[3]

    return {
      name = shallow_cut_ft_prism_name(n, width, height),

      ndim = 3,
      build = function(self)
        local sym = cd{n, 2}

        carve_prism(self, n)

        local d = sin(2*pi/n) * polygon_edge_length(n)/2
        local side_layers = utils.even_odd_layers(1, 1 - d, width)

        local h = polygon_edge_length(n)/2
        local base_axes = self.axes:add(prism_base_poles(n), utils.layers_exclusive(h, -h, height-1))
        local side_axes = self.axes:add(prism_side_poles(n), side_layers)
        self.axes:reorder(prism_face_order(n))

        add_prism_twists(self, n, base_axes[1], side_axes[1], h)
      end,
    }
  end,
}
