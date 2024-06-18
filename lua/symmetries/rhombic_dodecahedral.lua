AXIS_NAMES = {
  symmetry = cd'bc3',
  {'R', {2, 'U'}},
  {'L', {1, 'R'}},
  {'U', {3, 'F'}},
  {'D', {2, 'L'}},
  {'F', {}}, -- oox
  {'B', {3, 'D'}},
}

FACE_NAMES = require('utils').map_string_values(AXIS_NAMES, {
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
