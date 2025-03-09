twist_systems:add{
  id = 'ft_cube',
  name = "Face-Turning Cube",

  ndim = 3,
  symmetry = cd'bc3',
  build = function(self)
    local face_axes = self.axes:add(lib.symmetries.bc3.cube():iter_poles())

    -- Define twists
    for t, ax, rot in shape.sym.chiral:orbit(axes[1], shape.sym:thru(2, 1)) do
      self.twists:add(ax, rot, { gizmo_pole_distance = 1 })
    end

    self.vantage_sets:add{
      name = "Default",
      view_offset = rot{ fix = 'z', angle = degrees(35) }
                  * rot{ fix = 'y', angle = degrees(-20) },
      -- axes = self.axes, -- inferred
      twist_directions = {
        CW = function(ax) return tostring(ax.name) end,
        CCW = function(ax) return tostring(ax.name) .. "'" end,
      },
    }

    return {
      faces = face_axes,
    }
  end,
}

shapes:add{
  id = 'cube',
  name = "Cube",
  colors = 'cube',
  build = function(self)
    self:carve(lib.symmetries.bc3.cube():iter_poles())
  end,
}

twist_systems:add{
  id = 'ft_cuboctahedron',
  name = "Face-Turning Cuboctahedron",

  ndim = 3,
  symmetry = cd'bc3',
  build = function(self, cube_layers, octahedron_layers)
    local cube = lib.symmetries.bc3.cube()
    local cube_axes = self.axes:add(cube:iter_poles(), cube_layers)
    local octahedron_axes = self.axes:add(cube:iter_vertex_poles(), octahedron_layers)

    -- Define twists
    for t, ax, rot in shape.sym.chiral:orbit(cube_axes[1], shape.sym:thru(2, 1)) do
      self.twists:add(ax, rot, { gizmo_pole_distance = sqrt(3)/2 })
    end
    -- Define twists
    for t, ax, rot in shape.sym.chiral:orbit(octahedron_axes[1], shape.sym:thru(3, 2)) do
      self.twists:add(ax, rot, { gizmo_pole_distance = 1 })
    end

    self.vantage_sets:add{
      name = "Default",
      view_offset = rot{ fix = 'z', angle = degrees(35) }
                  * rot{ fix = 'y', angle = degrees(-20) },
      -- axes = self.axes, -- inferred
      twist_directions = {
        CW = function(ax) return tostring(ax.name) end,
        CCW = function(ax) return tostring(ax.name) .. "'" end,
      },
    }

    return {
      cube = cube_axes,
      octahedron = octahedron_axes,
    }
  end
}

puzzles:add{
  id = 'ft_cuboctahedron',
  version = '0.1.0',
  name = "FT Cuboctahedron",
  shape = 'cuboctahedron',
  twists = 'ft_cuboctahedron',
  build = function(self)
    self.twists = build_twists_system('ft_cuboctahedron', {INF, sqrt(3)/2 * 3/4}, {INF, 3/4})
  end,
}
