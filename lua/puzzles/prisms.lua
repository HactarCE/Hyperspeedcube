local utils = lib.utils
local polygonal = lib.symmetries.polygonal
local linear = lib.symmetries.linear

local dodecahedral = lib.symmetries.h3
local ft_dodecahedra = lib.puzzles.ft_dodecahedra

-- TODO: variant of duoprism with factor of `polygon_edge_length(m)/2` and `polygon_edge_length(n)/2`

VERSION = '0.1.0'
TAGS = {
  author = { "Andrew Farkas", "Luna Harran" },
  experimental = true,
}

PARAMS = {
  polygon_size = function(name, min) return { name = name, type = 'int', default = 5, min = min or 3, max = 24 } end,
  polygon_width = function(name) return { name = name, type = 'int', default = 3, min = 1, max = 10 } end,
  line_height = function(name) return { name = name, type = 'int', default = 3, min = 1, max = 10 } end,
}

FACET_GIZMO_EDGE_FACTOR = 2/3
RIDGE_GIZMO_FACTOR = 1/2

function get_default_color(color)
  local t = {
    -- 3D
    U = "Mono Dyad [1]",
    D = "Mono Dyad [2]",
    F = "Rainbow [0/0]",

    -- 4D
    A = "Dark Rainbow [0/0]",
    B = "Light Rainbow [0/0]",
  }
  return t[color.name.canonical:sub(1, 1)]
end

function facet_order(color_or_axis)
  local s = color_or_axis.name.canonical
  if s == 'U' then
    return -2
  elseif s == 'D' then
    return -1
  else
    -- 1,000,000 still has integer precision on f32, but is way more than the
    -- number of faces we should ever have
    return utils.uppercase_name_to_n(s:sub(1, 1)) * 1000000 + utils.uppercase_name_to_n(s:sub(2))
  end
end


function ft_prism_name(n, width, height, cut_type)
  local name = string.format("%s Prism", polygonal.ngonal_name(n))
  if width > 1 or height > 1 then
    name = string.format("Face-Turning %s (%s %dx%d)", name, cut_type, width, height)
  end
  return name
end

function ft_duoprism_name(n, m, n_size, m_size, n_cut_type, m_cut_type)
  local name = string.format("Facet-Turning {%d}x{%d} Duoprism (", n, m)
  if m_cut_type == nil or n_cut_type == m_cut_type then
    name = name .. string.format("%s %dx%d", n_cut_type, n_size, m_size)
  else
    name = name .. string.format("%s %d x %s %d", n_cut_type, n_size, m_cut_type, m_size)
  end
  name = name .. ")"
  return name
end

function build_prism_puzzle(self, n, polygon_cut_depths, height)
  local base_polygon = polygonal.ngon(n, 1)
  local h = base_polygon.edge_length / 2
  local line = linear.line(h, 'z', 'U', 'D')

  local line_cut_depths = utils.layers.inclusive(h, -h, height)

  local base_colors, base_axes = utils.cut_ft_shape(self, line, line_cut_depths, 'U', 'D')
  local side_colors, side_axes = utils.cut_ft_shape(self, base_polygon, polygon_cut_depths, 'F')

  self.colors:reorder(facet_order)
  self.colors:set_defaults(get_default_color)
  self.axes:reorder(facet_order)

  local sym = cd{n, 2}

  local U = base_axes[1]
  local F1 = side_axes[1]

  local function add_twist_set(axis, twist_transform, twist_data)
    for t in sym:orbit(axis) do
      self.twists:add(t:transform(axis), t:transform_oriented(twist_transform), twist_data)
    end
  end

  add_twist_set(U, sym:thru(2, 1), {gizmo_pole_distance = h})
  add_twist_set(F1, sym:thru(3, 1), {gizmo_pole_distance = 1})
end

