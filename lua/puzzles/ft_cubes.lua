local utils = lib.utils

local REALISITIC_PROPORTIONS = true
local CORNER_STALK_SIZE = 0.1

local function ft_cube_cut_depths(ndim, layers)
  if layers < 2 then return end
  if layers == 2 then return {1, 0, -1} end

  local outermost_cut
  local aesthetic_limit = 1 - 2/layers
  local mechanical_limit = 0
  if REALISITIC_PROPORTIONS then
    mechanical_limit = 1 / sqrt(ndim-1)
  end
  outermost_cut = min(aesthetic_limit, mechanical_limit - CORNER_STALK_SIZE)
  return utils.concatseq({1}, utils.layers.inclusive(outermost_cut, -outermost_cut, layers-2), {-1})
end

GIZMO_EDGE_FACTOR = 0.8

-- NxNxN Face-Turning Cube generator
puzzle_generators:add{
  id = 'ft_cube',
  version = '1.0.0',
  name = "NxNxN Face-Turning Cube",
  colors = 'cube',
  params = {
    { name = "Layers", type = 'int', default = 3, min = 1, max = 17 },
  },
  gen = function(params)
    local size = params[1]
    return {
      name = size .. "x" .. size .. "x" .. size,
      aliases = { size .. "^" .. 3 },
      ndim = 3,
      build = function(self)
        local shape = lib.symmetries.cubic.cube()
        local cut_depths = ft_cube_cut_depths(3, size)
        local colors, axes = utils.cut_ft_shape(self, shape, cut_depths)

        if size == 1 then
          lib.piece_types.mark_everything_core(self)
          return
        end

        -- Define twists
        for t, ax, rot in shape.sym.chiral:orbit(axes[1], shape.sym:thru(2, 1)) do
          self.twists:add(ax, rot, { gizmo_pole_distance = 1 })
        end

        -- Mark piece types
        lib.piece_types.triacron_subsets.mark_multilayer_UFRL(self, size)
        self:unify_piece_types(shape.sym.chiral) -- chiral because left vs. right obliques
      end,

      tags = {
        ['type/shape'] = size == 1,
        ['type/puzzle'] = size ~= 1,
        algebraic = {
          abelian = size == 1,
          trivial = size == 1,
        },
        canonical = size == 3,
        completeness = {
          complex = size == 1,
          laminated = size <= 2,
          real = size <= 3,
          super = size <= 2,
        },
        ['cuts/depth/deep/to_adjacent'] = size % 2 == 0,
        ['cuts/depth/half'] = size % 2 == 0,
        meme = size == 1,
      },
    }
  end,

  examples = {
    { params = {1}, tags = { 'algebraic/trivial', 'meme' } },
    {
      params = {2},
      aliases = { "Pocket Cube" },
      tags = {
        external = { gelatinbrain = '3.1.1', museum = 20, wca = '222' },
        inventor = "Ernő Rubik",
      }
    },
    {
      params = {3},
      aliases = { "Rubik's Cube" },
      tags = {
        external = { gelatinbrain = '3.1.2', museum = 7629, wca = '333' },
        inventor = "Ernő Rubik",
      },
    },
    {
      params = {4},
      aliases = { "Rubik's Revenge" },
      tags = {
        external = { gelatinbrain = '3.1.3', museum = 265, wca = '444' },
        inventor = "Peter Sebesteny",
      },
    },
    {
      params = {5},
      aliases = { "Professor's Cube" },
      tags = {
        external = { gelatinbrain = '3.1.4', museum = 6106, wca = '555' },
        inventor = "Jürgen Hoffmann",
      },
    },
    {
      params = {6},
      tags = {
        external = { museum = 3931, wca = '666' },
        inventor = "Daniel Tseng",
      },
    },
    {
      params = {7},
      tags = {
        external = { museum = 1486, wca = '777' },
        inventor = "Panagiotis Verdes",
      },
    },
  },

  tags = {
    builtin = '2.0.0',
    external = { '!gelatinbrain', '!hof', '!mc4d', '!museum', '!wca' },

    author = { "Andrew Farkas", "Milo Jacquet" },
    '!inventor',

    'shape/3d/platonic/cube',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire',
      '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/cubic', '!hybrid', '!multicore' },
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

-- NxNxNxN Face-Turning Hypercube generator
puzzle_generators:add{
  id = 'ft_hypercube',
  version = '1.0.0',
  name = "NxNxNxN Face-Turning Hypercube",
  colors = 'hypercube',
  params = {
    { name = "Layers", type = 'int', default = 3, min = 1, max = 13 },
  },
  gen = function(params)
    local size = params[1]

    return {
      name = size .. "x" .. size .. "x" .. size .. "x" .. size,
      aliases = { size .. "^" .. 4 },
      ndim = 4,
      build = function(self)
        local sym = cd'bc4'
        local shape = lib.symmetries.hypercubic.hypercube()
        self:carve(shape:iter_poles())

        if size == 1 then
          lib.piece_types.mark_everything_core(self)
          return
        end

        -- Define axes and slices
        self.axes:add(shape:iter_poles(), ft_cube_cut_depths(4, size))

        -- Define twists
        local a1 = self.axes[sym.ooox.unit]
        local a2 = sym:thru(4):transform(a1)
        local a3 = sym:thru(3):transform(a2)
        local a4 = sym:thru(2):transform(a3)
        local t = sym:thru(2, 1)
        for _, axis1, axis2, twist_transform in sym.chiral:orbit(a1, a2, t) do
          self.twists:add(axis1, twist_transform, {
            name = axis2,
            gizmo_pole_distance = 1,
          })
        end

        local ridge = a2.vector + a3.vector -- ridge orthogonal to `a1`
        local init_transform = sym:thru(3, 1) -- rot{fix = a1.vector ^ ridge, angle = PI}
        for t, axis1, _ridge, twist_transform in sym.chiral:orbit(a1, ridge, init_transform) do
          self.twists:add(axis1, twist_transform, {
            name = names.set(t:transform(a2), t:transform(a3)),
            gizmo_pole_distance = (1 + GIZMO_EDGE_FACTOR) / sqrt(2),
          })
        end

        local edge = ridge + a4.vector -- edge orthogonal to `a1`
        local init_transform = sym:thru(3, 2)
        for t, axis1, _edge, twist_transform in sym.chiral:orbit(a1, edge, init_transform) do
          self.twists:add(axis1, twist_transform, {
            name = names.set(t:transform(a2), t:transform(a3), t:transform(a4)),
            gizmo_pole_distance = (1 + 2 * GIZMO_EDGE_FACTOR) / sqrt(3),
          })
        end

        -- Mark piece types
        lib.piece_types.tetrahedracron_subsets.mark_multilayer_UFRLIO(self, size)
        self:unify_piece_types(sym.chiral) -- chiral because left vs. right obliques
      end,

      tags = {
        ['type/shape'] = size == 1,
        ['type/puzzle'] = size ~= 1,
        algebraic = {
          abelian = size == 1,
          trivial = size == 1,
        },
        canonical = size == 3,
        completeness = {
          complex = size == 1,
          laminated = size <= 2,
          real = size <= 3,
          super = size <= 2,
        },
        ['cuts/depth/deep/to_adjacent'] = size % 2 == 0,
        ['cuts/depth/half'] = size % 2 == 0,
        meme = size == 1,
      },
    }
  end,

  examples = {
    { params = {1} },
    { params = {2}, tags = { external = { gelatinbrain = '8.1.1' } } },
    { params = {3} },
    { params = {4} },
    { params = {5} },
    { params = {6} },
    { params = {7} },
  },

  tags = {
    builtin = '2.0.0',
    external = { '!gelatinbrain', '!hof', '!mc4d', '!museum', '!wca' },

    author = { "Andrew Farkas", "Milo Jacquet" },
    '!inventor',

    'shape/4d/platonic/hypercube',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire',
      '!fused', 'orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '4d/elementary/hypercubic', '!hybrid', '!multicore' },
    colors = { '!multi_facet_per', '!multi_per_facet' },
    cuts = { depth = { 'shallow' }, '!stored', '!wedge' },
    turns_by = { 'cell', 'facet' },
    '!experimental',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },
}
