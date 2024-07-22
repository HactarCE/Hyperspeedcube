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
})

function octahedron()
  return {
    sym = cd'bc3',
    iter_poles = function(self)
      return self.sym:orbit(self.sym.xoo.unit):named({
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
