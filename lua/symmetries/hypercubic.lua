AXIS_NAMES = {
  symmetry = cd'bc4',
  {'R', {2, 'U'}},
  {'L', {1, 'R'}},
  {'U', {3, 'F'}},
  {'D', {2, 'L'}},
  {'F', {4, 'O'}},
  {'B', {3, 'D'}},
  {'I', {4, 'B'}},
  {'O', {}}, -- ooox
}

FACE_NAMES = require('utils').map_string_values(AXIS_NAMES, {
  R = 'Right',
  L = 'Left',
  U = 'Up',
  D = 'Down',
  F = 'Front',
  B = 'Back',
  I = 'In',
  O = 'Out',
})

FACE_COLORS = {
  Right = 'red',
  Left = 'orange',
  Up = 'white',
  Down = 'yellow',
  Front = 'green',
  Back = 'blue',
  Out = 'pink',
  In = 'purple',
}
