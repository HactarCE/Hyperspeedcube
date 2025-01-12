local utils = lib.utils

local function shallow_ft_octahedron_cut_depths(layers)
  assert(layers >= 1)

  -- These are not realistic for large layer counts but it is usable.
  local center = 1/2
  local half_range = 1/6
  return utils.concatseq({1}, utils.layers.exclusive_centered(center, half_range, layers))
end

-- N-Layer Face-Turning Octahedron generator
puzzle_generators:add{
  id = 'ft_octahedron',
  version = '0.1.0',
  name = "N-Layer Face-Turning Octahedron",
  params = {
    { name = "Layers", type = 'int', default = 1, min = 0, max = 13 },
  },
  gen = function(params)
    local size = params[1]
    if size == 0 then return 'octahedron' end
    return {
      name = size .. "-Layer Face-Turning Octahedron",
      colors = 'octahedron',
      ndim = 3,
      build = function(self)
        local sym = cd'bc3'
        local shape = lib.symmetries.bc3.octahedron()
        self:carve(shape:iter_poles())

        -- Define axes and slices
        self.axes:add(shape:iter_poles(), utils.layers.inclusive(1, -1, size))

        -- Define twists
        for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
          self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
        end

        local center_layer = ceil(size/2)
        local precenter_layer = floor(size/2)

        lib.utils.unpack_named(_ENV, self.axes)

        if size == 1 then
          lib.piece_types.mark_everything_core(self)
        else
          -- Centers
          if size == 2 then
            local region = U(1) & F(2) & R(1) & L(1)
            self:mark_piece(region, 'center', "Center")
          elseif size == 3 then
            local region = U(1) & F(2) & R(1) & L(1)
            self:mark_piece(region, 'triangle', "Triangle")
          else
            self:add_piece_type('center', "Center")
            self:add_piece_type('center/outer', "Outer triangle")
            if size >= 3 then
              local region = U(1) & F(2) & R(1) & L(1)
              self:mark_piece(region, 'center/outer/x', "Outer X-triangle")
            end
            if size >= 4 and size % 2 == 0 then
              local region = U(1) & R(1) & L(center_layer) & F(center_layer+1)
              self:mark_piece(region, 'center/outer/t', "Outer T-triangle")
            end
            for i = 2, center_layer-1 do
              local name, display = string.fmt2('center/outer/oblique_%d', "Outer oblique (%d)", i-1)
              self:add_piece_type(name, display)
              local left_region = U(1) & R(1) & L(i) & F(i+1)
              local right_region = U(1) & R(1) & L(i+1) & F(i)
              self:mark_piece(left_region, name .. '/left', display .. ' (left)')
              self:mark_piece(right_region, name .. '/right', display .. ' (right)')
            end

            if size >= 5 then
              self:add_piece_type('center/thin', "Thin triangle")
            end
            self:add_piece_type('center/thick', "Thick triangle")
            for i = 2, size-1 do
              for j = 2, size-1 do
                local k = size + 1 - i - j
                if i <= j and j <= k then
                  local name, display = string.fmt2('center/thin/%d_%d_%d', "Thin triangle (%d, %d, %d)", i-1, j-1, k-1)
                  if i == j and j == k then
                    self:mark_piece(
                      U(1) & R(i) & L(j) & BD(k),
                      'center/thin/middle',
                      'Middle center'
                    )
                  elseif i < j and j < k then
                    self:add_piece_type(  name, display )
                    self:mark_piece(
                      U(1) & R(i) & L(j) & BD(k),
                      name .. '/left',
                      display .. ' (left)'
                    )
                    self:mark_piece(
                      U(1) & R(i) & L(k) & BD(j),
                      name .. '/right',
                      display .. ' (right)'
                    )
                  else
                    self:mark_piece(
                      U(1) & R(i) & L(j) & BD(k),
                      name,
                      display
                    )
                  end
                end

                local k = size + 2 - i - j
                if i <= j and j <= k then
                  local name, display = string.fmt2('center/thick/%d_%d_%d', "Thick triangle (%d, %d, %d)", i-1, j-1, k-1)
                  if i == j and j == k then
                    self:mark_piece(
                      U(1) & R(i) & L(j) & BD(k),
                      'center/thick/middle',
                      'Middle center'
                    )
                  elseif i < j and j < k then
                    self:add_piece_type( name, display )
                    self:mark_piece(
                      U(1) & R(i) & L(j) & BD(k),
                      name .. '/left',
                      display .. ' (left)'
                    )
                    self:mark_piece(
                      U(1) & R(i) & L(k) & BD(j),
                      name .. '/right',
                      display .. ' (right)'
                    )
                  else
                    self:mark_piece(
                      U(1) & R(i) & L(j) & BD(k),
                      name,
                      display
                    )
                  end
                end
              end
            end
          end

          -- Edges
          if size > 2 then
            self:add_piece_type( 'edge', "Edge" )
          end
          for i = 2, precenter_layer do
            local name, display = string.fmt2('edge/wing_%d', "Wing (%d)", i-1)
            self:mark_piece(
              U(1) & R(1) & F(i) & L(i),
              name,
              display
            )
          end

          -- Middle centers and edges
          local middle_suffix = ''
          local center_display, edge_display -- nil is ok here
          if size > 3 then
            middle_suffix = '/middle'
            center_display = "Middle center"
            edge_display = "Middle edge"
          end

          if size % 2 == 1 then
            self:mark_piece(
              U(1) & R(1) & F(center_layer) & L(center_layer),
              'edge' .. middle_suffix,
              edge_display
            )
          end

          self:mark_piece(
            U(1) & F(1) & R(1) & L(1),
            'corner',
            "Corner"
          )

          self:unify_piece_types(sym.chiral)
        end

      end,

      tags = {
        algebraic = {
          abelian = size == 1,
          trivial = size == 1,
        },
        canonical = size == 2 or size == 3,
        completeness = {
          complex = size == 1,
          laminated = size == 1,
          real = size <= 2,
          super = size == 1,
        },
        ['cuts/depth/shallow'] = size == 3,
        ['cuts/depth/deep/to_adjacent'] = size % 3 == 0,
        ['cuts/depth/deep/past_adjacent'] = size >= 4 or size == 2,
        ['cuts/depth/half'] = size % 2 == 0,
        meme = size == 1,
      },
    }
  end,

  examples = {
    { params = {1}, name = "Octahedron" },
    {
      params = {2},
      name = "Skewb Diamond",
      tags = {
        inventor = "Tony Fisher",
        external = { gelatinbrain = '4.1.1', museum = 393 },
      },
    },
    {
      params = {3},
      name = "Face-Turning Octahedron",
      tags = {
        inventor = "David Pitcher",
        external = { gelatinbrain = '4.1.2', museum = 1663 },
      },
    },
    {
      params = {4},
      name = "Master Face-Turning Octahedron",
      tags = {
        inventor = "Timur Evbatyrov",
        external = { gelatinbrain = '4.1.5', museum = 1729 },
      },
    },
    {
      params = {5},
      tags = {
        inventor = "Seth Holiday",
        external = { gelatinbrain = '4.1.8', museum = 9865 },
      },
    },
    {
      params = {6},
      tags = {
        inventor = "Seth Holiday",
        external = { museum = 9807 },
      },
    },
  },

  tags = {
    builtin = '2.0.0',
    external = { '!gelatinbrain', '!hof', '!mc4d', '!museum', '!wca' },

    author = { "Andrew Farkas", "Milo Jacquet" },
    '!inventor',

    'shape/3d/platonic/octahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire',
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/octahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!stored', '!wedge' },
    turns_by = { 'face', 'facet' },
    'experimental', -- needs piece type bikeshedding + testing
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },
}

