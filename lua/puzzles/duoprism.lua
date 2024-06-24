local gizmo_size = 0.5
local alpha = 0.8

puzzles:add('duoprism_6_8', {
  name = "{6}x{8} Shallow-Cut Duoprism",
  ndim = 4,
  build = function(p)
    local sym = cd{6, 2, 8} -- TODO: wants to be `cd'i6 x i8'`
    local ooox = sym.ooox.unit
    local oxoo = sym.oxoo.unit

    -- Build shape
    p:carve(sym:orbit(ooox))
    p:carve(sym:orbit(oxoo))
    -- p.colors:set_defaults(hypercubic.FACE_COLORS)

    -- -- Define axes and slices
    p.axes:add(sym:orbit(ooox), {4/5})
    p.axes:add(sym:orbit(oxoo), {2/3})
    p.axes:autoname()

    local function def_twists(init_vec, mirrors)
      local m1 = mirrors[1]
      local m2 = mirrors[2]
      local m3 = mirrors[3]
      local m4 = mirrors[4]

      local a1 = p.axes[init_vec]
      local a2 = sym:thru(m4):transform(a1)
      local a3 = sym:thru(m3):transform(a2)
      local a4 = sym:thru(m2):transform(a3)
      local t = sym:thru(m2, m1)
      for _, axis1, axis2, twist_transform in sym.chiral:orbit(a1, a2, t) do
        p.twists:add(axis1, twist_transform, {
          name = axis1.name .. axis2.name,
          gizmo_pole_distance = gizmo_size,
        })
      end

      local edge = a2.vector + a3.vector -- ridge orthogonal to `a1`
      local init_transform = sym:thru(m3, m1) -- rot{fix = a1.vector ^ edge, angle = PI}
      for t, axis1, _edge, twist_transform in sym.chiral:orbit(a1, edge, init_transform) do
        p.twists:add(axis1, twist_transform, {
          name = axis1.name .. t:transform(a2).name .. t:transform(a3).name,
          gizmo_pole_distance = 2.2 * gizmo_size,
        })
      end

      local vertex = edge + a4.vector -- edge orthogonal to `a1`
      local init_transform = sym:thru(m3, m2)
      for t, axis1, _vertex, twist_transform in sym.chiral:orbit(a1, vertex, init_transform) do
        p.twists:add(axis1, twist_transform, {
          name = axis1.name .. t:transform(a2).name .. t:transform(a3).name .. t:transform(a4).name,
          gizmo_pole_distance = 2.2 * gizmo_size,
        })
      end
    end

    def_twists(ooox, {1,2,3,4})
    def_twists(oxoo, {3,4,1,2})
  end,
})
