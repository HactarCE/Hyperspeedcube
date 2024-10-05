puzzles:add({
  id = 'unknown_64cell',
  version = '0.1.0',
  name = "LunaJumble - [Shape] Unknown 64-cell",
  ndim = 4,
  build = function(p)
    local sym = cd'bc4'
    local xoox = sym.xoox.unit

    -- Build shape
    p:carve(sym:orbit(xoox))
    -- p.colors:set_defaults(hypercubic.FACE_COLORS)

    -- -- Define axes and slices
    p.axes:add(sym:orbit(xoox), {6/7})
    p.axes:autoname()

    -- Define twists
    local gizmo_size = 0.6
    local a1 = p.axes[xoox]
    local a2 = sym:thru(4):transform(a1)
    local a3 = sym:thru(3):transform(a2)
    local a4 = sym:thru(2):transform(a3)

    local t = sym:thru(3, 2)
    for _, axis1, twist_transform in sym:orbit(a1, t) do
      p.twists:add(axis1, twist_transform, {
        gizmo_pole_distance = 0.4 * gizmo_size,
      })
    end

    local t = rot{fix = a1.vector ^ a2.vector, from = a3.vector, to = a4.vector}
    for _, axis1, twist_transform in sym:orbit(a1, t) do
      p.twists:add(axis1, twist_transform, {
        gizmo_pole_distance = 0.5 * gizmo_size,
      })
    end

    local t = rot{fix = a1.vector ^ sym:thru(2,1):transform(a1), from = a2.vector, to = a4.vector}
    for _, axis1, twist_transform in sym:orbit(a1, t) do
        p.twists:add(axis1, twist_transform, {
          gizmo_pole_distance = 0.6 * gizmo_size,
          inverse = true,
        })
    end
  end,
})
