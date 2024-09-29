local utils = require('utils')

DIAG_GIZMO_FACTOR = 2/3

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

local function get_default_color(color)
  local t = {
    -- 3D
    U = "Mono Dyad [1]",
    D = "Mono Dyad [2]",
    F = "Rainbow [0/0]",

    -- 4D
    A = "Light Rainbow [0/0]",
    B = "Dark Rainbow [0/0]",
  };
  return t[string.sub(color.name, 1, 1)]
end

local function side_face_name(prefix, i)
  return prefix .. utils.nth_uppercase_name(i)
end

function shallow_cut_ft_prism_name(n, width, height)
  local name = ngonal_name(n) .. " Prism"
  if width > 1 or height > 1 then
    name = "Face-Turning " .. name .. " (Shallow " .. width .. "x" .. height .. ")"
  end
  return name
end

function shallow_cut_ft_duoprism_name(n, m, n_size, m_size)
  local name = "{" .. n .. "}x{" .. m .. "} Duoprism"
  if n_size > 1 or m_size > 1 then
    name = "Facet-Turning " .. name .. " (Shallow " .. n_size .. "x" .. m_size .. ")"
  end
  return name
end

function prism_face_order(n)
  local order = {'U', 'D'}
  for i = 1, n do order[i+2] = side_face_name('F', i) end
  return order
end

function duoprism_face_order(n, m)
  local order = {}
  for i = 1, n do order[i] = side_face_name('A', i) end
  for i = 1, m do order[n+i] = side_face_name('B', i) end
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

function polygon_face_names(prefix, n, mirror1, mirror2)
  local names = {
    [side_face_name(prefix, 1)] = {}
  }
  for i = 2, ceil(n/2) do
    names[side_face_name(prefix, i)] = {mirror1 or 1, side_face_name(prefix, n-i+2)}
  end
  for i = 1, floor(n/2) do
    names[side_face_name(prefix, n-i+1)] = {mirror2 or 2, side_face_name(prefix, i)}
  end
  return names
end

function prism_side_poles(n)
  local sym = cd{n, 2}
  local names = polygon_face_names('F', n)
  return sym:orbit(sym.oxo.unit):named(names)
end

function duoprism_poles(n, m)
  local sym = cd{n, 2, m}
  local poles_a = sym:orbit(sym.oxoo.unit):named(polygon_face_names('A', n))
  local poles_b = sym:orbit(sym.ooox.unit):named(polygon_face_names('B', m, 3, 4))
  return poles_a, poles_b
end

function carve_prism(self, n)
  self:carve(prism_base_poles(n))
  self:carve(prism_side_poles(n))
  self.colors:reorder(prism_face_order(n))
  self.colors:set_defaults(get_default_color)
end

function carve_duoprism(self, n, m)
  local sym = cd{n, 2, m}
  local poles_a, poles_b = duoprism_poles(n, m)
  self:carve(poles_a)
  self:carve(poles_b)
  self.colors:reorder(duoprism_face_order(n, m))
  self.colors:set_defaults(get_default_color)
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

function add_duoprism_twists(self, n, m, axis_a, axis_b, which_set)
  local sym = cd{n, 2, m}

  local p = ({n, m})[which_set]

  local m1, m2, m3, m4 = 1, 2, 3, 4
  if which_set == 2 then
    m1, m2, m3, m4 = m3, m4, m1, m2
  end

  local h1 = polygon_edge_length(p)/2
  local diag1 = utils.lerp(1/cos(pi/p), 1, DIAG_GIZMO_FACTOR)

  local ax1 = ({axis_a, axis_b})[which_set]
  local ax2 = sym:thru(m1, m2):transform(ax1)
  for _, axis1, axis2, twist_transform in sym.chiral:orbit(ax1, ax2, sym:thru(m4, m3)) do
    self.twists:add(axis1, twist_transform, {
      name = '_' .. axis2.name,
      gizmo_pole_distance = h1,
    })
  end
  local ax2 = ({axis_b, axis_a})[which_set]
  for _, axis1, axis2, twist_transform in sym.chiral:orbit(ax1, ax2, sym:thru(m3, m1)) do
    self.twists:add(axis1, twist_transform, {
      name = '_' .. axis2.name,
      gizmo_pole_distance = 1,
    })
  end
  local ax3 = sym:thru(m4, m3):transform(ax2)
  local ridge = ax2.vector + ax3.vector -- ridge orthogonal to `a1`
  for t, axis, _ridge, twist_transform in sym.chiral:orbit(ax1, ridge, sym:thru(m1, m4)) do
    self.twists:add(axis, twist_transform, {
      name = '_' .. t:transform(ax3).name .. '_' .. t:transform(ax2).name,
      gizmo_pole_distance = diag1,
    })
  end
end

puzzle_generators:add{
  id = 'ft_prism_3',
  version = '0.1.0',

  meta = {
    author = { "Andrew Farkas", "Luna Harran" },
  },

  name = "Face-Turning Triangular Prism (Shallow)",

  params = {
    { name = "Width", type = 'int', default = 3, min = 1, max = 10 },
    { name = "Height", type = 'int', default = 3, min = 1, max = 5 },
  },

  examples = {
    { params = {2,3}, name = "3-Layer Pentahedron" }, -- museum 10916
    { params = {4,5}, name = "5-Layer Pentahedron" }, -- museum 11848
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
    { name = "Polygon size", type = 'int', default = 5, min = 5, max = 24 },
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

puzzle_generators:add{
  id = 'ft_duoprism',
  version = '0.1.0',

  meta = {
    author = { "Andrew Farkas", "Luna Harran" },
  },

  name = "Facet-Turning Polygonal Duoprism (Shallow, 5+)",

  params = {
    { name = "Polygon A", type = 'int', default = 5, min = 5, max = 12 },
    { name = "Polygon B", type = 'int', default = 5, min = 4, max = 12 },
    { name = "Size A", type = 'int', default = 3, min = 1, max = 7 },
    { name = "Size B", type = 'int', default = 3, min = 1, max = 7 },
  },

  examples = {},

  gen = function(params)
    local n = params[1]
    local m = params[2]
    local n_size = params[3]
    local m_size = params[4]

    if n < m or (n == m and n_size < m_size) then
      return 'ft_duoprism', {m, n, m_size, n_size}
    end

    return {
      name = shallow_cut_ft_duoprism_name(n, m, n_size, m_size),

      ndim = 4,
      build = function(self)
        local sym = cd{n, 2, m}

        carve_duoprism(self, n, m)

        local poles_a, poles_b = duoprism_poles(n, m)
        local da = sin(2*pi/n) * polygon_edge_length(n)/2
        local db = sin(2*pi/m) * polygon_edge_length(m)/2
        local axes_a = self.axes:add(poles_a, utils.even_odd_layers(1, 1 - da, n_size))
        local axes_b = self.axes:add(poles_b, utils.even_odd_layers(1, 1 - db, m_size))

        add_duoprism_twists(self, n, m, axes_a[1], axes_b[1], 1)
        add_duoprism_twists(self, n, m, axes_a[1], axes_b[1], 2)
      end,
    }
  end,
}
