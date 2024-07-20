print("Skipping " .. FILENAME) if true then return end

local gizmo_size = 1

puzzles:add('4_simplex', {
  ndim = 4,
  name = "4-simplex",
  build = function(self)
    local sym = cd'a4'
    self:carve(sym:orbit(sym.xooo))
    self.axes:add(sym:orbit(sym.ooox), {0})
    self.axes:autoname()

    -- Define twists
    local a1 = self.axes[sym.ooox.unit]
    local a2 = sym:thru(4):transform(a1)
    local a3 = sym:thru(3):transform(a2)
    local a4 = sym:thru(2):transform(a3)
    local t = sym:thru(2, 1)
    for _, axis1, axis2, twist_transform in sym.chiral:orbit(a1, a2, t) do
      self.twists:add(axis1, twist_transform, {
        name = axis1.name .. axis2.name,
        gizmo_pole_distance = gizmo_size,
      })
    end

    local edge = a2.vector + a3.vector -- ridge orthogonal to `a1`
    local init_transform = sym:thru(3, 1) -- rot{fix = a1.vector ^ edge, angle = PI}
    for t, axis1, _edge, twist_transform in sym.chiral:orbit(a1, edge, init_transform) do
      self.twists:add(axis1, twist_transform, {
        name = axis1.name .. t:transform(a2).name .. t:transform(a3).name,
        gizmo_pole_distance = gizmo_size*3,
      })
    end

    local vertex = edge + a4.vector -- edge orthogonal to `a1`
    local init_transform = sym:thru(3, 2)
    for t, axis1, _vertex, twist_transform in sym.chiral:orbit(a1, vertex, init_transform) do
      self.twists:add(axis1, twist_transform, {
        name = axis1.name .. t:transform(a2).name .. t:transform(a3).name .. t:transform(a4).name,
        gizmo_pole_distance = gizmo_size,
      })
    end
  end,
})
