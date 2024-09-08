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
    meta = {
      author = {'Andrew Farkas', 'Milo Jacquet'},
    },
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

      local center_layer = size + 1
      local R = self.axes.R
      local L = self.axes.L
      local U = self.axes.U
      local F = self.axes.F

      -- Mark piece types
      if size == 0 then
        self:mark_piece{
          region = ~U'*', -- TODO: construct 'everything' region
          name = 'core',
          display = "Core",
        }
      else
        local U_adj = symmetry{self.twists.U}:orbit(R('*')):union()

        -- Centers
        self:add_piece_type{ name = 'center', display = "Center" }
        for i = 2, center_layer do
          for j = 2, size do
            local region
            if i == center_layer then
              region = U(1) & F(j) & ~R(1, size) & ~L(1, size)
            else
              region = U(1) & R(i) & F(j)
            end

            local name, display
            if i == center_layer then
              name, display = string.fmt2('center/t_%d', "T-center (%d)", j-1)
            elseif i == j then
              name, display = string.fmt2('center/x_%d', "X-center (%d)", i-1)
            else
              if i < j then
                name, display = string.fmt2('center/oblique_%d_%d', "Oblique (%d, %d)", i-1, j-1)
                self:add_piece_type{ name = name, display = display }
                name = name .. '/left'
                display = display .. " (left)"
              else
                name, display = string.fmt2('center/oblique_%d_%d', "Oblique (%d, %d)", i-1, j-1)
                name = name .. '/right'
                display = display .. " (right)"
              end
            end
            self:mark_piece{ region = region, name = name, display = display }
          end
        end

        -- Edges
        self:add_piece_type{ name = 'edge', display = "Edge" }
        for i = 2, size do

          local name, display = string.fmt2('edge/wing_%d', "Wing (%d)", i-1)
          self:mark_piece{
            region = U(1) & F(1) & R(i),
            name = name,
            display = display,
          }
        end

        -- Middle centers and edges
        local middle_suffix = ''
        local center_display, edge_display -- nil is ok here
        if size > 1 then
          middle_suffix = '/middle'
          center_display = "Middle center"
          edge_display = "Middle edge"
        end

        self:mark_piece{
          region = U(1) & ~U_adj,
          name = 'center' .. middle_suffix,
          display = center_display,
        }
        self:mark_piece{
          region = U(1) & F(1) & ~R(1, size) & ~L(1, size),
          name = 'edge' .. middle_suffix,
          display = edge_display,
        }

        self:mark_piece{
          region = U(1) & F(1) & R(1),
          name = 'corner',
          display = "Corner",
        }

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

puzzles:add('megaminx_crystal', {
  ndim = 3,
  name = 'Megaminx Crystal',
  colors = 'dodecahedron',
  meta = {
    author = 'Milo Jacquet',
  },
  build = function(self)
    local sym = cd'h3'
    local shape = symmetries.dodecahedral.dodecahedron()
    self:carve(shape:iter_poles())

    -- Define axes and slices
    depth = 0.54 -- intermediate puzzle
    self.axes:add(shape:iter_poles(), {depth, -depth})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox.unit], sym:thru(2, 1)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    local R = self.axes.R
    local L = self.axes.L
    local U = self.axes.U
    local F = self.axes.F
    local BR = self.axes.BR
    local DR = self.axes.DR

    self:mark_piece{
      region = U(1) & symmetry{self.twists.U}:orbit(R(2)):intersection(),
      name = 'center',
      display = "Center",
    }
    self:mark_piece{
      region = U(1) & F(1) & R(2) & L(2),
      name = 'megaminx_edge',
      display = "Megaminx edge",
    }
    self:mark_piece{
      region = L(2) & BR(2) & DR(2) & U(1) & R(1) & F(1),
      name = 'corner',
      display = "Corner",
    }
    self:mark_piece{
      region = L(1) & R(1),
      name = 'crystal_edge',
      display = "Crystal edge",
    }
    self:unify_piece_types(sym.chiral)
  end,
})

puzzles:add('pyraminx_crystal', {
  ndim = 3,
  name = 'Pyraminx Crystal',
  colors = 'dodecahedron',
  meta = {
    author = 'Milo Jacquet',
  },
  build = function(self)
    local sym = cd'h3'
    local shape = symmetries.dodecahedral.dodecahedron()
    self:carve(shape:iter_poles())

    -- Define axes and slices
    depth = 1/sqrt(5)
    self.axes:add(shape:iter_poles(), {depth, -depth})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox.unit], sym:thru(2, 1)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    local R = self.axes.R
    local L = self.axes.L
    local U = self.axes.U
    local F = self.axes.F
    local BR = self.axes.BR
    local DR = self.axes.DR

    self:mark_piece{
      region = L(2) & BR(2) & DR(2) & U(1),
      name = 'corner',
      display = "Corner",
    }
    self:mark_piece{
      region = L(1) & R(1),
      name = 'edge',
      display = "Edge",
    }
    self:unify_piece_types(sym.chiral)
  end,
})

