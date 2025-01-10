HYPERCUBE_FACET_NAMES = {
  R = {2, 'U'},
  L = {1, 'R'},
  U = {3, 'F'},
  D = {2, 'L'},
  F = {4, 'O'},
  B = {3, 'D'},
  I = {4, 'B'},
  O = {},
}

function hypercube(scale, basis)
  return {
    name = "Hypercube",
    aliases = { "4-Cube" },
    face_colors = 'hypercube',
    sym = cd('bc4', basis),
    iter_poles = function(self, prefix)
      return self.sym:orbit(self.sym.ooox.unit * (scale or 1)):named(HYPERCUBE_FACET_NAMES):prefixed(prefix)
    end,
  }
end

color_systems:add{
  id = 'hypercube',
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
}
