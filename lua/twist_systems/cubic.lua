local sym = cd{4, 3}

axis_systems:add('cubic', {
  ndim = 3,
  symmetry = sym,
  build = function(axes)
    axes:add(sym:vec('oox'))
    axes:rename{'F', 'U', 'R', 'L', 'D', 'B'}
    axes:reorder{'R', 'L', 'U', 'D', 'F', 'B'}
  end,
})

twist_systems:add('ft_cubic', {
  ndim = 3,
  axes = 'cubic',
  symmetry = sym,
  build = function(twists)
    local R = twists.axes.R
    local U = twists.axes.U
    local F = twists.axes.F
    local twist_rot = rot{fix = U, from = R, to = F}
    for transform, axis, twist_rot in sym:chiral():orbit(U, twist_rot) do
      twists:add({
        axis_prefix = true, -- this is default
        name = "", -- this is default
        inv_name = "'", -- this is default (not yet implemented)
        inverse = true, -- this is default (not yet implemented)
        axis = axis,
        transform = twist_rot,
      })
    end

    -- Twist directions are not implemented yet.

    -- twists.directions:add("CW", {twist = function(ax) return ax.name end})
    -- twists.directions:add("CCW", {twist = function(ax) return ax.name .. "'" end})
    -- twists.directions:add("180 CW", {twist = function(ax) return {ax.name, ax.name} end})
    -- twists.directions:add("180 CCW", {twist = function(ax) return {ax.name .. "'", ax.name .. "'"} end})
  end,
})
