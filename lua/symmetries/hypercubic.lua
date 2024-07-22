color_systems:add('hypercube', {
  name = "Hypercube",

  colors = {
    { name = 'R', display = "Right" },
    { name = 'L', display = "Left" },
    { name = 'U', display = "Up" },
    { name = 'D', display = "Down" },
    { name = 'F', display = "Front" },
    { name = 'B', display = "Back" },
    { name = 'O', display = "Out" },
    { name = 'I', display = "In" },
  },

  default_scheme = {
    R = "Red",
    L = "Orange",
    U = "White",
    D = "Yellow",
    F = "Green",
    B = "Blue",
    O = "Pink",
    I = "Purple",
  },
})

function hypercube()
  return {
    sym = cd'bc4',
    iter_poles = function(self)
      return self.sym:orbit(self.sym.ooox.unit):named({
        R = {2, "U"},
        L = {1, "R"},
        U = {3, "F"},
        D = {2, "L"},
        F = {4, "O"},
        B = {3, "D"},
        I = {4, "B"},
        O = {},
      })
    end,
  }
end
