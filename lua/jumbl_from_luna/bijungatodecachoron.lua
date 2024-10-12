puzzles:add({
  id = 'bijungatodecachoron_ft',
  version = '0.1.0',
  name = "LunaJumble - Facet-turning Bijungatodecachoron",
  ndim = 4,
  tags = { 'experimental' },
  build = function(p)
    local sym = cd'a4'
    local f1 = sym.oxoo.unit
    local f2 = sym.ooxo.unit

    local facets = {f1,f2}
    -- Build shape
    for _,f in pairs(facets) do
      p:carve(sym:orbit(f))
    end
    -- local t = {
    --   '#ff8888', --Red
    --   '#88ff88', --Green
    --   '#dd4444', --Red
    --   '#ffffff', --Grey
    --   '#44dd44', --Green
    --   '#884444', --Red
    --   '#8888ff', --Blue
    --   '#bbbbbb', --Grey
    --   '#448844', --Green
    --   '#661111', --Red
    --   '#bb88ff', --Purple
    --   '#4444dd', --Blue
    --   '#555555', --Grey
    --   '#116611', --Green
    --   '#8822bb', --Purple
    --   '#444488', --Blue
    --   '#222222', --Grey
    --   '#441166', --Purple
    --   '#111166', --Blue
    --   '#330044', --Purple
    -- }
    -- p.colors:set_defaults(t)

    -- Define axes and slices
    for _,f in pairs(facets) do
      p.axes:add(sym:orbit(f), {4/5})
    end
    p.axes:autoname()

    -- Define twists
    local gizmo_size = 1
    local a1 = p.axes[f1]

    local t = sym:thru(4, 1)
    for _, axis1, twist_transform in sym:orbit(a1, t) do
      p.twists:add(axis1, twist_transform, {
        gizmo_pole_distance = 0.5 * gizmo_size,
      })
    end
    local t = sym:thru(4, 3)
    for _, axis1, twist_transform in sym:orbit(a1, t) do
      p.twists:add(axis1, twist_transform, {
        gizmo_pole_distance = 0.5 * gizmo_size,
      })
    end

    local a1 = p.axes[f2]

    local t = sym:thru(4, 1)
    for _, axis1, twist_transform in sym:orbit(a1, t) do
      p.twists:add(axis1, twist_transform, {
        gizmo_pole_distance = 0.5 * gizmo_size,
      })
    end
    local t = sym:thru(2, 1)
    for _, axis1, twist_transform in sym:orbit(a1, t) do
      p.twists:add(axis1, twist_transform, {
        gizmo_pole_distance = 0.5 * gizmo_size,
      })
    end
  end,
})
