local utils = require('utils')
local symmetries = require('symmetries')

local REALISITIC_PROPORTIONS = true
local CORNER_STALK_SIZE = 0.1

local function ft_cube_cut_depths(ndim, size)
  local outermost_cut
  local aesthetic_limit = 1 - 2/size
  local mechanical_limit = 0
  if REALISITIC_PROPORTIONS then
    mechanical_limit = 1 / sqrt(ndim-1)
  end
  outermost_cut = min(aesthetic_limit, mechanical_limit - CORNER_STALK_SIZE)
  return utils.layers_inclusive(outermost_cut, -outermost_cut, size-1)
end

function define_ft_cube_3d(size)
  local id = size .. 'x' .. size .. 'x' .. size
  puzzles:add(id, {
    ndim = 3,
    name = string.format("FT Cube %d (%d^3)", size, size),
    colors = 'cube',
    -- piece_types = {
    --   { id = 'centers', name = "Centers" },
    --   {
    --     id = 'moving', name = "Moving pieces",
    --     { id = 'edges', name = "Edges" },
    --     { id = 'corners', name = "Corners" },
    --   },
    -- },
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
      local U_adj = symmetry{self.twists.U}:orbit(R(1, precenter_layer)):union()

      -- Centers
      for i = 2, center_layer do
        for j = 2, precenter_layer do
          local name
          if i == center_layer and size % 2 == 1 then
            name = string.format('t-centers (%d)', j-1)
          elseif i == j then
            name = string.format('x-centers (%d)', i-1)
          else
            if i < j then
              name = string.format('obliques (%d, %d) (left)', i-1, j-1)
            else
              name = string.format('obliques (%d, %d) (right)', j-1, i-1)
            end
          end
          self:mark_pieces(name, U(1) & R(i) & F(j))
        end
      end

      if size == 1 then
        self:mark_pieces('core', U(1))
        return
      end

      for i = 2, precenter_layer do
        self:mark_pieces(string.format('wings (%d)', i-1), U(1) & R(1) & F(i))
      end

      if size % 2 == 1 then
        self:mark_pieces('centers', U(1) & ~U_adj)
        self:mark_pieces('edges', U(1) & F(1) & ~R(1, precenter_layer) & ~L(1, precenter_layer))
      end

      self:mark_pieces('corners', U(1) & F(1) & R(1))
      self:unify_piece_types(sym.chiral)
    end,
  })
end

for size = 1, 21 do
  define_ft_cube_3d(size)
end

function define_ft_cube_4d(size)
  local gizmo_size = 1.2
  local alpha = 0.8

  local id = size .. 'x' .. size .. 'x' .. size .. 'x' .. size
  puzzles:add(id, {
    ndim = 4,
    name = string.format("Hypercube %d (%d^4)", size, size),
    colors = 'hypercube',
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
          name = axis1.name .. axis2.name,
          gizmo_pole_distance = gizmo_size,
        })
      end

      local edge = a2.vector + a3.vector -- ridge orthogonal to `a1`
      local init_transform = sym:thru(3, 1) -- rot{fix = a1.vector ^ edge, angle = PI}
      for t, axis1, _edge, twist_transform in sym.chiral:orbit(a1, edge, init_transform) do
        self.twists:add(axis1, twist_transform, {
          name = axis1.name .. t:transform(a2).name .. t:transform(a3).name,
          gizmo_pole_distance = (1+alpha)/sqrt(2) * gizmo_size,
        })
      end

      local vertex = edge + a4.vector -- edge orthogonal to `a1`
      local init_transform = sym:thru(3, 2)
      for t, axis1, _vertex, twist_transform in sym.chiral:orbit(a1, vertex, init_transform) do
        self.twists:add(axis1, twist_transform, {
          name = axis1.name .. t:transform(a2).name .. t:transform(a3).name .. t:transform(a4).name,
          gizmo_pole_distance = (1+2*alpha)/sqrt(3) * gizmo_size,
        })
      end

      local R = self.axes.R
      local U = self.axes.U
      local F = self.axes.F
      local I = self.axes.I

      self:mark_pieces('centers', U(1) & R(2) & F(2) & I(2))
      self:mark_pieces('ridges', U(1) & R(1) & F(2) & I(2))
      self:mark_pieces('edges', U(1) & R(1) & F(1) & I(2))
      self:mark_pieces('corners', U(1) & F(1) & R(1) & I(1))
      self:unify_piece_types(sym.chiral)
    end,
  })
end

for size = 1, 9 do
  define_ft_cube_4d(size)
end

puzzles:add('opposite_colors_same_cube', {
  ndim = 3,
  name = "Opposite colors are the same",
  colors = 'half_cube',
  build = function(self)
    local sym = cd'bc3'
    local shape = symmetries.cubic.cube()
    self:carve(shape:iter_poles(), {
      stickers = {
        R = 'X', L = 'X',
        U = 'Y', D = 'Y',
        F = 'Z', B = 'Z',
      },
    })

    -- Define axes and slices
    self.axes:add(shape:iter_poles(), utils.layers_exclusive(1, -1, 2))

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox], sym:thru(2, 1)) do
      self.twists:add(axis, twist_transform)
    end
  end,
})
