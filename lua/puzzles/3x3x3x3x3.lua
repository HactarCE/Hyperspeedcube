sym = cd{4, 3, 3, 3}:chiral()

puzzles:add('3x3x3x3x3', {
  name = "3x3x3x3x3",
  ndim = 5,
  symmetry = sym, -- auto expand carve, colors, axes, twists, slice, and layers
  build = function(p)
    for _, v in sym:orbit('oooox') do
      p.shape:carve(v) -- shape
      local axis = p.axes:add(v) -- axes
      p.shape:slice(v / 3) -- cuts
      axis.layers:add(v / 3) -- layers
      print(v)
    end

    -- p.axes:rename{'I', 'B', 'D', 'L', 'R', 'U', 'F', 'O'}

    -- local I, U, R, F = p.axes.I, p.axes.U, p.axes.R, p.axes.F
    -- local transform = rot{
    --   fix = I.vector ^ U.vector,
    --   from = R,
    --   to = F,
    -- }
    -- for _, I, U, transform in sym:orbit(I, U, transform) do
    --   p.twists:add{
    --     axis = I,
    --     name = U.name,
    --     transform = transform
    --   }
    -- end
  end,
})
