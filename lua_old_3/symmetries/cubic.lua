local meta = require('meta')

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

function cuboctahedron()
  local shape = setmetatable({}, meta.shape)
  shape.sym = cd'bc3'
  shape.cubic_pole = shape.oox.unit
  shape.octahedral_pole = shape.oox.unit * 2/3

  function shape:carve_into(p)
    p:carve(self.sym:orbit(self.cubic_pole):with(FACE_NAMES))
    p:carve(self.sym:orbit(self.octahedral_pole)) -- TODO: vertex names
    -- TODO: cuboctahedron colors
  end

  return shape
end

function cube()
  local shape = setmetatable({}, meta.shape)
  shape.sym = cd'bc3'
  shape.pole = shape.oox.unit

  function shape:carve_into(p)
    p:carve(self.sym:orbit(self.pole):with(FACE_NAMES))
    p.colors:set_defaults(CUBE_COLORS)
  end

  return shape
end

shapes:add('cuboctahedron', {
  name = "Cuboctahedron",
  ndim = 3,
  sym = 'bc3',
  faces = FACES_AND_VERTICES, -- TODO
  build = function(shape)
    -- TODO
    -- shape.symmetry =
    -- shape.cubic_pole = cd'bc3'
    -- shape.octahedral_pole =
    -- function shape:carve_into(p)
    --   p:carve(shape.sym:orbit(shape.symmetry):with())
    -- end
  end,
})
