AXIS_NAMES = {
  symmetry = cd'h3',
  {'U', {}}, -- oox
  {'F', {3, 'U'}},
  {'R', {2, 'F'}},
  {'L', {1, 'R'}},
  {'BR', {2, 'L'}},
  {'BL', {1, 'BR'}},
  {'DR', {3, 'BR'}},
  {'DL', {3, 'BL'}},
  {'PR', {2, 'DL'}},
  {'PL', {1, 'PR'}},
  {'PB', {2, 'PL'}},
  {'PD', {3, 'PB'}},
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
