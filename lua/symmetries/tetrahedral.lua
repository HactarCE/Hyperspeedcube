FACE_AXIS_NAMES = {
  symmetry = cd'a3',
  {'r', {'f', 2}},
  {'l', {'r', 1}},
  {'f', {'d', 3}},
  {'d', {}}, -- oox
}
VERTEX_AXIS_NAMES = {
  symmetry = cd'a3',
  {'R', {}}, -- xoo
  {'L', {'R', 1}},
  {'U', {'B', 3}},
  {'B', {'L', 2}},
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
