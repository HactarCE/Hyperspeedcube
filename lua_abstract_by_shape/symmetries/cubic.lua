shapes:add('cube', {
  name = "Cube",
  ndim = 3,

  -- Magically constructed into cd'bc3'?
  sym = 'bc3',

  -- This defines the names and order, but not actually where the faces go on the puzzle.
  faces = {
    { short = 'R', long = "Right" },
    { short = 'L', long = "Left" },
    { short = 'U', long = "Up" },
    { short = 'D', long = "Down" },
    { short = 'F', long = "Front" },
    { short = 'B', long = "Back" },
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
  default_colors = "Western",

  iter_poles = function(self)
    -- This assigns names to faces.
    return self.sym
      :orbit(self.sym.oox.unit)
      :with({
        F = {},
        U = {3, 'F'},
        R = {2, 'U'},
        L = {1, 'R'},
        D = {2, 'L'},
        B = {3, 'D'},
      })
  end,

  build = function(self, p)
    p:carve(self.faces:iter_poles())
  end,
})
