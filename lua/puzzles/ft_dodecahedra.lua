local utils = require('utils')
local symmetries = require('symmetries')

local REALISITIC_PROPORTIONS = true
local CORNER_STALK_SIZE = 0.03

local function ft_dodecahedron_cut_depths(size)
  if size == 1 then return {1/phi} end

  local outermost_cut
  local aesthetic_limit = 1 - (1 - 1/phi)/size
  local mechanical_limit = 1
  if REALISITIC_PROPORTIONS then
    mechanical_limit = 1/29 * (10 + 7 * sqrt(5))
  end
  outermost_cut = min(aesthetic_limit, mechanical_limit - CORNER_STALK_SIZE)
  return utils.layers_inclusive(outermost_cut, 1/phi, size)
end

function define_ft_dodecahedron(size, id, name)
  puzzles:add(id, {
    ndim = 3,
    name = string.format("FT Dodecahedron %d (%s)", size, name),
    colors = 'dodecahedron',
    -- piece_types = {
    --   { id = 'centers', name = "Centers" },
    --   {
    --     id = 'moving', name = "Moving pieces",
    --     { id = 'edges', name = "Edges" },
    --     { id = 'corners', name = "Corners" },
    --   },
    -- },
    build = function(self)
      local sym = cd'h3'
      local shape = symmetries.dodecahedral.dodecahedron()
      self:carve(shape:iter_poles())

      -- Define axes and slices
      self.axes:add(shape:iter_poles(), ft_dodecahedron_cut_depths(size))

      -- Define twists
      for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox.unit], sym:thru(2, 1)) do
        self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
      end

      local R = self.axes.R
      local L = self.axes.L
      local U = self.axes.U
      local F = self.axes.F

      if size == 0 then
        self:mark_pieces('core', ~U'*') -- TODO: construct 'everything' region
        return
      else
        local U_adj = symmetry{self.twists.U}:orbit(R('*')):union()

        -- Centers
        for i = 2, size do
          for j = 2, size do
            local name
            if i == j then
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

        for i = 2, size do
          self:mark_pieces(string.format('t-centers (%d)', i-1), U(1) & F(i) & ~R(1, size) & ~L(1, size))
          self:mark_pieces(string.format('wings (%d)', i-1), U(1) & F(1) & R(i))
        end

        -- this is so, on a big cube, 'edges' and 'centers' can refer to 2c and 1c
        local middle_prefix
        if size > 1 then
          middle_prefix = 'middle '
        else
          middle_prefix = ''
        end

        self:mark_pieces(middle_prefix .. 'centers', U(1) & ~U_adj)
        self:mark_pieces(middle_prefix .. 'edges', U(1) & F(1) & ~R(1, size) & ~L(1, size))

        self:mark_pieces('corners', U(1) & F(1) & R(1))
        self:unify_piece_types(sym.chiral)
      end
    end,
  })
end

define_ft_dodecahedron(0, 'dodecahedron', "Dodecahedron")
define_ft_dodecahedron(1, 'megaminx', "Megaminx")
define_ft_dodecahedron(2, 'gigaminx', "Gigaminx")
define_ft_dodecahedron(3, 'teraminx', "Teraminx")
define_ft_dodecahedron(4, 'petaminx', "Petaminx")
define_ft_dodecahedron(5, 'examinx', "Examinx")
define_ft_dodecahedron(6, 'zettaminx', "Zettaminx")
define_ft_dodecahedron(7, 'yottaminx', "Yottaminx")
define_ft_dodecahedron(8, 'ronnaminx', "Ronnaminx")
define_ft_dodecahedron(9, 'atlasminx', "Atlasminx") -- quettaminx
define_ft_dodecahedron(10, 'minx_of_madness', "Minx of Madness") -- no metric prefix!
