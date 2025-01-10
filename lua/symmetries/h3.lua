function dodecahedron(scale, basis)
  return {
    name = "Dodecahedron",
    face_colors = 'dodecahedron',
    sym = cd('h3', basis),
    iter_poles = function(self, prefix)
      return self.sym:orbit(self.sym.oox.unit * (scale or 1)):named({
        F = {},
        U = {3, 'F'},
        R = {2, 'U'},
        L = {1, 'R'},
        DR = {2, 'L'},
        DL = {1, 'DR'},
        BR = {3, 'DR'},
        BL = {3, 'DL'},
        PR = {2, 'BL'},
        PL = {1, 'PR'},
        PD = {2, 'PL'},
        PB = {3, 'PD'},
      }):prefixed(prefix)
    end,
  }
end

function icosahedron(scale, basis)
  return {
    name = "Icosahedron",
    face_colors = 'icosahedron',
    sym = cd('h3', basis),
    iter_poles = function(self, prefix)
      return self.sym:orbit(self.sym.xoo.unit * (scale or 1)):named({
        UR = {},
        U = {1, 'UR'},
        R = {2, 'U'},
        US = {3, 'R'},
        UL = {1, 'US'},
        F = {1, 'R'},
        FR = {2, 'F'},
        UP = {3, 'FR'},
        BR = {2, 'UP'},
        S = {3, 'BR'},
        L = {1, 'BR'},
        FL = {1, 'S'},
        DS = {2, 'L'},
        DL = {1, 'DS'},
        DR = {2, 'FL'},
        BL = {3, 'DR'},
        P = {3, 'DL'},
        B = {2, 'BL'},
        D = {2, 'P'},
        DP = {1, 'D'}
      }):prefixed(prefix)
    end,
  }
end

DODECAHEDRON_FACE_COLORS = {
  { name = 'U',  display = "Up",         default = "White" },
  { name = 'F',  display = "Front",      default = "Green Dyad [2]" },
  { name = 'R',  display = "Right",      default = "Red" },
  { name = 'L',  display = "Left",       default = "Purple" },
  { name = 'BR', display = "Back-right", default = "Blue Dyad [2]" },
  { name = 'BL', display = "Back-left",  default = "Yellow Dyad [2]" },
  { name = 'DR', display = "Down-right", default = "Yellow Dyad [1]" },
  { name = 'DL', display = "Down-left",  default = "Blue Dyad [1]" },
  { name = 'PR', display = "Para-right", default = "Pink" },
  { name = 'PL', display = "Para-left",  default = "Orange" },
  { name = 'PB', display = "Para-back",  default = "Green Dyad [1]" },
  { name = 'PD', display = "Para-down",  default = "Gray" },
}

color_systems:add{
  id = 'dodecahedron',
  name = "Dodecahedron",

  colors = DODECAHEDRON_FACE_COLORS,
}

color_systems:add{
  id = 'icosahedron',
  name = "Icosahedron",

  colors = {
    { name = 'U',  display = "Up"},
    { name = 'F',  display = "Front"},
    { name = 'R',  display = "Right"},
    { name = 'L',  display = "Left"},
    { name = 'FR', display = "Front-Right"},
    { name = 'FL', display = "Front-Left"},
    { name = 'UR', display = "Up-Right"},
    { name = 'UL', display = "Up-Left"},
    { name = 'DR', display = "Down-Right"},
    { name = 'DL', display = "Down-Left"},
    { name = 'D',  display = "Down"},
    { name = 'B',  display = "Back"},
    { name = 'S',  display = "Starboard"},
    { name = 'P',  display = "Portside"},
    { name = 'BR', display = "Back-Right"},
    { name = 'BL', display = "Back-Left"},
    { name = 'US', display = "Up-Star"},
    { name = 'UP', display = "Up-Port"},
    { name = 'DS', display = "Down-Star"},
    { name = 'DP', display = "Down-Port"},
  },

  schemes = {
    {"Twizzle", {
      U = "White",
      F = "Green Dyad [2]",
      R = "Mono Tetrad[3]",
      L = "Orange",
      FR = "Yellow",
      FL = "Purple",
      UR = "Red",
      UL = "Blue Tetrad [2]",
      DR = "Magenta Triad [2]",
      DL = "Teal",
      D = "Mono Tetrad [1]",
      B = "Green",
      S = "Brown",
      P = "Black",
      BR = "Purple Dyad [1]",
      BL = "Yellow Dyad [1]",
      US = "Blue Tetrad [1]", -- should be more teal
      UP = "Pink",
      DS = "Blue",
      DP = "Red Tetrad [3]"
    }},
    {"Gradient A", {
      U = "White",
      F = "Green",
      R = "Orange",
      L = "Teal",
      FR = "Yellow",
      FL = "Green Tetrad [3]",
      UR = "Magenta Tetrad [1]",
      UL = "Blue Tetrad [1]",
      DR = "Yellow Tetrad [3]",
      DL = "Green Tetrad [4]",
      D = "Mono Tetrad [3]",
      B = "Purple Tetrad [3]",
      S = "Red",
      P = "Blue Tetrad [3]",
      BR = "Red Tetrad [3]",
      BL = "Purple Tetrad [2]",
      US = "Magenta Tetrad [2]",
      UP = "Purple Tetrad [1]",
      DS = "Orange Tetrad [3]",
      DP = "Blue Tetrad [4]",
    }},
    {"Gradient B", {
      U = "White",
      F = "Blue Tetrad [1]",
      R = "Purple Tetrad [1]",
      L = "Green",
      FR = "Blue Tetrad [2]",
      FL = "Teal",
      UR = "Magenta Tetrad [1]",
      UL = "Yellow",
      DR = "Blue Tetrad [3]",
      DL = "Blue Tetrad [4]",
      D = "Mono Tetrad [3]",
      B = "Red",
      S = "Purple Tetrad [2]",
      P = "Green Tetrad [3]",
      BR = "Red Tetrad [3]",
      BL = "Red Tetrad [1]",
      US = "Magenta Tetrad [2]",
      UP = "Orange",
      DS = "Cividis [1/1]",
      DP = "Green Tetrad [4]",
    }},
  },
  default = "Gradient A",
}
