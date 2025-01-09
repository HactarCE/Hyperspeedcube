local utils = lib.utils

local REALISITIC_PROPORTIONS = true
local CORNER_STALK_SIZE = 0.03

CRYSTAL_DEPTH = 1/sqrt(5)
MEGAMINX_DEPTH = 1/phi

function shallow_ft_dodecahedron_cut_depths(layers)
  if layers == 0 then
    return {}
  elseif layers == 1 then
    return {1, MEGAMINX_DEPTH}
  else
    local outermost_cut
    local aesthetic_limit = 1 - (1 - MEGAMINX_DEPTH)/layers
    local mechanical_limit = 1
    if REALISITIC_PROPORTIONS then
      mechanical_limit = 1/29 * (10 + 7 * sqrt(5))
    end
    outermost_cut = min(aesthetic_limit, mechanical_limit - CORNER_STALK_SIZE)
    return utils.concatseq({1}, utils.layers.inclusive(outermost_cut, MEGAMINX_DEPTH, layers-1))
  end
end

local function curvy_starminx_cut_depths(size)
  assert(size >= 1)

  local mid_depth = 1/3
  local half_range = sqrt(5) - 2 - 1/3 + 0.15
  return utils.concatseq({1}, utils.layers.exclusive_centered(mid_depth, half_range, size))
end

local function pentultimate_cut_depths(size)
  assert(size >= 2)

  local outermost_cut
  local aesthetic_limit = (1 - 2/(size+0.6)) * (sqrt(5) - 2)
  local mechanical_limit = sqrt(5) - 2
  if REALISITIC_PROPORTIONS then
    -- this is the negative of the galois conjugate of the corresponding value for the megaminx
    mechanical_limit = (-10 + 7 * sqrt(5)) / 29
  end
  outermost_cut = min(aesthetic_limit, mechanical_limit - CORNER_STALK_SIZE)
  return utils.layers.inclusive_inf(outermost_cut, -outermost_cut, size)
end

-- Cut shape and add twists
function construct_ft_dodecahedron(puzzle, cut_depths, scale, basis)
  local shape = lib.symmetries.dodecahedral.dodecahedron(scale, basis)

  local colors, axes = utils.cut_ft_shape(puzzle, shape, cut_depths)

  -- Define twists
  if axes then
    for t, ax, rot in shape.sym.chiral:orbit(axes[1], shape.sym:thru(2, 1)) do
      puzzle.twists:add(ax, rot, { gizmo_pole_distance = 1 })
    end
  end

  return shape, colors, axes
end

