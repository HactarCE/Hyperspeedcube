HYPERCUBE_FACET_NAMES = {
  A = {},
  O = {5, 'A'},
  F = {4, 'O'},
  U = {3, 'F'},
  R = {2, 'U'},
  L = {1, 'R'},
  D = {2, 'L'},
  B = {3, 'D'},
  I = {4, 'B'},
  P = {5, 'I'},
}

function hypercube(scale, basis)
  return {
    name = "5-Cube",
    face_colors = '5_cube',
    sym = cd('bc5', basis),
    iter_poles = function(self, prefix)
      return self.sym:orbit(self.sym.oooox.unit * (scale or 1)):named(HYPERCUBE_FACET_NAMES):prefixed(prefix)
    end,
  }
end

color_systems:add{
  id = '5_cube',
  name = "5-cube",

  colors = {
    { name = 'R', display = "Right",     default = "Red" },
    { name = 'L', display = "Left",      default = "Orange" },
    { name = 'U', display = "Up",        default = "White" },
    { name = 'D', display = "Down",      default = "Yellow" },
    { name = 'F', display = "Front",     default = "Green" },
    { name = 'B', display = "Back",      default = "Blue" },
    { name = 'O', display = "Out",       default = "Pink" },
    { name = 'I', display = "In",        default = "Purple" },
    { name = 'A', display = "Anterior",  default = "Black" },
    { name = 'P', display = "Posterior", default = "Gray" },
  },
}