puzzles:add('curvy_starminx', {
  ndim = 3,
  name = 'Curvy Starminx',
  colors = 'dodecahedron',
  meta = {
    author = 'Milo Jacquet',
  },
  build = function(self)
    local sym = cd'h3'
    local shape = symmetries.dodecahedral.dodecahedron()
    self:carve(shape:iter_poles())

    -- Define axes and slices
    local depth = 0.33 -- intermediate puzzle
    self.axes:add(shape:iter_poles(), {depth, -depth})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox.unit], sym:thru(2, 1)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    local R = self.axes.R
    local L = self.axes.L
    local U = self.axes.U
    local F = self.axes.F
    local BR = self.axes.BR
    local BL = self.axes.BL
    local DR = self.axes.DR

    self:mark_piece{
      region = L(2) & BR(2) & DR(2) & U(1),
      name = 'corner',
      display = "Corner",
    }
    self:mark_piece{
      region = BR(2) & BL(2) & R(1) & L(1),
      name = 'edge',
      display = "Edge",
    }
    self:mark_piece{
      region = F(2) & R(1) & BR(1) & BL(1) & L(1),
      name = 'x_center',
      display = "X-center",
    }
    self:mark_piece{
      region = F(1) & R(1) & BR(1) & BL(1) & L(1),
      name = 'center',
      display = "Center",
    }
    self:unify_piece_types(sym.chiral)
  end,
})

puzzles:add('starminx', {
  ndim = 3,
  name = 'Starminx',
  colors = 'dodecahedron',
  meta = {
    author = 'Milo Jacquet',
  },
  build = function(self)
    local sym = cd'h3'
    local shape = symmetries.dodecahedral.dodecahedron()
    self:carve(shape:iter_poles())

    -- Define axes and slices
    local depth = sqrt(5) - 2
    self.axes:add(shape:iter_poles(), {depth, -depth})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox.unit], sym:thru(2, 1)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    local R = self.axes.R
    local L = self.axes.L
    local U = self.axes.U
    local F = self.axes.F
    local BR = self.axes.BR
    local BL = self.axes.BL
    local DR = self.axes.DR

    self:mark_piece{
      region = BR(2) & BL(2) & R(1) & L(1),
      name = 'edge',
      display = "edge",
    }
    self:mark_piece{
      region = U(2) & L(1) & R(1),
      name = 'x_center',
      display = "X-center",
    }
    self:mark_piece{
      region = F(1) & R(1) & BR(1) & BL(1) & L(1),
      name = 'center',
      display = "Center",
    }
    self:unify_piece_types(sym.chiral)
  end,
})


local function pentultimate_cut_depths(size)
  if size == 2 then return {0} end

  local outermost_cut
  local aesthetic_limit = (1 - 2/(size+0.6)) * (sqrt(5) - 2)
  local mechanical_limit = sqrt(5) - 2
  if REALISITIC_PROPORTIONS then
    -- this is the negative of the galois conjugate of the corresponding value for the megaminx
    mechanical_limit = (-10 + 7 * sqrt(5)) / 29
  end
  outermost_cut = min(aesthetic_limit, mechanical_limit - CORNER_STALK_SIZE)
  return utils.layers_inclusive(outermost_cut, -outermost_cut, size-1)
end

function define_pentultimate(size, name)
  puzzles:add(string.format("pentultimate_%d", size), {
    ndim = 3,
    name = string.format("Pentultimate %d (%s)", size, name),
    colors = 'dodecahedron',
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
      local sym = cd'h3'
      local shape = symmetries.dodecahedral.dodecahedron()
      self:carve(shape:iter_poles())

      -- Define axes and slices
      self.axes:add(shape:iter_poles(), pentultimate_cut_depths(size))

      -- Define twists
      for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox.unit], sym:thru(2, 1)) do
        self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
      end

      local R = self.axes.R
      local L = self.axes.L
      local U = self.axes.U
      local F = self.axes.F
      local BR = self.axes.BR
      local BL = self.axes.BL
      local DR = self.axes.DR
      local DL = self.axes.DL

      local center_layer = ceil(size/2)

      local middle_prefix = ''
      if size > 3 then
        middle_prefix = 'middle '
      end

      -- Centers
      for i = 2, center_layer do
        for j = i, size+1-i do
          local name, display
          local name2, display2
          if i == j and j*2 - 1 == size then
            if size > 3 then
              name = 'edge/middle'
              display = "Middle edge"
            else
              name = 'edge'
              display = "Edge"
            end
          elseif i == j then
            name, display = string.fmt2('edge/wing_%d', "Wing (%d)", i-1)
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
            region = U(1) & BL(j) & DL(i),
            name = name,
            display = display,
          }
          if name2 ~= nil then
            self:mark_piece{
              region = U(1) & BL(i) & DL(j),
              name = name2,
              display = display2,
            }
          end
        end
      end

      for i = 2, floor(size/2) do
        local name, display = string.fmt2('center/x_outer_%d', "Outer X-center (%d)", i-1)
        self:mark_piece{
          region = DR(i) & L(1) & BR(1),
          name = name,
          display = display
        }
        name, display = string.fmt2('center/x_inner_%d', "Inner X-center (%d)", i-1)
        self:mark_piece{
          region = DR(size+1-i) & L(1) & BR(1),
          name = name,
          display = display
        }
      end

      if size % 2 == 1 then
        local name, display
        if size > 3 then
          name = 'center/x_middle'
          display = "Middle X-center"
        else
          name = 'center/x'
          display = "X-center"
        end
        name = self:mark_piece{
          region = DR(center_layer) & L(1) & BR(1),
          name = name,
          display = display,
        }
      end

      self:mark_piece{
        region = F(1) & R(1) & BR(1) & BL(1) & L(1),
        name = 'center',
        display = "Center",
      }
      self:mark_piece{
        region = L(1) & BR(1) & DR(1),
        name = 'corner',
        display = "Corner",
      }
      self:unify_piece_types(sym.chiral)
    end,
  })
end

define_pentultimate(2, "Pentultimate")
define_pentultimate(3, "Master Pentultimate")
define_pentultimate(4, "Elite Pentultimate")
define_pentultimate(5, "Royal Pentultimate")
define_pentultimate(6, "6-layer Pentultimate")
define_pentultimate(7, "God Emperor Pentultimate")
