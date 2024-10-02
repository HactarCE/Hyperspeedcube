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

function cube(scale, basis)
  return {
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
  }
end
