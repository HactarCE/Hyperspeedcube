AXIS_NAMES = {
  symmetry = cd'bc3',
  {'F', {}}, -- xoo
  {'L', {1, 'F'}},
  {'D', {2, 'L'}},
  {'BL', {1, 'D'}},
  {'R', {3, 'D'}},
  {'U', {3, 'BL'}},
  {'BR', {2, 'U'}},
  {'BD', {1, 'BR'}}, -- B in standard notation
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