function build_duoprism_puzzle(self, n, m, n_cut_depths, m_cut_depths, n_opposite_cut_depths, m_opposite_cut_depths)
  local polygon_a = polygonal.ngon(n, 1, 'xy')
  local polygon_b = polygonal.ngon(m, 1, 'zw')

  local _colors_a, axes_a = utils.cut_ft_shape(self, polygon_a, n_cut_depths, 'A')
  local _colors_b, axes_b = utils.cut_ft_shape(self, polygon_b, m_cut_depths, 'B')

  local z1, y1
  if n_opposite_cut_depths ~= nil then
    local axes_z = self.axes:add(polygon_a:iter_vertices('Z'), n_opposite_cut_depths)
    z1 = axes_z[1]
  end
  if m_opposite_cut_depths ~= nil then
    local axes_y = self.axes:add(polygon_b:iter_vertices('Y'), m_opposite_cut_depths)
    y1 = axes_y[1]
  end

  self.colors:reorder(facet_order)
  self.colors:set_defaults(get_default_color)
  self.axes:reorder(facet_order)

  local sym = cd{n, 2, m}

  local function add_twist_set(orbit_sym, twist_mirrors, axis, neighbor_axes, gizmo_pole_distance)
    local twist_transform = sym:thru(table.unpack(twist_mirrors))

    -- generate a point that is fixed by the twist transform
    local coset_vec = vec()
    for i = 1, #sym.mirror_vectors do coset_vec = coset_vec + vec{[i] = 1} end
    for _, m in ipairs(twist_mirrors) do coset_vec = coset_vec - vec{[m] = 1} end
    local coset_point = sym:vec(coset_vec)

    for t in orbit_sym:orbit(coset_point) do
      local name = ''
      for _, neighbor in ipairs(neighbor_axes) do
        name = name .. '_' .. t:transform(neighbor).name
      end

      self.twists:add(t:transform(axis), t:transform_oriented(twist_transform), {
        name = name,
        gizmo_pole_distance = gizmo_pole_distance
      })
    end
  end

  -- Gizmo pole distances
  local gizmo_base_a = polygon_a.edge_length / 2
  local gizmo_base_b = polygon_b.edge_length / 2
  local gizmo_edge_a = utils.lerp(polygon_a.outradius, 1, FACET_GIZMO_EDGE_FACTOR)
  local gizmo_edge_b = utils.lerp(polygon_b.outradius, 1, FACET_GIZMO_EDGE_FACTOR)

  -- Symmetry that respects the orientations of both `polygon_a` and `polygon_b`
  local chiral_sym = symmetry{sym:thru(2, 1), sym:thru(4, 3)}

  if not (axes_a and axes_b) then return end -- TODO: handle piece types

  local a1 = axes_a[1]
  local a2 = sym:thru(2, 1):transform(a1)

  local b1 = axes_b[1]
  local b2 = sym:thru(4, 3):transform(b1)

  -- A twists
  add_twist_set(sym,        {3, 4}, a1, {a2}, gizmo_base_a)
  add_twist_set(chiral_sym, {3, 1}, a1, {b1}, 1)
  add_twist_set(chiral_sym, {1, 4}, a1, {b1, b2}, gizmo_edge_b)

  -- B twists
  add_twist_set(sym,        {1, 2}, b1, {b2}, gizmo_base_b)
  add_twist_set(chiral_sym, {1, 3}, b1, {a1}, 1)
  add_twist_set(chiral_sym, {3, 2}, b1, {a1, a2}, gizmo_edge_a)

  -- Z twists (opposite A)
  if z1 then
    add_twist_set(sym,        {4, 3}, z1, {a1}, RIDGE_GIZMO_FACTOR)
    add_twist_set(chiral_sym, {2, 3}, z1, {b1}, 1)
    add_twist_set(chiral_sym, {4, 2}, z1, {b1, b2}, gizmo_edge_b)
  end

  -- Y twists (opposite B)
  if y1 then
    add_twist_set(sym,        {2, 1}, y1, {b1}, RIDGE_GIZMO_FACTOR)
    add_twist_set(chiral_sym, {4, 1}, y1, {a1}, 1)
    add_twist_set(chiral_sym, {2, 4}, y1, {a1, a2}, gizmo_edge_a)
  end
end




-- PRISM GENERATORS

-- Face-Turning Polygonal Prism (Shallow)
puzzle_generators:add{
  id = 'ft_prism',
  version = VERSION,
  tags = TAGS,
  name = "Face-Turning Polygonal Prism (Shallow)",
  params = {
    PARAMS.polygon_size("Polygon size"),
    PARAMS.polygon_width("Width"),
    PARAMS.line_height("Height"),
  },
  gen = function(params)
    local n, width, height = table.unpack(params)
    return {
      name = ft_prism_name(n, width, height, "Shallow"),
      ndim = 3,
      build = function(self)
        local n_cuts = polygonal.ngon(n):shallow_cut_depths(width)
        build_prism_puzzle(self, n, n_cuts, height)
      end,
    }
  end,
}

-- Face-Turning Triangular Prism (Triminx)
puzzle_generators:add{
  id = 'ft_prism_3_minx',
  version = VERSION,
  tags = TAGS,
  name = "Face-Turning Triangular Prism (Triminx)",
  params = { PARAMS.polygon_width("Width"), PARAMS.line_height("Height") },
  gen = function(params)
    local width, height = table.unpack(params)
    return {
      name = ft_prism_name(3, width, height, "Triminx"),
      ndim = 3,
      build = function(self)
        local n_cuts = polygonal.ngon(3):full_cut_depths(width)
        build_prism_puzzle(self, 3, n_cuts, height)
      end,
    }
  end,
}



