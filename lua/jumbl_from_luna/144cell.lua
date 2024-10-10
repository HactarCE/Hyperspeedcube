puzzles:add({
  id = '144cell',
  version = '0.1.0',
  name = "LunaJumble - 144-cell",
  tags = { 'big' },
  ndim = 4,
  build = function(p)
    local sym = cd'f4'
    local xoox = sym.xoox.unit

    -- Build shape
    p:carve(sym:orbit(xoox))
    local t = {
      '#ff8888', --Red
      '#88ff88', --Green
      '#dd4444', --Red
      '#ffffff', --Grey
      '#44dd44', --Green
      '#884444', --Red
      '#8888ff', --Blue
      '#bbbbbb', --Grey
      '#448844', --Green
      '#661111', --Red
      '#bb88ff', --Purple
      '#4444dd', --Blue
      '#555555', --Grey
      '#116611', --Green
      '#8822bb', --Purple
      '#444488', --Blue
      '#222222', --Grey
      '#441166', --Purple
      '#111166', --Blue
      '#330044', --Purple
    }
    p.colors:set_defaults(t)

    -- Define axes and slices
    p.axes:add(sym:orbit(xoox), {15/16})
    p.axes:autoname()

    -- Define twists
    local gizmo_size = 0.4
    local a1 = p.axes[xoox]
    local a2 = sym:thru(4):transform(a1)
    local a3 = sym:thru(3):transform(a2)
    local a4 = sym:thru(2):transform(a3)

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

    -- Jamble?
    local t = rot{fix = a1.vector ^ a2.vector, from=a3.vector, to=a4.vector}
    for _, axis1, twist_transform in sym:orbit(a1, t) do
      p.twists:add(axis1, twist_transform, {
        gizmo_pole_distance = 1 * gizmo_size,
      })
    end
  end,
})
