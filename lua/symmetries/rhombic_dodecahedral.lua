AXIS_NAMES = {
  symmetry = cd'bc3',
  {'R', {'U', 2}},
  {'L', {'R', 1}},
  {'U', {'F', 3}},
  {'D', {'L', 2}},
  {'F', {}}, -- oox
  {'B', {'D', 3}},
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
