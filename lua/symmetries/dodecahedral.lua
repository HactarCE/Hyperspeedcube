FACE_NAMES = {
  symmetry = 'h3',
  [{       }] = {"U", "Up"}, -- oox
  [{3, "U" }] = {"F", "Front"},
  [{2, "F" }] = {"R", "Right"},
  [{1, "R" }] = {"L", "Left"},
  [{2, "L" }] = {"BR", "Back-right"},
  [{1, "BR"}] = {"BL", "Back-left"},
  [{3, "BR"}] = {"DR", "Down-right"},
  [{3, "BL"}] = {"DL", "Down-left"},
  [{2, "DL"}] = {"PR", "Para-right"},
  [{1, "PR"}] = {"PL", "Para-left"},
  [{2, "PL"}] = {"PB", "Para-back"},
  [{3, "PB"}] = {"PD", "Para-down"},
}

DODECAHEDRON_COLORS = {
  U = 'White',
  F = 'Green',
  R = 'Red',
  L = 'Purple',
  BR = 'Blue',
  BL = 'Yellow',
  DR = 'Light Yellow',
  DL = 'Light Blue',
  PR = 'Pink',
  PL = 'Orange',
  PB = 'Light Green',
  PD = 'Gray',
}

function dodecahedron()
  local shape = setmetatable({}, meta.shape)
  shape.sym = cd'h3'
  shape.pole = shape.oox.unit
  shape.colors = shape.colors

  function shape:carve_into(p)
    p:carve(shape.sym:orbit(shape.pole):with(FACE_NAMES))
    p.colors:set_defaults(FACE_COLORS)
  end

  return setmetatable(shape, meta.shape)
end
