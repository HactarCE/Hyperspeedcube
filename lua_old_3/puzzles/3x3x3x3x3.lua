local hypercubic = require('symmetries/hypercubic')

local gizmo_size = 1.2
local alpha = 0.8

puzzles:add('3x3x3x3x3', {
  name = "3x3x3x3x3",
  ndim = 5,
  build = function(p)
    local sym = cd'bc5'
    local oooox = sym.oooox.unit

    -- Build shape
    p:carve(sym:orbit(oooox))
    -- p.colors:set_defaults(hypercubic.FACE_COLORS)

    -- Define axes and slices
    p.axes:add(sym:orbit(oooox), {1/3, -1/3})

    -- -- Define twists
    -- local a1 = p.axes[oooox]
    -- local a2 = sym:thru(4):transform(a1)
    -- local a3 = sym:thru(3):transform(a2)
    -- local a4 = sym:thru(2):transform(a3)
    -- local t = sym:thru(2, 1)
    -- for _, axis1, axis2, twist_transform in sym.chiral:orbit(a1, a2, t) do
    --   p.twists:add(axis1, twist_transform, {
    --     name = axis1.name .. axis2.name,
    --     gizmo_pole_distance = gizmo_size,
    --   })
    -- end

    -- local edge = a2.vector + a3.vector -- ridge orthogonal to `a1`
    -- local init_transform = sym:thru(3, 1) -- rot{fix = a1.vector ^ edge, angle = PI}
    -- for t, axis1, _edge, twist_transform in sym.chiral:orbit(a1, edge, init_transform) do
    --   p.twists:add(axis1, twist_transform, {
    --     name = axis1.name .. t:transform(a2).name .. t:transform(a3).name,
    --     gizmo_pole_distance = (1+alpha)/sqrt(2) * gizmo_size,
    --   })
    -- end

    -- local vertex = edge + a4.vector -- edge orthogonal to `a1`
    -- local init_transform = sym:thru(3, 2)
    -- for t, axis1, _vertex, twist_transform in sym.chiral:orbit(a1, vertex, init_transform) do
    --   p.twists:add(axis1, twist_transform, {
    --     name = axis1.name .. t:transform(a2).name .. t:transform(a3).name .. t:transform(a4).name,
    --     gizmo_pole_distance = (1+2*alpha)/sqrt(3) * gizmo_size,
    --   })
    -- end
  end,
})
