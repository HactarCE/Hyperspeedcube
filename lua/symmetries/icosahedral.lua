FACE_COLORS = {
  { name = 'U',  display = "Up",         default = "White" },
  { name = 'F',  display = "Front",      default = "Green Dyad [2]" },
  { name = 'R',  display = "Right",      default = "Mono Tetrad [3]" },
  { name = 'L',  display = "Left",       default = "Orange" },
  { name = 'FR', display = "Front-Right", default = "Yellow"},
  { name = 'FL', display = "Front-Left", default = "Purple"},
  { name = 'UR', display = "Up-Right", default = "Red"},
  { name = 'UL', display = "Up-Left", default = "Blue Tetrad [2]"},
  { name = 'DR', display = "Down-Right", default = "Magenta Triad [2]"},
  { name = 'DL', display = "Down-Left", default = "Teal"},

  { name = 'D',  display = "Down",         default = "Mono Tetrad [1]" },
  { name = 'B',  display = "Back",      default = "Green" },
  { name = 'S',  display = "Starboard",      default = "Brown" },
  { name = 'P',  display = "Portside",       default = "Black" },
  { name = 'BR', display = "Back-Right", default = "Purple Dyad [1]"},
  { name = 'BL', display = "Back-Left", default = "Yellow Dyad [1]"},
  { name = 'US', display = "Up-Star", default = "Blue Tetrad [1]"},
  { name = 'UP', display = "Up-Port", default = "Pink"},
  { name = 'DS', display = "Down-Star", default = "Blue"},
  { name = 'DP', display = "Down-Port", default = "Red Tetrad [3]"},
  
}

color_systems:add{
  id = 'icosahedron',
  name = "Icosahedron",

  colors = FACE_COLORS,
}

function icosahedron(scale, basis)
  return {
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
