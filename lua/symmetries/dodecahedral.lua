AXIS_NAMES = {
  symmetry = cd'h3',
  {'U', {}}, -- oox
  {'F', {'U', 3}},
  {'R', {'F', 2}},
  {'L', {'R', 1}},
  {'BR', {'L', 2}},
  {'BL', {'BR', 1}},
  {'DR', {'BR', 3}},
  {'DL', {'BL', 3}},
  {'PR', {'DL', 2}},
  {'PL', {'PR', 1}},
  {'PB', {'PL', 2}},
  {'PD', {'PB', 3}},
}

FACE_NAMES = require('utils').map_string_values(AXIS_NAMES, {
  U = 'Up',
  F = 'Front',
  R = 'Right',
  L = 'Left',
  BR = 'Back-right',
  BL = 'Back-left',
  DR = 'Down-right',
  DL = 'Down-left',
  PR = 'Para-right',
  PL = 'Para-left',
  PB = 'Para-back',
  PD = 'Para-down',
})

FACE_COLORS = {
  ['Up'] = 'white',
  ['Front'] = 'green',
  ['Right'] = 'red',
  ['Left'] = 'purple',
  ['Back-right'] = 'blue',
  ['Back-left'] = 'yellow',
  ['Down-right'] = 'light yellow',
  ['Down-left'] = 'light blue',
  ['Para-right'] = 'pink',
  ['Para-left'] = 'orange',
  ['Para-back'] = 'light green',
  ['Para-down'] = 'gray',
}