-- DUOPRISM GENERATORS

-- Facet-Turning Polygonal Duoprism (Shallow)
puzzle_generators:add{
  id = 'ft_duoprism',
  version = VERSION,
  tags = TAGS,
  name = "Facet-Turning Polygonal Duoprism (Shallow)",
  params = {
    PARAMS.polygon_size("Polygon A"),
    PARAMS.polygon_size("Polygon B"),
    PARAMS.polygon_width("Size A"),
    PARAMS.polygon_width("Size B"),
  },
  gen = function(params)
    local n, m, n_size, m_size = table.unpack(params)
    if n < m or (n == m and n_size < m_size) then
      return 'ft_duoprism', {m, n, m_size, n_size}
    end
    return {
      name = ft_duoprism_name(n, m, n_size, m_size, "Shallow"),
      ndim = 4,
      build = function(self)
        local n_cuts = polygonal.ngon(n):shallow_cut_depths(n_size)
        local m_cuts = polygonal.ngon(m):shallow_cut_depths(m_size)
        build_duoprism_puzzle(self, n, m, n_cuts, m_cuts)
      end,
    }
  end,
}

-- Facet-Turning Onehundredagonal Duoprism
puzzle_generators:add{
  id = 'ft_duoprism_100_4',
  version = VERSION,
  tags = TAGS,
  name = "Facet-Turning Onehundredagonal Duoprism",
  params = {
    { name = "Size (100)", type = 'int', default = 3, min = 1, max = 3 },
    { name = "Size (4)", type = 'int', default = 3, min = 1, max = 3 },
  },
  examples = {
    { params = {1, 1} },
    { params = {3, 3} },
  },
  gen = function(params)
    local n_size, m_size = table.unpack(params)
    return {
      name = ft_duoprism_name(100, 4, n_size, m_size, "Shallow"),
      ndim = 4,
      build = function(self)
        local n_cuts = polygonal.ngon(100):shallow_cut_depths(n_size)
        local m_cuts = polygonal.ngon(4):shallow_cut_depths(m_size)
        build_duoprism_puzzle(self, 100, 4, n_cuts, m_cuts)
      end,
    }
  end,
}

-- Facet-Turning Polygonal Duoprism (Shallow, Triminx)
puzzle_generators:add{
  id = 'ft_duoprism_3_minx',
  version = VERSION,
  tags = TAGS,
  name = "Facet-Turning Polygonal Duoprism (Shallow, Triminx)",
  params = {
    PARAMS.polygon_size("Polygon A"),
    PARAMS.polygon_width("Size A"),
    PARAMS.polygon_width("Size (3)"),
  },
  gen = function(params)
    local n, n_size, m_size = table.unpack(params)
    return {
      name = ft_duoprism_name(n, 3, n_size, m_size, "Shallow", "Triminx"),
      ndim = 4,
      build = function(self)
        local n_cuts = polygonal.ngon(n):shallow_cut_depths(n_size)
        local m_cuts, m_opp_cuts = polygonal.ngon(3):full_cut_depths(m_size)
        build_duoprism_puzzle(self, n, 3, n_cuts, m_cuts, nil, m_opp_cuts)
      end,
    }
  end,
}

-- Facet-Turning Triangular Duoprism (Triminx)
puzzle_generators:add{
  id = 'ft_duoprism_3_minx_3_minx',
  version = VERSION,
  tags = TAGS,
  name = "Facet-Turning Triangular Duoprism (Triminx)",
  params = {
    PARAMS.polygon_width("Size A"),
    PARAMS.polygon_width("Size B"),
  },
  gen = function(params)
    local n_size, m_size = table.unpack(params)
    return {
      name = ft_duoprism_name(3, 3, n_size, m_size, "Triminx", "Triminx"),
      ndim = 4,
      build = function(self)
        local n_cuts, n_opposite_cuts = polygonal.ngon(3):full_cut_depths(n_size)
        local m_cuts, m_opposite_cuts = polygonal.ngon(3):full_cut_depths(m_size)
        build_duoprism_puzzle(self, 3, 3, n_cuts, m_cuts, n_opposite_cuts, m_opposite_cuts)
      end,
    }
  end,
}



-- MEGAMINX PRISM GENERATOR

DODECAHEDRAL_PRISM_FACET_COLORS = {}
for k, v in pairs(dodecahedral.DODECAHEDRON_FACE_COLORS) do
  DODECAHEDRAL_PRISM_FACET_COLORS[k] = v
end
table.insert(DODECAHEDRAL_PRISM_FACET_COLORS, {
  name = 'O', display = "Out", default = "Brown",
})
table.insert(DODECAHEDRAL_PRISM_FACET_COLORS, {
  name = 'I', display = "In",  default = "Teal",
})
color_systems:add{
  id = 'dodecahedron_prism',
  name = "Dodecahedral Prism",

  colors = DODECAHEDRAL_PRISM_FACET_COLORS,
}

