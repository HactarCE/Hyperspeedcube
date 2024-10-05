local utils = require('utils')
local symmetries = require('symmetries')

local REALISITIC_PROPORTIONS = true
local CORNER_STALK_SIZE = 0.03


function shallow_ft_dodecahedron(puzzle, layers, scale, basis)
  local shape = symmetries.dodecahedral.dodecahedron(scale, basis)

  local cut_depths
  do
    if layers == 1 then
      cut_depths = {1/phi}
    else
      local outermost_cut
      local aesthetic_limit = 1 - (1 - 1/phi)/layers
      local mechanical_limit = 1
      if REALISITIC_PROPORTIONS then
        mechanical_limit = 1/29 * (10 + 7 * sqrt(5))
      end
      outermost_cut = min(aesthetic_limit, mechanical_limit - CORNER_STALK_SIZE)
      cut_depths = utils.layers.inclusive(outermost_cut, 1/phi, layers)
    end
  end

  local colors, axes = utils.cut_shape(puzzle, shape, cut_depths, prefix)

  return {
    puzzle = puzzle,
    colors = colors,
    axes = axes,
    twist_sets = {
      {
        axis = axes[1],
        symmetry = shape.sym,
        fix = shape.sym.xxx,
        reflections = {
          {shape.sym:thru(1), shape.sym.xoo},
          {shape.sym:thru(2), shape.sym.oxo},
        },
      },
    },
  }
end


function shallow_ft_dodecahedron_cut_depths(size)
  if size == 1 then return {1/phi} end

  local outermost_cut
  local aesthetic_limit = 1 - (1 - 1/phi)/size
  local mechanical_limit = 1
  if REALISITIC_PROPORTIONS then
    mechanical_limit = 1/29 * (10 + 7 * sqrt(5))
  end
  outermost_cut = min(aesthetic_limit, mechanical_limit - CORNER_STALK_SIZE)
  return utils.layers.inclusive(outermost_cut, 1/phi, size)
end

local SHALLOW_FT_DODECAHEDRON_EXAMPLES = {
  { params = {0}, name = "Dodecahedron" },
  { params = {1}, name = "Megaminx" },
  { params = {2}, name = "Gigaminx" },
  { params = {3}, name = "Teraminx" },
  { params = {4}, name = "Petaminx" },
  { params = {5}, name = "Examinx" },
  { params = {6}, name = "Zettaminx" },
  { params = {7}, name = "Yottaminx" },
  { params = {8}, name = "Ronnaminx" },
  {
    params = {9},
    name = "Atlasminx",
    meta = { aliases = { "Quettaminx" } },
  },
  {
    params = {10},
    name = "Minx of Madness", -- no metric prefix
  },
}

SHALLOW_FT_DODECAHEDRA = {}
for _, example in ipairs(SHALLOW_FT_DODECAHEDRON_EXAMPLES) do
  SHALLOW_FT_DODECAHEDRA[example.params[1]] = example
end

