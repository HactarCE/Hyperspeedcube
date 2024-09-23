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

  puzzles:add{
    id = string.format("skewb_%d", size),
    name = puzzle_name,
    version = '0.1.0',
    ndim = 3,
    colors = 'cube',
    meta = {
      author = 'Milo Jacquet',
    },
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

      -- Centers
      for i = 2, center_layer do
        for j = i, size+1-i do
          local name, display
          local name2, display2
          if i == j and j*2 - 1 == size then
            if size > 3 then
              name, display = 'edge/middle', "Middle edge"
            else
              name, display = 'edge', "Edge"
            end
          elseif i == j then
            name, display = string.fmt2("edge/wing_%d", "Wing (%d)", i-1)
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
            region = F(1) & L(j) & R(i),
            name = name,
            display = display,
          }
          if name2 ~= nil then
            self:mark_piece{
              region = F(1) & L(i) & R(j),
              name = name2,
              display = display2,
            }
          end
        end
      end

      for i = 2, floor(size/2) do
        local name, display = string.fmt2('center/x/outer_%d', "Outer X-center (%d)", i-1)
        self:mark_piece{
          region = BD(i) & L(1) & R(1),
          name = name,
          display = display,
        }

        local name, display = string.fmt2('center/x/inner_%d', "Inner X-center (%d)", i-1)
        self:mark_piece{
          region = BD(size+1-i) & L(1) & R(1),
          name = name,
          display = display,
        }
      end

      if size % 2 == 1 then
        local name, display
        if size > 3 then
          name, display = 'center/x/middle', "Middle X-center"
        else
          name, display = 'center/x', "X-center"
        end
        name = self:mark_piece{
          region = U(center_layer) & L(1) & R(1),
          name = name,
          display = display,
        }
      end

      local name, display
      if size > 3 then
        name, display = 'center/middle', "Middle center"
      else
        name, display = 'center', "Center"
      end
      self:mark_piece{
        region = F(1) & R(1) & U(1) & L(1),
        name = name,
        display = display,
      }

      self:mark_piece{
        region = L(1) & R(1) & BD(1),
        name = 'corner',
        display = "Corner",
      }

      self:unify_piece_types(sym.chiral)
    end,
  }
end

define_skewb(2, "Skewb")
define_skewb(3, "Master Skewb")
define_skewb(4, "Elite Skewb")
define_skewb(5, "Royal Skewb")
define_skewb(6, "")
define_skewb(7, "")
define_skewb(8, "")
define_skewb(9, "")


puzzles:add{
  id = 'dino_cube',
  name = 'Dino Cube',
  version = '0.1.0',
  ndim = 3,
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

    self:mark_piece{
      region =  R(1) & U(1),
      name = 'edge',
      display = "Edge",
    }

    self:unify_piece_types(sym.chiral)
  end,
}

puzzles:add{
  id = 'compy_cube',
  name = 'Compy Cube',
  version = '0.1.0',
  ndim = 3,
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

    self:mark_piece{
      region = R(1) & U(1),
      name = 'edge',
      display = 'Edge',
    }
    self:mark_piece{
      region = U(1) & ~R(1) & ~L(1) & ~BD(1),
      name = 'corner',
      display = 'Corner',
    }
    self:mark_piece{
      region = sym:orbit(~U'*'):intersection(), -- TODO: construct 'everything' region
      name = 'core',
      display = 'Core',
    }
    self:unify_piece_types(sym.chiral)
  end,
}
