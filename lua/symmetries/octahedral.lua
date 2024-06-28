FACE_NAMES = {
  symmetry = 'bc3',
  {{3, "D" }, "R", "Right"},
  {{1, "F" }, "L", "Left"},
  {{3, "BL"}, "U", "Up"},
  {{2, "L" }, "D", "Down"},
  {{       }, "F", "Front"}, -- xoo
  {{2, "U" }, "BR", "Back-right"},
  {{1, "D" }, "BL", "Back-left"},
  {{1, "BR"}, "BD", "Back-down"}, -- B in standard notation
}

OCTAHEDRON_COLOR_SCHEMES = {
  ["Lanlan"] = {
    R = "Green",
    L = "Purple",
    U = "Mono Dyad [1]",
    D = "Yellow",
    F = "Red",
    BR = "Mono Dyad [2]",
    BL = "Orange",
    BD = "Blue",
  },
  ["Benpuzzles Classic"] = {
    R = "Yellow",
    L = "Cyan",
    U = "Mono Dyad [1]",
    D = "Mono Dyad [2]",
    F = "Green",
    BR = "Red",
    BL = "Blue",
    BD = "Magenta",
  },
  ["Benpuzzles Alt"] = {
    R = "Red",
    L = "Yellow",
    U = "Mono Dyad [1]",
    D = "Mono Dyad [2]",
    F = "Green",
    BR = "Purple",
    BL = "Orange",
    BD = "Blue", -- lighter
  },
  ["Diansheng"] = {
    R = "Red",
    L = "Purple",
    U = "Mono Dyad [1]",
    D = "Yellow",
    F = "Green",
    BR = "Mono Dyad [2]",
    BL = "Orange",
    BD = "Blue",
  },
  ["MF8"] = {
    R = "Red",
    L = "Pink",
    U = "White",
    D = "Yellow",
    F = "Green",
    BR = "Purple",
    BL = "Orange",
    BD = "Blue", -- lighter
  },
}

OCTAHEDRON_COLORS = OCTAHEDRON_COLOR_SCHEMES["Diansheng"]

function octahedron()
  local shape = setmetatable({}, meta.shape)
  shape.sym = cd'bc3'
  shape.pole = shape.xoo.unit

  function shape:carve_into(p)
    p:carve(self.sym:orbit(self.oox.unit):with(FACE_NAMES))
    for _, name in ipairs{"Lanlan", "Benpuzzles Classic", "Benpuzzles Alt", "Diansheng", "MF8"} do
      p.colors:add_scheme(name, OCTAHEDRON_COLOR_SCHEMES[name])
    end
    p.colors:set_defaults("Diansheng")
  end

  return shape
end
