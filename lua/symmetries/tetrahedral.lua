FACE_AXIS_NAMES = {
  symmetry = cd'a3',
  {'r', {2, 'f'}},
  {'l', {1, 'r'}},
  {'f', {3, 'd'}},
  {'d', {}}, -- oox
}
VERTEX_AXIS_NAMES = {
  symmetry = cd'a3',
  {'R', {}}, -- xoo
  {'L', {1, 'R'}},
  {'U', {3, 'B'}},
  {'B', {2, 'L'}},
}

FACE_NAMES = require('utils').map_string_values(FACE_AXIS_NAMES, {
  r = 'Right',
  l = 'Left',
  f = 'Front',
  d = 'Down',
})

FACE_COLORS = {
  Right = 'blue',
  Left = 'red',
  Front = 'green',
  Down = 'yellow',
}