puzzle_generators:add{
  id = 'ft_dodecahedron',
  version = '0.1.0',

  name = "N-Layer Megaminx",
  meta = {
    author = {"Andrew Farkas", "Milo Jacquet"},
  },

  params = {
    { name = "Layers", type = 'int', default = 1, min = 0, max = 10 },
  },

  examples = SHALLOW_FT_DODECAHEDRON_EXAMPLES,

  gen = function(params)
    local size = params[1]

    return {
      name = size .. "-Layer Face-Turning Dodecahedron",
      colors = 'dodecahedron',
      ndim = 3,
      build = function(self)
        local sym = cd'h3'

        utils.add_puzzle_twists(shallow_ft_dodecahedron(self, size))

        local center_layer = size + 1
        local R = self.axes.R
        local L = self.axes.L
        local U = self.axes.U
        local F = self.axes.F

        -- Mark piece types
        if size == 0 then
          self:mark_piece{
            region = REGION_ALL,
            name = 'core',
            display = "Core",
          }
        else
          local U_adj = symmetry{self.twists.U}:orbit(R('*')):union()

          -- Centers
          self:add_piece_type{ name = 'center', display = "Center" }
          for i = 2, center_layer do
            for j = 2, size do
              local region
              if i == center_layer then
                region = U(1) & F(j) & ~R(1, size) & ~L(1, size)
              else
                region = U(1) & R(i) & F(j)
              end

              local name, display
              if i == center_layer then
                name, display = string.fmt2('center/t_%d', "T-center (%d)", j-1)
              elseif i == j then
                name, display = string.fmt2('center/x_%d', "X-center (%d)", i-1)
              else
                if i < j then
                  name, display = string.fmt2('center/oblique_%d_%d', "Oblique (%d, %d)", i-1, j-1)
                  self:add_piece_type{ name = name, display = display }
                  name = name .. '/left'
                  display = display .. " (left)"
                else
                  name, display = string.fmt2('center/oblique_%d_%d', "Oblique (%d, %d)", i-1, j-1)
                  name = name .. '/right'
                  display = display .. " (right)"
                end
              end
              self:mark_piece{ region = region, name = name, display = display }
            end
          end

          -- Edges
          self:add_piece_type{ name = 'edge', display = "Edge" }
          for i = 2, size do

            local name, display = string.fmt2('edge/wing_%d', "Wing (%d)", i-1)
            self:mark_piece{
              region = U(1) & F(1) & R(i),
              name = name,
              display = display,
            }
          end

          -- Middle centers and edges
          local middle_suffix = ''
          local center_display, edge_display -- nil is ok here
          if size > 1 then
            middle_suffix = '/middle'
            center_display = "Middle center"
            edge_display = "Middle edge"
          end

          self:mark_piece{
            region = U(1) & ~U_adj,
            name = 'center' .. middle_suffix,
            display = center_display,
          }
          self:mark_piece{
            region = U(1) & F(1) & ~R(1, size) & ~L(1, size),
            name = 'edge' .. middle_suffix,
            display = edge_display,
          }

          self:mark_piece{
            region = U(1) & F(1) & R(1),
            name = 'corner',
            display = "Corner",
          }

          self:unify_piece_types(sym.chiral)
        end
      end,
    }
  end,
}

function define_ft_dodecahedron(size, id, name)
  puzzles:add{
    id = id,
    name = string.format("FT Dodecahedron %d (%s)", size, name),
    version = '0.1.0',
    ndim = 3,
    colors = 'dodecahedron',
  }
end

puzzles:add{
  id = 'megaminx_crystal',
  name = 'Megaminx Crystal',
  version = '0.1.0',
  ndim = 3,
  colors = 'dodecahedron',
  meta = {
    author = 'Milo Jacquet',
  },
  build = function(self)
    local sym = cd'h3'
    local shape = symmetries.dodecahedral.dodecahedron()
    self:carve(shape:iter_poles())

    -- Define axes and slices
    depth = 0.54 -- intermediate puzzle
    self.axes:add(shape:iter_poles(), {depth, -depth})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox.unit], sym:thru(2, 1)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    local R = self.axes.R
    local L = self.axes.L
    local U = self.axes.U
    local F = self.axes.F
    local BR = self.axes.BR
    local DR = self.axes.DR

    self:mark_piece{
      region = U(1) & symmetry{self.twists.U}:orbit(R(2)):intersection(),
      name = 'center',
      display = "Center",
    }
    self:mark_piece{
      region = U(1) & F(1) & R(2) & L(2),
      name = 'megaminx_edge',
      display = "Megaminx edge",
    }
    self:mark_piece{
      region = L(2) & BR(2) & DR(2) & U(1) & R(1) & F(1),
      name = 'corner',
      display = "Corner",
    }
    self:mark_piece{
      region = L(1) & R(1),
      name = 'crystal_edge',
      display = "Crystal edge",
    }
    self:unify_piece_types(sym.chiral)
  end,
}

