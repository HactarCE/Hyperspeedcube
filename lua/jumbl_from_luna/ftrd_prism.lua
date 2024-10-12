puzzles:add({
  id = 'ftrd_prism',
  version = '0.1.0',
  name = "LunaJumble - FTRD prism",
  ndim = 4,
  tags = { 'experimental' },
  build = function(p)
    local sym = cd{4,3,2}
    local f1 = sym.oxoo.unit
    local i1 = sym.ooox.unit
    p:carve(sym:orbit(f1))
    p:carve(sym:orbit(i1))

    --p.axes:add(sym:orbit(pole):with(octahedral.AXIS_NAMES), {0})
    p.axes:add(sym:orbit(f1), {2/3})
    p.axes:add(sym:orbit(i1), {1/3})

    for _, axis, twist_transform in sym.chiral:orbit(p.axes[i1], sym:thru(2, 1)) do
      p.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end
    for _, axis, twist_transform in sym.chiral:orbit(p.axes[i1], sym:thru(3, 1)) do
      p.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end
    for _, axis, twist_transform in sym.chiral:orbit(p.axes[i1], sym:thru(3, 2)) do
      p.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    for _, axis, twist_transform in sym.chiral:orbit(p.axes[f1], sym:thru(3, 1)) do
      p.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end
    for _, axis, twist_transform in sym.chiral:orbit(p.axes[f1], sym:thru(4, 1)) do
      p.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end
    for _, axis, twist_transform in sym.chiral:orbit(p.axes[f1], sym:thru(4, 3)) do
      p.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    for _, axis, twist_transform in sym.chiral:orbit(p.axes[f1], rot{fix = f1 ^ i1, angle = math.acos(1/3)}) do
      p.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end


    -- for _, axis, twist_transform in sym.chiral:orbit(p.axes[f1], sym:thru(4, 2)) do
    --   p.twists:add(axis, twist_transform, {gizmo_pole_distance = 0.7})
    -- end
  end,
})