function dodecahedron(scale, basis)
  return {
    sym = cd('h3', basis),
    iter_poles = function(self, prefix)
      return self.sym:orbit(self.sym.oox.unit * (scale or 1)):named({
        F = {},
        U = {3, 'F'},
        R = {2, 'U'},
        L = {1, 'R'},
        DR = {2, 'L'},
        DL = {1, 'DR'},
        BR = {3, 'DR'},
        BL = {3, 'DL'},
        PR = {2, 'BL'},
        PL = {1, 'PR'},
        PD = {2, 'PL'},
        PB = {3, 'PD'},
      }):prefixed(prefix)
    end,
  }
end


-- N-Layer Megaminx Prism
puzzle_generators:add{
  id = 'ft_dodecahedron_prism',
  version = VERSION,
  tags = TAGS,
  name = "N-Layer Megaminx Prism",
  params = {
    { name = "Dodecahedron layers", type = 'int', default = 1, min = 0, max = 10 },
    PARAMS.line_height("Prism layers"),
  },
  examples = { { params = {1, 3} } },
  gen = function(params)
    local dodecahedron_size, prism_size = table.unpack(params)

    local dodeca_info = ft_dodecahedra.SHALLOW_FT_DODECAHEDRA[dodecahedron_size]
    local funny_name = dodeca_info and dodeca_info.name or "Megaminx"

    return {
      -- TODO: better names that depend on megaminx layers
      name = string.format("%s Prism (%dx%d)", funny_name, dodecahedron_size, prism_size),
      colors = 'dodecahedron_prism',
      ndim = 4,
      build = function(self)
        local dodeca = dodecahedral.dodecahedron()
        local dodeca_cuts = ft_dodecahedra.shallow_ft_dodecahedron_cut_depths(dodecahedron_size)

        local line = linear.line(1, 'w')
        local line_cuts = utils.layers.inclusive(1, -1, prism_size)

        local dodeca_colors, dodeca_axes = utils.cut_ft_shape(self, dodeca, dodeca_cuts)
        local base_colors, base_axes = utils.cut_ft_shape(self, line, line_cuts, 'O', 'I')

        local sym = cd{5, 3, 2}

        local dodeca1 = dodeca_axes[1]
        local dodeca2 = sym:thru(3):transform(dodeca1)
        local dodeca3 = sym:thru(2):transform(dodeca2)
        local base1 = base_axes[1]

        for t in sym.chiral:orbit(sym.ooxx) do
          self.twists:add(t:transform(base1), t:transform_oriented(sym:thru(1, 2)), {
            name = t:transform(dodeca1).name,
            gizmo_pole_distance = 1,
          })
        end

        for t in sym.chiral:orbit(sym.oxox) do
          self.twists:add(t:transform(base1), t:transform_oriented(sym:thru(1, 3)), {
            name = t:transform(dodeca1).name .. t:transform(dodeca2).name,
            gizmo_pole_distance = 1.08, -- TODO: correct number
          })
        end

        for t in sym.chiral:orbit(sym.xoox) do
          self.twists:add(t:transform(base1), t:transform_oriented(sym:thru(2, 3)), {
            name = t:transform(dodeca1).name .. t:transform(dodeca2).name .. t:transform(dodeca3).name,
            gizmo_pole_distance = 1.12, -- TODO: correct number
          })
        end



        for t in sym.chiral:orbit(sym.ooxx) do
          self.twists:add(t:transform(dodeca1), t:transform_oriented(sym:thru(2, 1)), {
            name = t:transform(base1).name,
            gizmo_pole_distance = 1,
          })
        end

        local dodeca2 = sym:thru(3):transform(dodeca1)
        local dodeca3 = sym:thru(2):transform(dodeca2)

        local ridge_distance = tan(acos(dodeca1.vector:dot(dodeca2.vector))/2)
        local edge_distance = ridge_distance / cos(pi/5)

        for t in sym.chiral:orbit(sym.oxxo) do
          self.twists:add(t:transform(dodeca1), t:transform_oriented(sym:thru(4, 1)), {
            name = t:transform(dodeca2).name,
            gizmo_pole_distance = ridge_distance,
          })
        end

        for t in sym.chiral:orbit(sym.xoxo) do
          self.twists:add(t:transform(dodeca1), t:transform_oriented(sym:thru(2, 4)), {
            name = t:transform(dodeca2).name .. t:transform(dodeca3).name,
            gizmo_pole_distance = utils.lerp(edge_distance, ridge_distance, RIDGE_GIZMO_FACTOR),
          })
        end
      end,
    }
  end,
}
