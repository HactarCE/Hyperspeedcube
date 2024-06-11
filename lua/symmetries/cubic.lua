-- All of these assume BC3 symmetry

FACE_NAMES_SHORT = {
  symmetry = 'bc3',
  {'R', {2, 'U'}},
  {'L', {1, 'R'}},
  {'U', {3, 'F'}},
  {'D', {2, 'L'}},
  {'F', {}}, -- oox
  {'B', {3, 'D'}},
}

FACE_NAMES_LONG = require('utils').map_string_values(FACE_NAMES_SHORT, {
  R = 'Right',
  L = 'Left',
  U = 'Up',
  D = 'Down',
  F = 'Front',
  B = 'Back',
})

FACE_COLORS = {
  Right = 'red',
  Left = 'orange',
  Up = 'white',
  Down = 'yellow',
  Front = 'green',
  Back = 'blue',
}

VERTEX_AXIS_NAMES = {
  symmetry = cd'bc3',
  {'RUF', {}}, -- xoo
  {'L', {'RUF', 1}},
  {'D', {'L', 2}},
  {'BL', {'D', 1}},
  {'R', {'D', 3}},
  {'U', {'BL', 3}},
  {'BR', {'U', 2}},
  {'BD', {'BR', 1}}, -- B in standard notation
}

VERTEX_NAMES = {
  symmetry = cd'bc3',
  {'Front', {}}, -- xoo
  {'L', {'F', 1}},
  {'D', {'L', 2}},
  {'BL', {'D', 1}},
  {'R', {'D', 3}},
  {'U', {'BL', 3}},
  {'BR', {'U', 2}},
  {'BD', {'BR', 1}}, -- B in standard notation
}

local meta = require('meta')

cuboctahedron = {
  sym = cd'bc3',
  cubic_pole = function(self) return self.oox.unit end,
  octahedral_pole = function(self) return self.xoo * 2/3 end,
  carve_into = function(self, p)
    p:carve(self.sym:orbit(self:cubic_pole()):with(FACE_NAMES_LONG))
    p:carve(self.sym:orbit(self:octahedral_pole()):with(VERTEX_NAMES_LONG))
    -- TODO: cuboctahedron colors
  end,
}
cuboctahedron = setmetatable(cuboctahedron, meta.shape)

cube = {
  sym = cd'bc3',
  carve_into = function(self, p)
    p:carve(self.sym:orbit(self.oox.unit):with(FACE_NAMES_LONG))
    p.colors:set_defaults(FACE_COLORS)
  end,
}
cube = setmetatable(cube, meta.shape)
