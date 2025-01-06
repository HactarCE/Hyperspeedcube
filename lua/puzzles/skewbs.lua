local utils = lib.utils

local REALISITIC_PROPORTIONS = true
local CORNER_STALK_SIZE = 0.03

local function skewb_cut_depths(size)
  local outermost_cut
  local aesthetic_limit = (1 - 2/(size+0.6)) * (1 / sqrt(3))
  local mechanical_limit = 1 / sqrt(3)
  if REALISITIC_PROPORTIONS then
    -- this is the negative of the galois conjugate of the corresponding value for the megaminx
    mechanical_limit = sqrt(3) / 5
  end
  outermost_cut = min(aesthetic_limit, mechanical_limit - CORNER_STALK_SIZE)
  return utils.layers.inclusive_inf(outermost_cut, -outermost_cut, size)
end

local function construct_vt_cube(puzzle, cut_depths)
  local cube = lib.symmetries.cubic.cube()
  local octahedron = lib.symmetries.octahedral.octahedron()

  puzzle:carve(cube:iter_poles())

  -- Define axes and slices
  puzzle.axes:add(octahedron:iter_poles(), cut_depths)

  local sym = octahedron.sym

  -- Define twists
  for _, axis, twist_transform in sym.chiral:orbit(puzzle.axes[sym.xoo.unit], sym:thru(3, 2)) do
    puzzle.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
  end

  return cube, octahedron
end

-- N-Layer Skewb generator
puzzle_generators:add{
  id = 'skewb',
  version = '1.0.0',
  name = "N-Layer Skewb",
  params = {
    { name = "Layers", type = 'int', default = 2, min = 2, max = 9 },
  },
  gen = function(params)
    local size = params[1]
    return {
      name = size .. "-Layer Skewb",
      colors = 'cube',
      ndim = 3,
      build = function(self)
        local _cube, octahedron = construct_vt_cube(self, skewb_cut_depths(size))

        utils.unpack_named(_ENV, self.axes)

        do -- Mark piece types
          self:add_piece_type('center', "Center")

          local center_layer = ceil((size+1)/2)

          -- Middle centers
          local middle_center_region = F(1) & R(1) & U(1) & L(1)
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
            local display = string.format('%s X-center (%d)', prefix, i)
            self:mark_piece(U(i+1) & L(1) & R(1), name, display)
          end

          lib.piece_types.unknown_vt_guys.mark_diamond_pieces(self, size, F(1), L, R)

          -- Corners
          self:mark_piece(L(1) & R(1) & BD(1), 'corner', "Corner")

          self:unify_piece_types(octahedron.sym.chiral)
        end
      end,

      tags = {
        'type/puzzle',
        completeness = {
          real = size == 2,
        },
        ['cuts/depth/half'] = size % 2 == 0,
      }
    }
  end,

  examples = {
    { params = {2}, name = "Skewb",
      aliases = { "Pyraminx Cube" },
      tags = {
        external = { gelatinbrain = '3.2.1', museum = 621, wca = 'skewb' },
        inventor = "Tony Durham",
      },
    },
    { params = {3}, name = "Master Skewb",
      tags = {
        external = { gelatinbrain = '3.2.2', museum = 1353 },
        inventor = "Katsuhiko Okamoto",
      }
    },
    { params = {4}, name = "Elite Skewb",
      tags = {
        external = { gelatinbrain = '3.2.3', museum = 2004 },
        inventor = "Andrew Cormier",
      },
    },
    { params = {5}, name = "Royal Skewb" }, -- no museum link? it's been built: https://www.youtube.com/watch?v=g-Orrt6I_2U
    { params = {7},
      tags = {
        external = { museum = 11849 },
        inventor = "Kairis Wu",
      },
    },
  },

  tags = {
    builtin = '2.0.0',
    external = { '!gelatinbrain', '!hof', '!mc4d', '!museum', '!wca' },

    author = { "Milo Jacquet", "Andrew Farkas" },
    '!inventor',

    'shape/3d/platonic/cube',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire',
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/octahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!laminated', '!complex' },
    cuts = { 'depth/deep/past_adjacent', '!stored', '!wedge' },
    turns_by = { 'peak', 'vertex' },
    '!experimental',
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },
}

