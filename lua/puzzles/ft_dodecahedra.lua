local utils = lib.utils

local REALISITIC_PROPORTIONS = true
local CORNER_STALK_SIZE = 0.03


function shallow_ft_dodecahedron(puzzle, layers, scale, basis)
  local shape = lib.symmetries.dodecahedral.dodecahedron(scale, basis)

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
  {
    params = {1},
    name = "Megaminx",
    aliases = "Hungarian Supernova",
    tags = {
      external = {
        gelatinbrain = '1.1.1',
        museum = 650,
        wca = 'minx',
      },
    },
  },
  { params = {2}, name = "Gigaminx",
    tags = {
      inventor = "Tyler Fox",
      external = { gelatinbrain = '1.1.9', museum = 1475 },
    },
  },
  { params = {3}, name = "Teraminx",
    tags = {
      inventor = "Andrew Cormier",
      external = { gelatinbrain = '1.1.41', museum = 1477 },
    },
  },
  { params = {4}, name = "Petaminx",
    tags = {
      inventor = "Andrew Cormier",
      external = { museum = 1647 },
    },
  },
  { params = {5}, name = "Examinx",
    tags = {
      inventor = "Matthew Bahner",
      external = { museum = 4346 },
    },
  },
  { params = {6}, name = "Zettaminx",
    tags = {
      external = { museum = 10786 },
    },
  },
  { params = {7}, name = "Yottaminx",
    tags = {
      inventor = "Matthew Bahner",
      external = { museum = 1185 },
    },
  },
  { params = {8}, name = "Ronnaminx",
    tags = { 'big' }, -- couldn't find on the museum, at time of writing
  },
  {
    params = {9}, name = "Atlasminx", aliases = { "Quettaminx" },
    tags = {
      inventor = "Coren Broughton",
      external = { museum = 9058 },
      'big',
    },
  },
  { params = {10}, name = "Minx of Madness", -- no metric prefix
    -- couldn't find on the museum
    tags = {
      inventor = "Matthew Bahner",
      'big',
    },
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
  colors = "dodecahedron",

  tags = {
    builtin = '1.0.0',
    external = { '!gelatinbrain', '!hof', '!mc4d', '!museum', '!wca' },

    author = { "Andrew Farkas", "Milo Jacquet" },
    '!inventor',

    'shape/3d/platonic/dodecahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire',
      '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/dodecahedral', '!hybrid', '!multicore' },
    colors = { '!multi_facet_per', '!multi_per_facet' },
    cuts = { depth = { 'shallow' }, '!stored', '!wedge' },
    turns_by = { 'face', 'facet' },
    '!experimental',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },

  params = {
    { name = "Layers", type = 'int', default = 1, min = 0, max = 10 },
  },

  examples = SHALLOW_FT_DODECAHEDRON_EXAMPLES,

  gen = function(params)
    local size = params[1]

    return {
      name = size .. "-Layer Face-Turning Dodecahedron",

      tags = {
        ['type/shape'] = size == 0,
        ['type/puzzle'] = size ~= 0,
        algebraic = {
          abelian = size == 0,
          trivial = size == 0,
        },
        canonical = size == 1,
        completeness = {
          complex = size == 0,
          laminated = size == 0,
          real = size <= 1,
          super = size == 0,
        },
        meme = size == 0,
      },

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
                  name, display = string.fmt2('center/oblique_%d_%d', "Oblique (%d, %d)", j-1, i-1)
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
  -- between a Megaminx Crystal (which has no centers) and a Pyraminx Crystal (which has no edges)
  id = 'megaminx_crystal_intermediate',
  name = 'Megaminx-Crystal Intermediate',
  version = '0.1.0',
  ndim = 3,
  colors = 'dodecahedron',

  tags = {
    builtin = '1.0.0',
    external = { gelatinbrain = '1.1.2', '!hof', '!mc4d', '!museum', '!wca' },

    author = {"Milo Jacquet"},
    '!inventor',

    'type/puzzle',
    'shape/3d/platonic/dodecahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire',
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/dodecahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { 'depth/deep', '!stored', '!wedge' },
    turns_by = { 'face', 'facet' },
    '!experimental',
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },

  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.dodecahedral.dodecahedron()
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

  tags = {
    builtin = '1.0.0',
    external = { gelatinbrain = '1.1.3', '!hof', '!mc4d', museum = 652, '!wca' },

    author = {"Milo Jacquet"},
    inventor = "Aleh Hladzilin",

    'type/puzzle',
    'shape/3d/platonic/dodecahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire',
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/dodecahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { 'depth/deep/to_adjacent', '!stored', '!wedge' },
    turns_by = { 'face', 'facet' },
    '!experimental',
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },

  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.dodecahedral.dodecahedron()
    self:carve(shape:iter_poles())

    -- Define axes and slices
    local depth = 1/sqrt(5)
    self.axes:add(shape:iter_poles(), {depth, -depth, -1})

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
  aliases = {"Litestarminx"}, -- museum = 11394
  version = '0.1.0',
  ndim = 3,
  colors = 'dodecahedron',

  tags = {
    builtin = '1.0.0',
    external = { gelatinbrain = '1.1.4', '!hof', '!mc4d', museum = 4344, '!wca' },

    author = {"Milo Jacquet"},
    inventor = "Mr. Fok",

    'type/puzzle',
    'shape/3d/platonic/dodecahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire',
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/dodecahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { 'depth/deep/to_adjacent', '!stored', '!wedge' },
    turns_by = { 'face', 'facet' },
    '!experimental',
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },

  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.dodecahedral.dodecahedron()
    self:carve(shape:iter_poles())

    -- Define axes and slices
    local depth = 0.33 -- intermediate puzzle
    self.axes:add(shape:iter_poles(), {depth, -depth, -1})

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

  tags = {
    builtin = '1.0.0',
    external = { gelatinbrain = '1.1.5', '!hof', '!mc4d', museum = 1759, '!wca' },

    author = {"Milo Jacquet"},
    inventor = "Aleh Hladzilin",

    'type/puzzle',
    'shape/3d/platonic/dodecahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire',
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/dodecahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { 'depth/deep/past_adjacent', '!stored', '!wedge' },
    turns_by = { 'face', 'facet' },
    '!experimental',
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },

  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.dodecahedral.dodecahedron()
    self:carve(shape:iter_poles())

    -- Define axes and slices
    local depth = sqrt(5) - 2
    self.axes:add(shape:iter_poles(), {depth, -depth, -1})

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
  return utils.concatseq(utils.layers.inclusive(outermost_cut, -outermost_cut, size-1), {-1})
end

puzzle_generators:add{
  id = 'pentultimate',
  version = '0.1.0',

  name = "N-Layer Pentultimate",
  colors = "dodecahedron",

  tags = {
    builtin = '1.0.0',
    external = { '!gelatinbrain', '!hof', '!mc4d', '!museum', '!wca' },

    author = {"Milo Jacquet"},
    '!inventor',

    'shape/3d/platonic/dodecahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire',
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/dodecahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { 'depth/deep/past_adjacent', '!stored', '!wedge' },
    turns_by = { 'face', 'facet' },
    '!experimental',
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },

  params = {
    { name = "Layers", type = 'int', default = 2, min = 2, max = 7 },
  },

  examples = {
    { params = {2}, name = "Pentultimate",
      tags = {
        inventor = "Jason Smith",
        external = { gelatinbrain = '1.1.7', museum = 1741 },
      },
    },
    { params = {3}, name = "Master Pentultimate",
      tags = {
        inventor = "Scott Bedard",
        external = { gelatinbrain = '1.1.6', museum = 1906 },
      },
    },
    { params = {4}, name = "Elite Pentultimate",
      tags = {
        inventor = "Raphaël Mouflin",
        external = { museum = 6529 },
      },
    },
    { params = {5}, name = "Royal Pentultimate",
      tags = {
        inventor = "Eric Vergo",
        external = { museum = 1909 },
      },
    },
    { params = {7}, name = "God Emperor Pentultimate",
      tags = { inventor = "Matthew Bahner" },
    },
  },

  gen = function(params)
    local size = params[1]

    return {
      name = size .. "-Layer Pentultimate",

      tags = {
        'type/puzzle',
        algebraic = { '!abelian', '!trivial' },
        completeness = {
          real = size <= 2,
          '!complex', '!laminated', '!super',
        },
        ['cuts/depth/half'] = size % 2 == 0,
        '!meme',
      },

      ndim = 3,
      build = function(self)
        local sym = cd'h3'
        local shape = lib.symmetries.dodecahedral.dodecahedron()
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

        -- Centers and edges
        self:add_piece_type{ name = 'center', display = "Center" }
        self:add_piece_type{ name = 'edge', display = "Edge" }
        if size >= 4 then
          self:add_piece_type{ name = 'center/x', display = "X-center" }
          self:add_piece_type{ name = 'center/diamond', display = "Diamond center" }
        end
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
              name, display = string.fmt2('center/diamond/t_%d', "T-center (%d)", i-1)
            else
              name, display = string.fmt2('center/diamond/oblique_%d_%d', "Oblique (%d, %d)", i-1, j-1)
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
          local name, display = string.fmt2('center/x/outer_%d', "Outer X-center (%d)", i-1)
          self:mark_piece{
            region = DR(i) & L(1) & BR(1),
            name = name,
            display = display
          }
        end
        if size % 2 == 1 then
          local name, display
          if size > 3 then
            name = 'center/x/middle'
            display = "Middle X-center"
          else
            name = 'center/x'
            display = "X-center"
          end
          self:mark_piece{
            region = DR(center_layer) & L(1) & BR(1),
            name = name,
            display = display,
          }
        end
        for i = floor(size/2), 2, -1 do
          local name, display = string.fmt2('center/x/inner_%d', "Inner X-center (%d)", i-1)
          self:mark_piece{
            region = DR(size+1-i) & L(1) & BR(1),
            name = name,
            display = display
          }
        end

        self:mark_piece{
          region = F(1) & R(1) & BR(1) & BL(1) & L(1),
          name = 'center/center',
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
