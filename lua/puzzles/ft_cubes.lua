local utils = require('utils')
local symmetries = require('symmetries')

local REALISITIC_PROPORTIONS = true
local CORNER_STALK_SIZE = 0.1

local function ft_cube_cut_depths(ndim, size)
  if size == 2 then return {0} end

  local outermost_cut
  local aesthetic_limit = 1 - 2/size
  local mechanical_limit = 0
  if REALISITIC_PROPORTIONS then
    mechanical_limit = 1 / sqrt(ndim-1)
  end
  outermost_cut = min(aesthetic_limit, mechanical_limit - CORNER_STALK_SIZE)
  return utils.layers_inclusive(outermost_cut, -outermost_cut, size-1)
end

-- n^3
puzzle_generators:add{
  id = 'ft_cube',
  version = '0.1.0',

  name = "NxNxN Face-Turning Cube",
  meta = {
    authors = { "Andrew Farkas", "Milo Jacquet" },
  },

  params = {
    { name = "Layers", type = 'int', default = 3, min = 1, max = 17 },
  },

  examples = {
    { params = {1} },
    {
      params = {2},
      meta = {
        aliases = { "Pocket Cube" },
        external = { wca = '222' },
        inventor = "Ernő Rubik",
      }
    },
    {
      params = {3},
      meta = {
        aliases = { "Rubik's Cube" },
        external = { wca = '333' },
        inventor = "Ernő Rubik",
      },
    },
    {
      params = {4},
      meta = { aliases = { "Rubik's Revenge" }, external = { wca = '444' } },
    },
    {
      params = {5},
      meta = { aliases = { "Professor's Cube" }, external = { wca = '555' } },
    },
    { params = {6}, meta = { external = { wca = '666' } } },
    { params = {7}, meta = { external = { wca = '777' } } },
  },

  gen = function(params)
    local size = params[1]

    return {
      name = size .. "x" .. size .. "x" .. size,

      colors = 'cube',

      ndim = 3,
      build = function(self)
        local sym = cd'bc3'
        local shape = symmetries.cubic.cube()
        self:carve(shape:iter_poles())

        -- Define axes and slices
        self.axes:add(shape:iter_poles(), ft_cube_cut_depths(3, size))

        -- Define twists
        for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox.unit], sym:thru(2, 1)) do
          self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
        end

        local center_layer = ceil(size/2)
        local precenter_layer = floor(size/2)
        local R = self.axes.R
        local L = self.axes.L
        local U = self.axes.U
        local F = self.axes.F

        -- Mark piece types
        if size == 1 then
          self:mark_piece{
            region = REGION_ALL,
            name = 'core',
            display = "Core",
          }
        else
          local U_adj = symmetry{self.twists.U}:orbit(R(1, precenter_layer)):union()

          -- Centers
          self:add_piece_type{ name = 'center', display = "Center" }
          for i = 2, center_layer do
            for j = 2, precenter_layer do
              local name, display
              if i == center_layer and size % 2 == 1 then
                name, display = string.fmt2('center/t_%d', "T-center (%d)", j-1)
              elseif i == j then
                name, display = string.fmt2('center/x_%d', "X-center (%d)", i-1)
              else
                if i < j then
                  name, display = string.fmt2('center/oblique_%d_%d', "Oblique (%d, %d)", i-1, j-1)
                  self:add_piece_type{ name = name, display = display }
                  name = name .. "/left"
                  display = display .. " (left)"
                else
                  name, display = string.fmt2('center/oblique_%d_%d', "Oblique (%d, %d)", j-1, i-1)
                  name = name .. "/right"
                  display = display .. " (right)"
                end
              end
              self:mark_piece{
                region = U(1) & R(i) & F(j),
                name = name,
                display = display,
              }
            end
          end

          -- Edges
          self:add_piece_type{ name = 'edge', display = "Edge" }
          for i = 2, precenter_layer do
            local name, display = string.fmt2('edge/wing_%d', "Wing (%d)", i-1)
            self:mark_piece{
              region = U(1) & R(1) & F(i),
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
              region = U(1) & ~U_adj,
              name = 'center' .. middle_suffix,
              display = center_display,
            }
            self:mark_piece{
              region = U(1) & F(1) & ~R(1, precenter_layer) & ~L(1, precenter_layer),
              name = 'edge' .. middle_suffix,
              display = edge_display,
            }
          end

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

-- n^4
puzzle_generators:add{
  id = 'ft_hypercube',
  version = '0.1.0',

  name = "NxNxNxN Face-Turning Hypercube",
  meta = {
    authors = { "Andrew Farkas", "Milo Jacquet" },
  },

  params = {
    { name = "Layers", type = 'int', default = 3, min = 1, max = 9 },
  },

  gen = function(params)
    local size = params[1]

    local gizmo_size = 1.2
    local alpha = 0.8

    return {
      name = size .. "x" .. size .. "x" .. size .. "x" .. size,

      colors = 'hypercube',

      ndim = 4,
      build = function(self)
        local sym = cd'bc4'
        local shape = symmetries.hypercubic.hypercube()
        self:carve(shape:iter_poles())

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
            name = axis2.name,
            gizmo_pole_distance = gizmo_size,
          })
        end

        local edge = a2.vector + a3.vector -- ridge orthogonal to `a1`
        local init_transform = sym:thru(3, 1) -- rot{fix = a1.vector ^ edge, angle = PI}
        for t, axis1, _edge, twist_transform in sym.chiral:orbit(a1, edge, init_transform) do
          self.twists:add(axis1, twist_transform, {
            name = t:transform(a2).name .. t:transform(a3).name,
            gizmo_pole_distance = (1+alpha)/sqrt(2) * gizmo_size,
          })
        end

        local vertex = edge + a4.vector -- edge orthogonal to `a1`
        local init_transform = sym:thru(3, 2)
        for t, axis1, _vertex, twist_transform in sym.chiral:orbit(a1, vertex, init_transform) do
          self.twists:add(axis1, twist_transform, {
            name = t:transform(a2).name .. t:transform(a3).name .. t:transform(a4).name,
            gizmo_pole_distance = (1+2*alpha)/sqrt(3) * gizmo_size,
          })
        end

        local R = self.axes.R
        local U = self.axes.U
        local F = self.axes.F
        local I = self.axes.I

        if size == 1 then
          self:mark_piece{
            region = REGION_ALL,
            name = 'core',
            display = "Core",
          }
        else
          -- TODO: more piece types

          if size >= 3 then
            local mid = '{2-' .. size-1 .. '}'
            self:mark_piece{
              region = U(1) & R(mid) & F(mid) & I(mid),
              name = 'center',
              display = 'Center',
            }
            self:mark_piece{
              region = U(1) & R(1) & F(mid) & I(mid),
              name = 'ridge',
              display = 'Ridge',
            }
            self:mark_piece{
              region = U(1) & R(1) & F(1) & I(mid),
              name = 'edge',
              display = 'Edge',
            }
          end

          self:mark_piece{
            region = U(1) & F(1) & R(1) & I(1),
            name = 'corner',
            display = "Corner",
          }

          self:unify_piece_types(sym.chiral)
        end
      end,
    }
  end,
}
