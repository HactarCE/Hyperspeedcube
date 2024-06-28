FACE_NAMES = {
  symmetry = 'bc3',
  {{2, "U"}, "R", "Right"},
  {{1, "R"}, "L", "Left"},
  {{3, "F"}, "U", "Up"},
  {{2, "L"}, "D", "Down"},
  {{      }, "F", "Front"}, -- oox
  {{3, "D"}, "B", "Back"},
}

CUBE_COLORS = {
  R = 'Red',
  L = 'Orange',
  U = 'White',
  D = 'Yellow',
  F = 'Green',
  B = 'Blue',
}

local meta = require('meta')

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
