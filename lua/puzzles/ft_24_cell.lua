local gizmo_size = 0.6

puzzles:add('ft_24_cell', {
  name = "Face-Turning 24-cell",
  ndim = 4,
  build = function(p)
    local sym = cd'f4'
    local ooox = sym.ooox.unit

    -- Build shape
    p:carve(sym:orbit(ooox))
    local t = {}
    for i = 1,24 do
      table.insert(t, 'c' .. i)
    end
    p.colors:set_defaults(t)

    -- Define axes and slices
    p.axes:add(sym:orbit(ooox), {2/3})
    p.axes:autoname()

    -- Define twists
    local a1 = p.axes[ooox]
    local a2 = sym:thru(4):transform(a1)
    local a3 = sym:thru(3):transform(a2)
    local a4 = sym:thru(2):transform(a3)
    local t = sym:thru(2, 1)
    for _, axis1, axis2, twist_transform in sym.chiral:orbit(a1, a2, t) do
      p.twists:add(axis1, twist_transform, {
        name = axis1.name .. axis2.name,
        gizmo_pole_distance = gizmo_size,
      })
    end

    local edge = a2.vector + a3.vector -- ridge orthogonal to `a1`
    local init_transform = sym:thru(3, 1) -- rot{fix = a1.vector ^ edge, angle = PI}
    for t, axis1, _edge, twist_transform in sym.chiral:orbit(a1, edge, init_transform) do
      p.twists:add(axis1, twist_transform, {
        name = axis1.name .. t:transform(a2).name .. t:transform(a3).name,
        gizmo_pole_distance = 1.1 * gizmo_size,
      })
    end

    local vertex = edge + a4.vector -- edge orthogonal to `a1`
    local init_transform = sym:thru(3, 2)
    for t, axis1, _vertex, twist_transform in sym.chiral:orbit(a1, vertex, init_transform) do
      p.twists:add(axis1, twist_transform, {
        name = axis1.name .. t:transform(a2).name .. t:transform(a3).name .. t:transform(a4).name,
        gizmo_pole_distance = 1.4 * gizmo_size,
      })
    end
  end,
})
