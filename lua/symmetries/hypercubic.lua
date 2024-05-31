AXIS_NAMES = {
  symmetry = cd'bc4',
  {'R', {'U', 2}},
  {'L', {'R', 1}},
  {'U', {'F', 3}},
  {'D', {'L', 2}},
  {'F', {'O', 4}},
  {'B', {'D', 3}},
  {'I', {'B', 4}},
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
