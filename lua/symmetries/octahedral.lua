color_systems:add('octahedron', {
  name = "Octahedron",

  colors = {
    { name = "R", display = "Right"},
    { name = "L", display = "Left"},
    { name = "U", display = "Up"},
    { name = "D", display = "Down"},
    { name = "F", display = "Front"},
    { name = "BR", display = "Back-right"},
    { name = "BL", display = "Back-left"},
    { name = "BD", display = "Back-down"},
  },

  color_schemes = {
    {"Lanlan", {
      R = "Green",
      L = "Purple",
      U = "Mono Dyad [1]",
      D = "Yellow",
      F = "Red",
      BR = "Mono Dyad [2]",
      BL = "Orange",
      BD = "Blue",
    }},
    {"Benpuzzles Classic", {
      R = "Yellow",
      L = "Cyan",
      U = "Mono Dyad [1]",
      D = "Mono Dyad [2]",
      F = "Green",
      BR = "Red",
      BL = "Blue",
      BD = "Magenta",
    }},
    {"Benpuzzles Alt", {
      R = "Red",
      L = "Yellow",
      U = "Mono Dyad [1]",
      D = "Mono Dyad [2]",
      F = "Green",
      BR = "Purple",
      BL = "Orange",
      BD = "Blue", -- lighter
    }},
    {"Diansheng", {
      R = "Red",
      L = "Purple",
      U = "Mono Dyad [1]",
      D = "Yellow",
      F = "Green",
      BR = "Mono Dyad [2]",
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
  default_scheme = "Diansheng",
})

function octahedron()
  return {
    sym = cd'bc3',
    iter_poles = function(self)
      return self.sym:orbit(self.sym.xoo.unit):with({
        R = {3, "D"},
        L = {1, "F"},
        U = {3, "BL"},
        D = {2, "L"},
        F = {},
        BR = {2, "U"},
        BL = {1, "D"},
        BD = {1, "BR"}, -- B in standard notation
      })
    end,
  }
end
