puzzles:add('20cell', {
  name = "20-cell",
  ndim = 4,
  build = function(p)
    local sym = cd'a4'
    local xoox = sym.xoox.unit

    -- Build shape
    p:carve(sym:orbit(xoox))
    local t = {
      [{1, 3, 6, 10}] = 'Red Tetrad',
      [{2, 5, 9, 14}] = 'Green Tetrad',
      [{4, 8, 13, 17}] = 'Mono Tetrad',
      [{7, 12, 16, 19}] = 'Blue Tetrad',
      [{11, 15, 18, 20}] = 'Purple Tetrad',
    }
    p.colors:set_defaults(t)

    -- Define axes and slices
    p.axes:add(sym:orbit(xoox), {2/3})
    p.axes:autoname()

    -- Define twists
    local gizmo_size = 0.8
    local a1 = p.axes[xoox]
    local a2 = sym:thru(4):transform(a1)

    local t = sym:thru(3, 2)
    for _, axis1, twist_transform in sym:orbit(a1, t) do
      p.twists:add(axis1, twist_transform, {
        gizmo_pole_distance = 0.5 * gizmo_size,
      })
    end

    local edge = sym.oxxo.unit
    local t = rot{fix = a1.vector ^ edge, angle = pi}
    for _, axis1, twist_transform in sym:orbit(a1, t) do
        p.twists:add(axis1, twist_transform, {
          gizmo_pole_distance = gizmo_size,
          inverse = true,
        })
    end

    -- -- Jumbling twists?
    -- local v = a2.vector:rejected_from(a1.vector)
    -- local t = rot{fix = a1.vector ^ v, angle = math.acos(1/4)}
    -- for _, axis1, twist_transform in sym:orbit(a1, t) do
    --   p.twists:add(axis1, twist_transform, {
    --     gizmo_pole_distance = 1 * gizmo_size,
    --   })
    -- end
  end,
})