-- N-Layer Face-Turning Octahedron (Shallow) generator
puzzle_generators:add{
  id = 'ft_octahedron_shallow',
  version = '0.1.0',
  name = "N-Layer Face-Turning Octahedron (Shallow)",
  params = {
    { name = "Layers", type = 'int', default = 1, min = 0, max = 13 },
  },
  gen = function(params)
    local size = params[1]
    if size == 0 then return 'octahedron' end
    return {
      name = size .. "-Layer Face-Turning Octahedron (Shallow)",
      colors = 'octahedron',
      ndim = 3,
      build = function(self)
        local sym = cd'bc3'
        local shape = lib.symmetries.bc3.octahedron()
        self:carve(shape:iter_poles())

        -- Define axes and slices
        self.axes:add(shape:iter_poles(), shallow_ft_octahedron_cut_depths(size))

        -- Define twists
        for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
          self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
        end

        lib.utils.unpack_named(_ENV, self.axes)

        self:add_piece_type('center', "Center")
        self:add_piece_type('edge', "Edge")
        self:add_piece_type('petal', "Petal")
        self:add_piece_type('corner', "Corner")

        -- Center
        local F_adj = R('*') | L('*') | D('*')
        local FD_adj = R('*') | L('*') | BL('*') | BR('*')
        local center_region = F(1) & ~F_adj
        local edge_region = F(1) & D(1) & ~FD_adj
        local petal_region = F(1) & U(1) & R(1) & ~L('*')
        local corner_region = F(1) & U(1) & R(1) & L(1)
        self:mark_piece(center_region, 'center/0_0', 'Middle center')
        self:mark_piece(edge_region, 'edge/0_0', 'Middle Edge')
        self:mark_piece(petal_region, 'petal/0', 'Petal')
        self:mark_piece(corner_region, 'corner')

        -- Center (to petal)
        for i = 1, size-1 do
          local region = F(1) & D(size-i+1) & ~FD_adj
          self:mark_piece(region, string.fmt2('center/ce_%d', "CE (%d)", i))
          for j=1, size-1 do
            local region = F(1) & D(size-i+1) & R(size-j+1)
            self:mark_piece(region, string.fmt2('center/cep_%d_%d', "CEP (%d, %d)", i, j))
          end
        end

        -- Edge (to vertex)
        for i = 0, size-1 do
          for j = 0,size-1 do
            if i>0 or j>0 then
              local x = i>0 and L(size-i+1) or ~L('*')
              local y = j>0 and F(size-j+1) or ~F('*')

              local region = U(1) & R(1) & x & y
              self:mark_piece(region, string.fmt2('edge/epv_%d_%d', "EPV (%d,%d)", i, j))
            end
          end
        end

        -- Petal (to vertex)
        for i = 1, size-1 do
          local region = U(1) & R(1) & L(1) & F(size-i+1)
          self:mark_piece(region, string.fmt2('petal/pv_%d', "PV (%d)", i))
        end

        self:unify_piece_types(shape.sym.chiral)
      end,

      tags = {
        algebraic = {
          abelian = size == 1,
          trivial = size == 1,
        },
        canonical = size == 2 or size == 3,
        completeness = {
          complex = size == 1,
          laminated = size == 1,
          real = size <= 2,
          super = size == 1,
        },
        ['cuts/depth/shallow'] = size == 3,
        ['cuts/depth/deep/to_adjacent'] = size % 3 == 0,
        ['cuts/depth/deep/past_adjacent'] = size >= 4 or size == 2,
        ['cuts/depth/half'] = size % 2 == 0,
        meme = size == 1,
      },
    }
  end,

  examples = {
    {
      params = {1},
      name = "Dino Octa",
      tags = {
        inventor = "Katsuhiko Okamoto",
        external = { gelatinbrain = '4.1.3', museum = 2673 },
      },
    },
  },

  tags = {
    builtin = '2.0.0',
    external = { '!gelatinbrain', '!hof', '!mc4d', '!museum', '!wca' },

    author = { "Andrew Farkas", "Milo Jacquet", "Luna Harran" },
    '!inventor',

    'shape/3d/platonic/octahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire',
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/octahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!stored', '!wedge' },
    turns_by = { 'face', 'facet' },
    'experimental', -- needs piece type bikeshedding + testing
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },
}
