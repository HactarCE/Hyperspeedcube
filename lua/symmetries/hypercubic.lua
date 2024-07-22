color_systems:add('hypercube', {
  name = "Hypercube",

  colors = {
    { name = 'R', display = "Right", default = "Red" },
    { name = 'L', display = "Left",  default = "Orange" },
    { name = 'U', display = "Up",    default = "White" },
    { name = 'D', display = "Down",  default = "Yellow" },
    { name = 'F', display = "Front", default = "Green" },
    { name = 'B', display = "Back",  default = "Blue" },
    { name = 'O', display = "Out",   default = "Pink" },
    { name = 'I', display = "In",    default = "Purple" },
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
