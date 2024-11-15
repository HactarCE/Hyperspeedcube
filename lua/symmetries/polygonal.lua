local utils = lib.utils

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

local function face_names(n, mirror2, mirror1)
  local m2, m1 = mirror2 or 2, mirror1 or 1
  local names = { [utils.nth_uppercase_name(1)] = {} }
  for i = 2, n do
    names[utils.nth_uppercase_name(i)] = {m2, m1, utils.nth_uppercase_name(i-1)}
  end
  return names
end

-- Length of an edge
local function edge_length(n)
  return 2 * tan(pi / n)
end

-- Length of an edge when projected perpendicular to an adjacent edge
local function edge_depth(n)
  return sin(2 * pi / n) * edge_length(n)
end

local function outradius(n)
  return 1 / cos(pi / n)
end

-- Constructs an N-gon with inradius `scale`
function ngon(n, scale, basis)
  local scale = scale or 1

  local diameter
  if n % 2 == 0 then
    diameter = 2
  else
    diameter = 1 + 1 / cos(pi / n)
  end

  return {
    sym = cd({n}, basis),
    iter_poles = function(self, prefix)
      return self.sym:orbit(self.sym.ox.unit * scale):named(face_names(n)):prefixed(prefix)
    end,
    iter_vertices = function(self, prefix)
      return self.sym:orbit(self.sym.xo.unit * scale):named(face_names(n, 1, 2)):prefixed(prefix)
    end,

    edge_length = edge_length(n) * scale,
    edge_depth = edge_depth(n) * scale,
    outradius = outradius(n) * scale,
    diameter = diameter * scale,

    shallow_cut_depths = function(self, layer_count)
      if n == 3 then
        local cut_depths = utils.layers.even_odd(1, 0, layer_count)
        table.insert(cut_depths, -2)
        return cut_depths
      else
        local max_cut_depth = 1 - self.edge_depth / 2
        return utils.layers.even_odd(1, max_cut_depth, layer_count)
      end
    end,

    full_cut_depths = function(self, layer_count)
      local max_cut_depth = 1 - self.edge_depth
      local ret1 = utils.layers.double_ended(scale, max_cut_depth * scale, layer_count)
      local ret2
      if n == 3 then ret2 = utils.layers.double_ended(-max_cut_depth * scale, -scale, layer_count) end

      -- local start, stop = 1, 1 - self.outradius
      -- local ret1 = utils.layers.double_ended(start * scale, stop * scale, layer_count)
      -- local ret2 = utils.layers.double_ended(-stop * scale, -start * scale, layer_count)
      return ret1, ret2
    end,
  }
end
