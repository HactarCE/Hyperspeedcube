color_systems:add('4_simplex', {
  name = "4-simplex",

  colors = {
    { name = 'r', display = "Right", default = "Red" },
    { name = 'l', display = "Left",  default = "Yellow" },
    { name = 'd', display = "Down",  default = "Green" },
    { name = 'b', display = "Back",  default = "Blue" },
    { name = 'i', display = "In",    default = "Purple" },
  },
})

function simplex_4d()
  return {
    sym = cd'a4',
    iter_poles = function(self)
      return self.sym:orbit(self.sym.ooox.unit):named({
        r = {},
        l = {1, 'r'},
        d = {2, 'l'},
        b = {3, 'd'},
        i = {4, 'b'},
      })
    end,
    iter_vertices = function(self)
      return self.sym:orbit(self.sym.xooo.unit):named({
        R = {2, 'U'},
        L = {1, 'R'},
        U = {3, 'F'},
        F = {4, 'O'},
        O = {},
      })
    end,
  }
end