local SHALLOW_FT_DODECAHEDRON_EXAMPLES = {
  { params = {0}, name = "Dodecahedron" },
  { params = {1}, name = "Megaminx",
    aliases = { "Hungarian Supernova" },
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
  { params = {9}, name = "Atlasminx",
    aliases = { "Quettaminx" },
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
local KILOMINX_EXAMPLES = {
  { params = {1}, name = "Kilominx",
    aliases = { "Duominx" },
    tags = {
      inventor = "Thomas de Bruin",
      external = { gelatinbrain = '1.1.12', museum = 1600 },
    },
  },
  { params = {2}, name = "Master Kilominx",
    tags = {
      external = { museum = 2325 },
    }
  },
  { params = {3}, name = "Elite Kilominx",
    tags = {
      external = { museum = 2377 },
    }
  },
  { params = {4}, name = "Royal Kilominx" },
}

SHALLOW_FT_DODECAHEDRA = {}
for _, example in ipairs(SHALLOW_FT_DODECAHEDRON_EXAMPLES) do
  SHALLOW_FT_DODECAHEDRA[example.params[1]] = example
end

-- N-Layer Megaminx generator
puzzle_generators:add{
  id = 'ft_dodecahedron',
  version = '1.0.0',
  name = "N-Layer Megaminx",
  colors = "dodecahedron",
  params = {
    { name = "Layers", type = 'int', default = 1, min = 0, max = 10 },
  },
  gen = function(params)
    local size = params[1]

    return {
      name = size .. "-Layer Face-Turning Dodecahedron",
      ndim = 3,
      build = function(self)
        local cut_depths = shallow_ft_dodecahedron_cut_depths(size)
        local shape = construct_ft_dodecahedron(self, cut_depths)

        if size == 0 then
          lib.piece_types.mark_everything_core(self)
          return
        end

        -- Mark piece types
        lib.piece_types.triacron_subsets.mark_multilayer_UFRL(self, 2*size + 1)
        self:unify_piece_types(shape.sym.chiral) -- chiral because left vs. right obliques
      end,

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
    }
  end,

  examples = SHALLOW_FT_DODECAHEDRON_EXAMPLES,

  tags = {
    builtin = '2.0.0',
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
}

-- N-Layer Kilominx generator
puzzle_generators:add{
  id = 'kilominx',
  version = '1.0.0',
  name = "N-Layer Kilominx",
  colors = "dodecahedron",
  params = {
    { name = "Layers", type = 'int', default = 1, min = 1, max = 10 },
  },
  gen = function(params)
    local size = params[1]

    return {
      ndim = 3,

      build = function(self)
        local cut_depths = shallow_ft_dodecahedron_cut_depths(size)
        local shape = construct_ft_dodecahedron(self, cut_depths)

        -- Mark piece types
        utils.unpack_named(_ENV, self.axes)
        local UFR_adj = REGION_NONE
        lib.piece_types.triacron_subsets.mark_multilayer_corners(self, size, U, F, R, UFR_adj)
        self:unify_piece_types(shape.sym.chiral)
        self:delete_untyped_pieces() -- delete centers & edges
      end,

      tags = {
        'type/puzzle',
        completeness = { super = size <= 1 },
      },
    }
  end,

  examples = KILOMINX_EXAMPLES,

  tags = {
    builtin = '2.0.0',
    external = { '!gelatinbrain', '!hof', '!mc4d', '!museum', '!wca' },

    author = "Andrew Farkas",
    '!inventor',

    'shape/3d/platonic/dodecahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire',
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/dodecahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!real', '!laminated', '!complex' },
    cuts = { depth = { 'shallow' }, '!stored', '!wedge' },
    turns_by = { 'face', 'facet' },
    '!experimental',
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },
}

-- Megaminx-Crystal Intermediate
puzzles:add{
  -- between a Megaminx Crystal (which has no centers) and a Pyraminx Crystal (which has no edges)
  id = 'megaminx_crystal_intermediate',
  version = '0.1.0',
  name = "Megaminx-Crystal Intermediate",
  ndim = 3,
  colors = 'dodecahedron',
  build = function(self)
    local t = cos(pi/10) * tan(pi/5) / (2 - sin(pi/10))
    local depth = utils.lerp(MEGAMINX_DEPTH, CRYSTAL_DEPTH, t)
    local cut_depths = {1, depth, -depth, -1}
    local shape = construct_ft_dodecahedron(self, cut_depths)

    do -- Mark piece types
      utils.unpack_named(_ENV, self.axes)

      local region = U(1) & symmetry{self.twists.U}:orbit(R(2)):intersection()
      self:mark_piece(region, 'center', "Center")

      local region = U(1) & F(1) & R(2) & L(2)
      self:mark_piece(region, 'megaminx_edge', "Megaminx edge")

      local region = L(2) & BR(2) & DR(2) & U(1) & R(1) & F(1)
      self:mark_piece(region, 'corner', "Corner")

      local region = L(1) & R(1)
      self:mark_piece(region, 'crystal_edge', "Crystal edge")

      self:unify_piece_types(shape.sym.chiral)
    end
  end,

  tags = {
    builtin = '2.0.0',
    external = { gelatinbrain = '1.1.2', '!hof', '!mc4d', '!museum', '!wca' },

    author = "Milo Jacquet",
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
}

-- Pyraminx Crystal
puzzles:add{
  id = 'pyraminx_crystal',
  version = '1.0.0',
  name = 'Pyraminx Crystal',
  ndim = 3,
  colors = 'dodecahedron',
  build = function(self)
    local depth = 1/sqrt(5)
    local cut_depths = {1, CRYSTAL_DEPTH, -CRYSTAL_DEPTH, -1}
    local shape = construct_ft_dodecahedron(self, cut_depths)

    -- Mark piece types
    utils.unpack_named(_ENV, self.axes)
    self:mark_piece(L(1) & R(1), 'edge', "Edge")
    self:mark_piece(L(2) & BR(2) & DR(2) & U(1), 'corner', "Corner")
    self:unify_piece_types(shape.sym.chiral)
  end,

  tags = {
    builtin = '2.0.0',
    external = { gelatinbrain = '1.1.3', '!hof', '!mc4d', museum = 652, '!wca' },

    author = "Milo Jacquet",
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
}

-- N-Layer Curvy Starminx generator
puzzle_generators:add{
  id = 'curvy_starminx',
  version = '0.1.0',
  name = "N-Layer Curvy Starminx",
  params = {
    { name = "Layers", type = 'int', default = 1, min = 1, max = 13 },
  },
  gen = function(params)
    local size = params[1]
    return {
      name = size .. "-Layer Curvy Starminx",
      colors = 'dodecahedron',
      ndim = 3,
      build = function(self)
        local cut_depths = curvy_starminx_cut_depths(size)
        local shape = construct_ft_dodecahedron(self, cut_depths)

        -- Mark piece types
        lib.utils.unpack_named(_ENV, self.axes)

        self:add_piece_type('center', "Center")
        self:add_piece_type('edge', "Edge")
        self:add_piece_type('point', "Point")
        self:add_piece_type('corner', "Corner")

        -- Center (to starminx point)
        self:mark_piece(F(1) & R(1) & BR(1) & BL(1) & L(1), 'center/middle', "Center")
        for i=0, size-1 do
          for j=0, size-1 do
            if j>0 then
              local x = BR(i+1)
              local y = BL(j+1)
              local region = F(1) & R(1) & L(1) & x & y
              self:mark_piece(region, string.fmt2('center/cpe_%d_%d', "CPE (%d, %d)", i, j))
            end
          end
        end

        -- Point (to corner)
        self:mark_piece(~F('*') & R(1) & BR(1) & BL(1) & L(1), 'point/middle', "Point")
        for i=0, size-1 do
          for j=0, size-1 do
            if i>0 or j>0 then
              local x = R(i+1)
              local y = L(j+1)
              local region = ~F('*') & BR(1) & BL(1) & x & y
              self:mark_piece(region, string.fmt2('point/pev_%d_%d', "PEV (%d, %d)", i, j))
            end
          end
        end

        -- Edge (to corner)
        local base = ~(BR('*') | BL('*') | DL('*') | DR('*')) & L(1)
        self:mark_piece(base & R(1), 'edge/middle', "Edge")
        for i=1, size-1 do
          local x = R(i+1)
          local region = base & x
          self:mark_piece(region, string.fmt2('edge/ev_%d', "EV (%d)", i))
        end

        -- Corner
        self:mark_piece(~L('*') & ~BR('*') & ~DR('*') & U(1), 'corner', "Corner")

        self:unify_piece_types(shape.sym.chiral)
      end,

      tags = { 'type/puzzle' }
    }
  end,

  examples = {
    {
      params = {1},
      name = "Curvy Starminx",
      aliases = { "Litestarminx" }, -- museum = 11394
      tags = {
        external = { gelatinbrain = '1.1.4', '!hof', '!mc4d', museum = 4344, '!wca' },
        inventor = "Mr. Fok",
      }
    }
  },

  tags = {
    builtin = '2.0.0',

    author = { "Milo Jacquet", "Luna Harran" },

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
    'experimental', -- needs piece type bikeshedding + testing
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },
}

-- Starminx
puzzles:add{
  id = 'starminx',
  version = '1.0.0',
  name = 'Starminx',
  ndim = 3,
  colors = 'dodecahedron',
  build = function(self)
    local depth = sqrt(5) - 2
    local cut_depths = {1, depth, -depth, -1}
    local shape = construct_ft_dodecahedron(self, cut_depths)

    -- Mark piece types
    utils.unpack_named(_ENV, self.axes)
    self:mark_piece(BR(2) & BL(2) & R(1) & L(1), 'edge', "edge")
    self:mark_piece(U(2) & L(1) & R(1), 'x_center', "X-center")
    self:mark_piece(F(1) & R(1) & BR(1) & BL(1) & L(1), 'center', "Center")
    self:unify_piece_types(shape.sym.chiral)
  end,

  tags = {
    builtin = '2.0.0',
    external = { gelatinbrain = '1.1.5', '!hof', '!mc4d', museum = 1759, '!wca' },

    author = "Milo Jacquet",
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
}

-- N-Layer Pentultimate generator
puzzle_generators:add{
  id = 'pentultimate',
  version = '0.1.0',
  name = "N-Layer Pentultimate",
  colors = "dodecahedron",
  params = {
    { name = "Layers", type = 'int', default = 2, min = 2, max = 20 },
  },
  gen = function(params)
    local size = params[1]

    return {
      name = size .. "-Layer Pentultimate",
      ndim = 3,
      build = function(self)
        local cut_depths = pentultimate_cut_depths(size)
        local shape = construct_ft_dodecahedron(self, cut_depths)

        utils.unpack_named(_ENV, self.axes)

        do -- Mark piece types
          self:add_piece_type('center', "Center")

          local center_layer = ceil((size+1)/2)

          -- Middle centers
          local middle_center_region = F(1) & R(1) & BR(1) & BL(1) & L(1)
          if size >= 3 then
            self:add_piece_type('edge', "Edge")
            self:mark_piece(middle_center_region, 'center/0_0', "Middle center")
          else
            self:mark_piece(middle_center_region, 'center')
          end

          -- X-centers
          for i = 1, size-2 do
            local prefix = lib.piece_types.inner_outer_prefix(i, (size-1)/2)
            local name = string.format('center/0_%d', i)
            local display = string.format("%s X-center (%d)", prefix, i)
            self:mark_piece(DR(size-i) & L(1) & BR(1), name, display)
          end

          -- T-centers and oblique centers
          for i = 1, size-3 do
            for j = 1, size-i-2 do
              local region = U(1) & BL(j+1) & DL(size-i)
              self:mark_piece(region, string.fmt2('center/%d_%d', "Center (%d, %d)", i, j))
            end
          end

          -- Edges and wings
          local center_layer = ceil((size+1)/2)
          if size % 2 == 1 then
            local region = U(1) & BL(center_layer) & DL(center_layer)
            if size > 3 then
              self:mark_piece(region, 'edge/middle', "Middle edge")
            else
              self:mark_piece(region, 'edge')
            end
          end
          for i = 1, center_layer-2 do
            local region = U(1) & BL(center_layer-i) & DL(center_layer-i)
            self:mark_piece(region, string.fmt2('edge/%d', "Wing (%d)", i))
          end

          -- Corners
          self:mark_piece(L(1) & BR(1) & DR(1), 'corner', "Corner")

          self:unify_piece_types(shape.sym.chiral)
        end
      end,

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
    }
  end,

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

  tags = {
    builtin = '2.0.0',
    external = { '!gelatinbrain', '!hof', '!mc4d', '!museum', '!wca' },

    author = { "Milo Jacquet", "Andrew Farkas" },
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
}
