local utils = require('utils')
local symmetries = require('symmetries')

local REALISITIC_PROPORTIONS = true
local CORNER_STALK_SIZE = 0.03


local function skewb_cut_depths(size)
  if size == 2 then return {0} end

  local outermost_cut
  local aesthetic_limit = (1 - 2/(size+0.6)) * (1 / sqrt(3))
  local mechanical_limit = 1 / sqrt(3)
  if REALISITIC_PROPORTIONS then
    -- this is the negative of the galois conjugate of the corresponding value for the megaminx
    mechanical_limit = sqrt(3) / 5
  end
  outermost_cut = min(aesthetic_limit, mechanical_limit - CORNER_STALK_SIZE)
  return utils.layers_inclusive(outermost_cut, -outermost_cut, size-1)
end

function define_skewb(size, name)
  local puzzle_name
  if name == '' then
    puzzle_name = string.format("Skewb %d", size)
  else
    puzzle_name = string.format("Skewb %d (%s)", size, name)
  end

  puzzles:add(string.format("skewb_%d", size), {
    ndim = 3,
    name = puzzle_name,
    colors = 'cube',
    meta = {
      author = 'Milo Jacquet',
    },
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
      self.axes:add(symmetries.octahedral.octahedron():iter_poles(), skewb_cut_depths(size))

      -- Define twists
      for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
        self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1.1})
      end

      local R = self.axes.R
      local L = self.axes.L
      local U = self.axes.U
      local F = self.axes.F
      local BD = self.axes.BD

      local center_layer = ceil(size/2)

      local middle_prefix = ''
      if size > 3 then
        middle_prefix = 'middle '
      end

      -- Centers
      for i = 2, center_layer do
        for j = i, size+1-i do
          local name
          local name2 = ''
          if i == j and j*2 - 1 == size then
            name = middle_prefix .. 'edges'
          elseif i == j then
            name = string.format('wings (%d)', i-1)
          elseif i + j == size+1 then
            name = string.format('t-centers (%d)', i-1)
          else
            name = string.format('obliques (%d, %d) (right)', i-1, j-1)
            name2 = string.format('obliques (%d, %d) (left)', i-1, j-1)
          end
          self:mark_piece(name, F(1) & L(i) & R(j))
          if name2 ~= '' then
            self:mark_piece(name2, F(1) & L(j) & R(i))
          end
        end
      end

      for i = 2, floor(size/2) do
        self:mark_piece(string.format('outer x-centers (%d)', i-1), BD(i) & L(1) & R(1))
        self:mark_piece(string.format('inner x-centers (%d)', i-1), BD(size+1-i) & L(1) & R(1))
      end

      if size % 2 == 1 then
        name = self:mark_piece(middle_prefix .. 'x-centers', U(center_layer) & L(1) & R(1))
      end

      self:mark_piece('centers', F(1) & R(1) & U(1) & L(1))
      self:mark_piece('corners', L(1) & R(1) & BD(1))
      self:unify_piece_types(sym.chiral)
    end,
  })
end

define_skewb(2, "Skewb")
define_skewb(3, "Master Skewb")
define_skewb(4, "Elite Skewb")
define_skewb(5, "Royal Skewb")
define_skewb(6, "")
define_skewb(7, "")
define_skewb(8, "")
define_skewb(9, "")


puzzles:add('dino_cube', {
  ndim = 3,
  name = 'Dino Cube',
  colors = 'cube',
  meta = {
    author = 'Milo Jacquet',
  },
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
    self.axes:add(symmetries.octahedral.octahedron():iter_poles(), {1/sqrt(3)})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1.1})
    end

    local R = self.axes.R
    local L = self.axes.L
    local U = self.axes.U
    local BD = self.axes.BD

    self:mark_piece('edges', R(1) & U(1))
    self:unify_piece_types(sym.chiral)
  end,
})

puzzles:add('compy_cube', {
  ndim = 3,
  name = 'Compy Cube',
  colors = 'cube',
  meta = {
    author = 'Milo Jacquet',
  },
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
    self.axes:add(symmetries.octahedral.octahedron():iter_poles(), {0.82})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1.1})
    end

    local R = self.axes.R
    local L = self.axes.L
    local U = self.axes.U
    local BD = self.axes.BD

    self:mark_piece('edges', R(1) & U(1))
    self:mark_piece('corners', U(1) & ~R(1) & ~L(1) & ~BD(1))
    self:mark_piece('core', sym:orbit(~U'*'):intersection())
    self:unify_piece_types(sym.chiral)
  end,
})