puzzles:add{
  id = 'pyraminx_crystal',
  name = 'Pyraminx Crystal',
  version = '0.1.0',
  ndim = 3,
  colors = 'dodecahedron',
  meta = {
    author = 'Milo Jacquet',
  },
  build = function(self)
    local sym = cd'h3'
    local shape = symmetries.dodecahedral.dodecahedron()
    self:carve(shape:iter_poles())

    -- Define axes and slices
    local depth = 1/sqrt(5)
    self.axes:add(shape:iter_poles(), {depth, -depth})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox.unit], sym:thru(2, 1)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    local R = self.axes.R
    local L = self.axes.L
    local U = self.axes.U
    local F = self.axes.F
    local BR = self.axes.BR
    local DR = self.axes.DR

    self:mark_piece{
      region = L(2) & BR(2) & DR(2) & U(1),
      name = 'corner',
      display = "Corner",
    }
    self:mark_piece{
      region = L(1) & R(1),
      name = 'edge',
      display = "Edge",
    }
    self:unify_piece_types(sym.chiral)
  end,
}

puzzles:add{
  id = 'curvy_starminx',
  name = 'Curvy Starminx',
  version = '0.1.0',
  ndim = 3,
  colors = 'dodecahedron',
  meta = {
    author = 'Milo Jacquet',
  },
  build = function(self)
    local sym = cd'h3'
    local shape = symmetries.dodecahedral.dodecahedron()
    self:carve(shape:iter_poles())

    -- Define axes and slices
    local depth = 0.33 -- intermediate puzzle
    self.axes:add(shape:iter_poles(), {depth, -depth})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox.unit], sym:thru(2, 1)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    local R = self.axes.R
    local L = self.axes.L
    local U = self.axes.U
    local F = self.axes.F
    local BR = self.axes.BR
    local BL = self.axes.BL
    local DR = self.axes.DR

    self:mark_piece{
      region = L(2) & BR(2) & DR(2) & U(1),
      name = 'corner',
      display = "Corner",
    }
    self:mark_piece{
      region = BR(2) & BL(2) & R(1) & L(1),
      name = 'edge',
      display = "Edge",
    }
    self:mark_piece{
      region = F(2) & R(1) & BR(1) & BL(1) & L(1),
      name = 'x_center',
      display = "X-center",
    }
    self:mark_piece{
      region = F(1) & R(1) & BR(1) & BL(1) & L(1),
      name = 'center',
      display = "Center",
    }
    self:unify_piece_types(sym.chiral)
  end,
}

puzzles:add{
  id = 'starminx',
  name = 'Starminx',
  version = '0.1.0',
  ndim = 3,
  colors = 'dodecahedron',
  meta = {
    author = 'Milo Jacquet',
  },
  build = function(self)
    local sym = cd'h3'
    local shape = symmetries.dodecahedral.dodecahedron()
    self:carve(shape:iter_poles())

    -- Define axes and slices
    local depth = sqrt(5) - 2
    self.axes:add(shape:iter_poles(), {depth, -depth})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox.unit], sym:thru(2, 1)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    local R = self.axes.R
    local L = self.axes.L
    local U = self.axes.U
    local F = self.axes.F
    local BR = self.axes.BR
    local BL = self.axes.BL
    local DR = self.axes.DR

    self:mark_piece{
      region = BR(2) & BL(2) & R(1) & L(1),
      name = 'edge',
      display = "edge",
    }
    self:mark_piece{
      region = U(2) & L(1) & R(1),
      name = 'x_center',
      display = "X-center",
    }
    self:mark_piece{
      region = F(1) & R(1) & BR(1) & BL(1) & L(1),
      name = 'center',
      display = "Center",
    }
    self:unify_piece_types(sym.chiral)
  end,
}


local function pentultimate_cut_depths(size)
  if size == 2 then return {0} end

  local outermost_cut
  local aesthetic_limit = (1 - 2/(size+0.6)) * (sqrt(5) - 2)
  local mechanical_limit = sqrt(5) - 2
  if REALISITIC_PROPORTIONS then
    -- this is the negative of the galois conjugate of the corresponding value for the megaminx
    mechanical_limit = (-10 + 7 * sqrt(5)) / 29
  end
  outermost_cut = min(aesthetic_limit, mechanical_limit - CORNER_STALK_SIZE)
  return utils.layers.inclusive(outermost_cut, -outermost_cut, size-1)
