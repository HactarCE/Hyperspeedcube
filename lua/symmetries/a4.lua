color_systems:add{
  id = '4_simplex',
  name = "4-Simplex",

  colors = {
    { name = 'O',  display = "O",  default = "Purple" },
    { name = 'D',  display = "D",  default = "Yellow" },
    { name = 'F',  display = "F",  default = "Green" },
    { name = 'BR', display = "BR", default = "Blue" },
    { name = 'BL', display = "BL", default = "Red" },
  },
}


function simplex_4d(scale, basis)
  return {
    sym = cd('a4', basis),
    iter_poles = function(self, prefix)
      return self.sym:orbit(self.sym.ooox.unit * (scale or 1)):named({
        O = {},
        D = {4, 'O'},
        F = {3, 'D'},
        BR = {2, 'F'},
        BL = {1, 'BR'},
      }):prefixed(prefix)
    end,
    iter_vertices = function(self, prefix)
      return self.sym:orbit(self.sym.xooo.unit * (scale or 1)):named({
        R = {},
        L = {1, 'R'},
        B = {2, 'L'},
        U = {3, 'B'},
        I = {4, 'U'},
      }):prefixed(prefix)
    end,
  }
end
