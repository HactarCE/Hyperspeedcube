AXIS_NAMES = {
  symmetry = cd'bc3',
  {'F', {}}, -- xoo
  {'L', {'F', 1}},
  {'D', {'L', 2}},
  {'BL', {'D', 1}},
  {'R', {'D', 3}},
  {'U', {'BL', 3}},
  {'BR', {'U', 2}},
  {'BD', {'BR', 1}}, -- B in standard notation
}

FACE_NAMES = require('utils').map_string_values(AXIS_NAMES, {
  F = 'Front',
  L = 'Left',
  D = 'Down',
  BL = 'Back-left',
  R = 'Right',
  U = 'Up',
  BR = 'Back-right',
  BD = 'Back-down',
})

-- TODO: octahedron color scheme
FACE_COLORS = {}
