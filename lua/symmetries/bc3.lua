function cube(scale, basis)
  return {
    name = "Cube",
    aliases = { "3-Cube", "Hexahedron", "1x1x1" },
    face_colors = 'cube',
    sym = cd('bc3', basis),
    iter_poles = function(self, prefix)
      return self.sym:orbit(self.sym.oox.unit * (scale or 1)):named({
        F = {},
        U = {3, 'F'},
        R = {2, 'U'},
        L = {1, 'R'},
        D = {2, 'L'},
        B = {3, 'D'},
      }):prefixed(prefix)
    end,
    iter_edge_poles = function(self, prefix)
      local charset = names.charset
      return self.sym:orbit(self.sym.oxo.unit * (scale or 1) * sqrt(2)):named({
        [charset'FU'] = {},
        [charset'FR'] = {2, 'FU'},
        [charset'FL'] = {1, 'FR'},
        [charset'FD'] = {2, 'FL'},
        [charset'UR'] = {3, 'FR'},
        [charset'UL'] = {3, 'FL'},
        [charset'DR'] = {2, 'UL'},
        [charset'DL'] = {1, 'DR'},
        [charset'BU'] = {3, 'FD'},
        [charset'BR'] = {2, 'BU'},
        [charset'BL'] = {1, 'BR'},
        [charset'BD'] = {2, 'BL'},
      }):prefixed(prefix)
    end,
  }
end

function octahedron(scale, basis)
  return {
    name = "Octahedron",
    aliases = { "4-Orthoplex" },
    face_colors = 'octahedron',
    sym = cd('bc3', basis),
    iter_poles = function(self, prefix)
      return self.sym:orbit(self.sym.xoo.unit * (scale or 1)):named({
        R = {3, 'D'},
        L = {1, 'F'},
        U = {3, 'BL'},
        D = {2, 'L'},
        F = {},
        BR = {2, 'U'},
        BL = {1, 'D'},
        BD = {1, 'BR'}, -- B in standard notation
      }):prefixed(prefix)
    end,
  }
end

-- Cube
color_systems:add{
  id = 'cube',
  name = "Cube",

  colors = {
    { name = 'R', display = "Right" },
    { name = 'L', display = "Left" },
    { name = 'U', display = "Up" },
    { name = 'D', display = "Down" },
    { name = 'F', display = "Front" },
    { name = 'B', display = "Back" },
  },

  schemes = {
    {"Western", {
      R = "Red",
      L = "Orange",
      U = "White",
      D = "Yellow",
      F = "Green",
      B = "Blue",
    }},
    {"Japanese", {
      R = "Red",
      L = "Orange",
      U = "White",
      D = "Blue",
      F = "Green",
      B = "Yellow",
    }},
  },
  default = "Western",
}

-- Octahedron
color_systems:add{
  id = 'octahedron',
  name = "Octahedron",

  colors = {
    { name = 'R', display = "Right"},
    { name = 'L', display = "Left"},
    { name = 'U', display = "Up"},
    { name = 'D', display = "Down"},
    { name = 'F', display = "Front"},
    { name = 'BR', display = "Back-right"},
    { name = 'BL', display = "Back-left"},
    { name = 'BD', display = "Back-down"},
  },

  schemes = {
    {"Lanlan", {
      R = "Green",
      L = "Purple",
      U = "White",
      D = "Yellow",
      F = "Red",
      BR = "Gray",
      BL = "Orange",
      BD = "Blue",
    }},
    {"Benpuzzles Classic", {
      R = "Yellow",
      L = "Cyan",
      U = "White",
      D = "Gray",
      F = "Green",
      BR = "Red",
      BL = "Blue",
      BD = "Magenta",
    }},
    {"Benpuzzles Alt", {
      R = "Red",
      L = "Yellow",
      U = "White",
      D = "Gray",
      F = "Green",
      BR = "Purple",
      BL = "Orange",
      BD = "Blue", -- lighter
    }},
    {"Diansheng", {
      R = "Red",
      L = "Purple",
      U = "White",
      D = "Yellow",
      F = "Green",
      BR = "Gray",
      BL = "Orange",
      BD = "Blue",
    }},
    {"MF8", {
      R = "Red",
      L = "Pink",
      U = "White",
      D = "Yellow",
      F = "Green",
      BR = "Purple",
      BL = "Orange",
      BD = "Blue", -- lighter
    }},
  },
  default = "Diansheng",
}