-- Dino Cube
puzzles:add{
  id = 'dino_cube',
  name = 'Dino Cube',
  version = '1.0.0',
  ndim = 3,
  colors = 'cube',
  build = function(self)
    local _cube, octahedron = construct_vt_cube(puzzle, {INF, 1/sqrt(3), -1/sqrt(3), -INF})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1.1})
    end

    -- Mark piece types
    utils.unpack_named(_ENV, self.axes)
    self:mark_piece(R(1) & U(1), 'edge', "Edge")
    self:unify_piece_types(sym.chiral)
  end,

  tags = {
    builtin = '2.0.0',
    external = { gelatinbrain = '3.2.4', '!hof', '!mc4d', museum = 5020, '!wca' },

    author = { "Milo Jacquet", "Andrew Farkas" },
    inventor = "Robert Webb",

    'type/puzzle',
    'shape/3d/platonic/cube',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire',
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/octahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { 'super', '!real', '!laminated', '!complex' },
    cuts = { 'depth/deep/to_adjacent', '!stored', '!wedge' },
    turns_by = { 'peak', 'vertex' },
    '!experimental',
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },
}

-- Dino Skewb
puzzles:add{
  id = 'dino_skewb',
  name = 'Dino Skewb',
  version = '1.0.0',
  ndim = 3,
  colors = 'cube',
  remove_internals = false,
  build = function(self)
    local cube, octahedron = construct_vt_cube(self, {INF, 1/sqrt(3), 0, -1/sqrt(3), -INF})

    -- Mark piece types
    utils.unpack_named(_ENV, self.axes)
    self:mark_piece(R(1) & U(1) & F(2) & L(2), 'center', "Center")
    self:mark_piece(R(1) & U(1) & F(2) & L(3), 'wing', "Wing")
    self:unify_piece_types(octahedron.sym.chiral)
  end,

  tags = {
    builtin = '2.0.0',
    external = { gelatinbrain = '3.2.4', '!hof', '!mc4d', museum = 5020, '!wca' },

    author = { "Milo Jacquet", "Andrew Farkas" },
    inventor = "Robert Webb",

    'type/puzzle',
    'shape/3d/platonic/cube',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire',
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/octahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { 'super', '!real', '!laminated', '!complex' },
    cuts = { 'depth/deep/to_adjacent', 'depth/half', '!stored', '!wedge' },
    turns_by = { 'peak', 'vertex' },
    '!experimental',
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },
}

-- Compy Cube
puzzles:add{
  id = 'compy_cube',
  name = 'Compy Cube',
  version = '1.0.0',
  ndim = 3,
  colors = 'cube',
  build = function(self)
    local sym = cd'bc3'
    local shape = lib.symmetries.cubic.cube()
    self:carve(shape:iter_poles())

    -- Define axes and slices
    self.axes:add(lib.symmetries.octahedral.octahedron():iter_poles(), {INF, 0.82})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1.1})
    end

    -- Mark piece types
    utils.unpack_named(_ENV, self.axes)
    self:mark_piece(R(1) & U(1), 'edge', "Edge")
    self:mark_piece(U(1) & ~R(1) & ~L(1) & ~BD(1), 'corner', "Corner")
    self:mark_piece(sym:orbit(~U'*'):intersection(), 'core', "Core")
    self:unify_piece_types(sym.chiral)
  end,

  tags = {
    builtin = '2.0.0',
    external = { '!gelatinbrain', '!hof', '!mc4d', museum = 5020, '!wca' }, -- surprisingly not in gelatinbrain

    author = { "Milo Jacquet", "Andrew Farkas" },
    inventor = "Robert Webb",

    'type/puzzle',
    'shape/3d/platonic/cube',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire',
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/octahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { 'super', '!real', '!laminated', '!complex' },
    cuts = { 'depth/deep/to_adjacent', '!stored', '!wedge' },
    turns_by = { 'peak', 'vertex' },
    '!experimental',
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },
}
