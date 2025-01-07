-- Symmetry authored by Luna Harran and Jason White
color_systems:add{
    id = 'tetrahedron',
    name = "Tetrahedron",

    colors = {
        { name = 'R', display = "Right", default = "Red" },
        { name = 'F', display = "Left", default = "Green" },
        { name = 'U', display = "Up", default = "Yellow" },
        { name = 'L', display = "Back", default = "Blue" },
    },
}
  
  function tetrahedron(scale, basis)
    return {
      sym = cd('a3', basis),
      iter_poles = function(self, prefix)
        return self.sym:orbit(self.sym.oox.unit * (scale or 1)):named({
          F = {},
          U = {3, 'F'},
          R = {2, 'U'},
          L = {1, 'R'},
        }):prefixed(prefix)
      end,
      iter_vertices = function(self, prefix)
        return self.sym:orbit(self.sym.xoo.unit * (scale or 1)):named({ -- labelled based on opposite face? lowercase ok?
          l= {},
          r = {1, 'l'},
          u = {2, 'r'},
          f = {3, 'u'},
        }):prefixed(prefix)
      end,
    }
  end
  