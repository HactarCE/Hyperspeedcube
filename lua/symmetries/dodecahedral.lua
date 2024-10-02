color_systems:add{
  id = 'dodecahedron',
  name = "Dodecahedron",

  colors = {
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
  },
}

function dodecahedron(scale, basis)
  return {
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
