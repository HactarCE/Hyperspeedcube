color_systems:add('cube', {
  name = "Cube",

  colors = {
    { name = 'R', display = "Right" },
    { name = 'L', display = "Left" },
    { name = 'U', display = "Up" },
    { name = 'D', display = "Down" },
    { name = 'F', display = "Front" },
    { name = 'B', display = "Back" },
  },

  color_schemes = {
    ["Western"] = {
      R = "Red",
      L = "Orange",
      U = "White",
      D = "Yellow",
      F = "Green",
      B = "Blue",
    },
    ["Japanese"] = {
      R = "Red",
      L = "Orange",
      U = "White",
      D = "Blue",
      F = "Green",
      B = "Yellow",
    },
  },
  default_scheme = "Western",
})

function cube()
  return {
    sym = cd'bc3',
    iter_poles = function(self)
      return self.sym:orbit(self.sym.oox.unit):with({
        F = {},
        U = {3, 'F'},
        R = {2, 'U'},
        L = {1, 'R'},
        D = {2, 'L'},
        B = {3, 'D'},
      })
    end,
  }
end