end

puzzle_generators:add{
  id = 'pentultimate',
  version = '0.1.0',

  name = "N-Layer Pentultimate",
  meta = {
    author = { "Milo Jacquet" },
  },

  params = {
    { name = "Layers", type = 'int', default = 2, min = 2, max = 7 },
  },

  examples = {
    { params = {2}, name = "Pentultimate" },
    { params = {3}, name = "Master Pentultimate" },
    { params = {4}, name = "Elite Pentultimate" },
    { params = {5}, name = "Royal Pentultimate" },
    { params = {7}, name = "God Emperor Pentultimate" },
  },

  gen = function(params)
    local size = params[1]

    return {
      name = size .. "-Layer Pentultimate",

      colors = 'dodecahedron',

      ndim = 3,
      build = function(self)
        local sym = cd'h3'
        local shape = symmetries.dodecahedral.dodecahedron()
        self:carve(shape:iter_poles())

        -- Define axes and slices
        self.axes:add(shape:iter_poles(), pentultimate_cut_depths(size))

        -- Define twists
        for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox.unit], sym:thru(2, 1)) do
          self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
        end

        local R = self.axes.R
        local L = self.axes.L
        local U = self.axes.U
        local F = self.axes.F
        local BR = self.axes.BR
        local BL = self.axes.BL
        local DR = self.axes.DR
        local DL = self.axes.DL

        local center_layer = ceil(size/2)

        local middle_prefix = ''
        if size > 3 then
          middle_prefix = 'middle '
        end

        -- Centers
        for i = 2, center_layer do
          for j = i, size+1-i do
            local name, display
            local name2, display2
            if i == j and j*2 - 1 == size then
              if size > 3 then
                name = 'edge/middle'
                display = "Middle edge"
              else
                name = 'edge'
                display = "Edge"
              end
            elseif i == j then
              name, display = string.fmt2('edge/wing_%d', "Wing (%d)", i-1)
            elseif i + j == size+1 then
              name, display = string.fmt2('center/t_%d', "T-center (%d)", i-1)
            else
              name, display = string.fmt2('center/oblique_%d_%d', "Oblique (%d, %d)", i-1, j-1)
              self:add_piece_type{ name = name, display = display }
              name2 = name .. '/right'
              display2 = display .. " (right)"
              name = name .. '/left'
              display = display .. " (left)"
            end
            self:mark_piece{
              region = U(1) & BL(j) & DL(i),
              name = name,
              display = display,
            }
            if name2 ~= nil then
              self:mark_piece{
                region = U(1) & BL(i) & DL(j),
                name = name2,
                display = display2,
              }
            end
          end
        end

        for i = 2, floor(size/2) do
          local name, display = string.fmt2('center/x_outer_%d', "Outer X-center (%d)", i-1)
          self:mark_piece{
            region = DR(i) & L(1) & BR(1),
            name = name,
            display = display
          }
          name, display = string.fmt2('center/x_inner_%d', "Inner X-center (%d)", i-1)
          self:mark_piece{
            region = DR(size+1-i) & L(1) & BR(1),
            name = name,
            display = display
          }
        end

        if size % 2 == 1 then
          local name, display
          if size > 3 then
            name = 'center/x_middle'
            display = "Middle X-center"
          else
            name = 'center/x'
            display = "X-center"
          end
          name = self:mark_piece{
            region = DR(center_layer) & L(1) & BR(1),
            name = name,
            display = display,
          }
        end

        self:mark_piece{
          region = F(1) & R(1) & BR(1) & BL(1) & L(1),
          name = 'center',
          display = "Center",
        }
        self:mark_piece{
          region = L(1) & BR(1) & DR(1),
          name = 'corner',
          display = "Corner",
        }
        self:unify_piece_types(sym.chiral)
      end,
    }
  end,
}
