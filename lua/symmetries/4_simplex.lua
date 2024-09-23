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


function simplex_4d()
  return {
    sym = cd'a4',
    iter_poles = function(self)
      return self.sym:orbit(self.sym.ooox.unit):named({
        O = {},
        D = {4, 'O'},
        F = {3, 'D'},
        BR = {2, 'F'},
        BL = {1, 'BR'},
      })
    end,
    iter_vertices = function(self)
      return self.sym:orbit(self.sym.xooo.unit):named({
        R = {},
        L = {1, 'R'},
        B = {2, 'L'},
        U = {3, 'B'},
        I = {4, 'U'},
      })
    end,
  }
end
