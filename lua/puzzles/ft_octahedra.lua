local utils = lib.utils

puzzle_generators:add{
  id = 'ft_octahedron',
  version = '0.1.0',

  name = "N-Layer Face-Turning Octahedron",

  tags = {
    builtin = '1.0.0',
    --external = { '!gelatinbrain', '!hof', '!mc4d', '!museum', '!wca' },

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
    '!experimental',
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },

  params = {
    { name = "Layers", type = 'int', default = 1, min = 1, max = 13 },
  },

  examples = {
    { params = {1}, name = "Octahedron" },
    { params = {2}, name = "Skewb Diamond" },
    { params = {3}, name = "Face-Turning Octahedron" },
    { params = {4}, name = "Master Face-Turning Octahedron" },
  },

  gen = function(params)
    local size = params[1]

    return {
      name = size .. "-Layer Face-Turning Octahedron",

      colors = 'octahedron',

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

      ndim = 3,
      build = function(self)
        local sym = cd'bc3'
        local shape = lib.symmetries.octahedral.octahedron()
        self:carve(shape:iter_poles())

        -- Define axes and slices
        self.axes:add(shape:iter_poles(), utils.layers.inclusive(1, -1, size))

        -- Define twists
        for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
          self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
        end

        local center_layer = ceil(size/2)
        local precenter_layer = floor(size/2)
        local R = self.axes.R
        local L = self.axes.L
        local U = self.axes.U
        local F = self.axes.F
        local BD = self.axes.BD

        if size == 1 then
          self:mark_piece{
            region = REGION_ALL,
            name = 'core',
            display = "Core",
          }
        else
          -- Centers
          if size == 2 then
            self:mark_piece{
              region = U(1) & F(2) & R(1) & L(1),
              name = 'center',
              display = "Center",
            }
          elseif size == 3 then
            self:mark_piece{
              region = U(1) & F(2) & R(1) & L(1),
              name = 'triangle',
              display = "Triangle",
            }
          else
            self:add_piece_type{ name = 'center', display = "Center" }
            self:add_piece_type{ name = 'center/outer', display = "Outer triangle" }
            if size >= 3 then
              self:mark_piece{
                region = U(1) & F(2) & R(1) & L(1),
                name = 'center/outer/x',
                display = "Outer X-triangle",
              }
            end
            if size >= 4 and size % 2 == 0 then
              self:mark_piece{
                region = U(1) & R(1) & L(center_layer) & F(center_layer+1),
                name = 'center/outer/t',
                display = "Outer T-triangle",
              }
            end
            for i = 2, center_layer-1 do
              local name, display = string.fmt2('center/outer/oblique_%d', "Outer oblique (%d)", i-1)
              self:add_piece_type{ name = name, display = display }
              self:mark_piece{
                region = U(1) & R(1) & L(i) & F(i+1),
                name = name .. '/left',
                display = display .. ' (left)',
              }
              self:mark_piece{
                region = U(1) & R(1) & L(i+1) & F(i),
                name = name .. '/right',
                display = display .. ' (right)',
              }
            end

            if size >= 5 then
              self:add_piece_type{ name = 'center/thin', display = "Thin triangle" }
            end
            self:add_piece_type{ name = 'center/thick', display = "Thick triangle" }
            for i = 2, size-1 do
              for j = 2, size-1 do
                local k = size + 1 - i - j
                if i <= j and j <= k then
                  local name, display = string.fmt2('center/thin/_%d_%d_%d', "Thin triangle (%d, %d, %d)", i-1, j-1, k-1)
                  if i == j and j == k then
                    self:mark_piece{
                      region = U(1) & R(i) & L(j) & BD(k),
                      name = 'center/thin/middle',
                      display = 'Middle center',
                    }
                  elseif i < j and j < k then
                    self:add_piece_type{ name = name, display = display }
                    self:mark_piece{
                      region = U(1) & R(i) & L(j) & BD(k),
                      name = name .. '/left',
                      display = display .. ' (left)',
                    }
                    self:mark_piece{
                      region = U(1) & R(i) & L(k) & BD(j),
                      name = name .. '/right',
                      display = display .. ' (right)',
                    }
                  else
                    self:mark_piece{
                      region = U(1) & R(i) & L(j) & BD(k),
                      name = name,
                      display = display,
                    }
                  end
                end

                local k = size + 2 - i - j
                if i <= j and j <= k then
                  local name, display = string.fmt2('center/thick/_%d_%d_%d', "Thick triangle (%d, %d, %d)", i-1, j-1, k-1)
                  if i == j and j == k then
                    self:mark_piece{
                      region = U(1) & R(i) & L(j) & BD(k),
                      name = 'center/thick/middle',
                      display = 'Middle center',
                    }
                  elseif i < j and j < k then
                    self:add_piece_type{ name = name, display = display }
                    self:mark_piece{
                      region = U(1) & R(i) & L(j) & BD(k),
                      name = name .. '/left',
                      display = display .. ' (left)',
                    }
                    self:mark_piece{
                      region = U(1) & R(i) & L(k) & BD(j),
                      name = name .. '/right',
                      display = display .. ' (right)',
                    }
                  else
                    self:mark_piece{
                      region = U(1) & R(i) & L(j) & BD(k),
                      name = name,
                      display = display,
                    }
                  end
                end
              end
            end
          end

          -- Edges
          self:add_piece_type{ name = 'edge', display = "Edge" }
          for i = 2, precenter_layer do
            local name, display = string.fmt2('edge/wing_%d', "Wing (%d)", i-1)
            self:mark_piece{
              region = U(1) & R(1) & F(i) & L(i),
              name = name,
              display = display,
            }
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
            self:mark_piece{
              region = U(1) & R(1) & F(center_layer) & L(center_layer),
              name = 'edge' .. middle_suffix,
              display = edge_display,
            }
          end

          self:mark_piece{
            region = U(1) & F(1) & R(1) & L(1),
            name = 'corner',
            display = "Corner",
          }

          self:unify_piece_types(sym.chiral)
        end

      end,
    }
  end
}
