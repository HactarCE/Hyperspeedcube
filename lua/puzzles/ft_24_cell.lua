local gizmo_size = 0.75
local alpha = 0.3

puzzles:add{
  id = 'ft_24_cell',
  name = "Facet-Turning 24-cell",
  version = '0.1.0',
  ndim = 4,
  build = function(p)
    local sym = cd'f4'
    local ooox = sym.ooox.unit

    -- Build shape
    p:carve(sym:orbit(ooox))
    local t = {
      'Mono Triad [1]',
      'Mono Triad [2]',
      'Mono Triad [3]',
      'Red Triad [1]',
      'Red Triad [2]',
      'Red Triad [3]',
      'Orange Triad [1]',
      'Orange Triad [2]',
      'Orange Triad [3]',
      'Yellow Triad [1]',
      'Yellow Triad [2]',
      'Yellow Triad [3]',
      'Green Triad [1]',
      'Green Triad [2]',
      'Green Triad [3]',
      'Blue Triad [1]',
      'Blue Triad [2]',
      'Blue Triad [3]',
      'Purple Triad [1]',
      'Purple Triad [2]',
      'Purple Triad [3]',
      'Magenta Triad [1]',
      'Magenta Triad [2]',
      'Magenta Triad [3]',
    }
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
        name = axis2.name,
        gizmo_pole_distance = (1+2*alpha)/sqrt(3) * gizmo_size,
      })
    end

    local ridge = a2.vector + a3.vector -- ridge orthogonal to `a1`
    local init_transform = sym:thru(3, 1) -- rot{fix = a1.vector ^ ridge, angle = PI}
    for t, axis1, _ridge, twist_transform in sym.chiral:orbit(a1, ridge, init_transform) do
      p.twists:add(axis1, twist_transform, {
        name = t:transform(a2).name .. t:transform(a3).name,
        gizmo_pole_distance = (1+alpha)/sqrt(2) * gizmo_size,
      })
    end

    local edge = ridge + a4.vector -- edge orthogonal to `a1`
    local init_transform = sym:thru(3, 2)
    for t, axis1, _edge, twist_transform in sym.chiral:orbit(a1, edge, init_transform) do
      p.twists:add(axis1, twist_transform, {
        name = t:transform(a2).name .. t:transform(a3).name .. t:transform(a4).name,
        gizmo_pole_distance = gizmo_size,
      })
    end
  end,
}
