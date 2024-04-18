sym = cd{5, 3}
cut_depth = 2/3

puzzles:add('megaminx', {
  name = "Megaminx",
  ndim = 3,
  shape = 'dodecahedron',
  symmetry = sym, -- auto expand carve, colors, axes, twists, slice, and layers
  build = function(p)
    -- axes
    for _, v in sym:orbit('oox') do
      p.twists.axes:add(v)
    end
    p.twists.axes:autoname()

    -- -- print axes so we know which is which
    -- for i, ax in ipairs(p.twists.axes) do
    --   print(i, ax.vector)
    -- end

    -- twists
    local R = p.twists.axes[1]
    local U = p.twists.axes[3]
    local F = p.twists.axes[2]
    local twist_rot = rot{fix = U, from = R, to = F}
    for transform, axis, twist_rot in sym:chiral():orbit(U, twist_rot) do
      p.twists:add({
        axis_prefix = true, -- this is default
        name = "", -- this is default
        inv_name = "'", -- this is default (not yet implemented)
        inverse = true, -- this is default (not yet implemented)
        axis = axis,
        transform = twist_rot,
      })
    end

    -- slicing & layers
    p.shape:slice(sym:vec('oox'):normalized() * cut_depth)

    for _, ax in ipairs(p.twists.axes) do
      ax.layers:add(ax.vector:normalized() * cut_depth)
    end
  end,

})