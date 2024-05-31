puzzles:add('foldy_3x3x3', {
  name = "Foldy 3x3x3",
  ndim = 3,
  build = function(p)
    p.shape:fold{4, 3}
    p.shape:carve('x')
    p.shape:unfold()
  end,
})


puzzles:add('foldy_3x3x3', {
  name = "Foldy 3x3x3",
  ndim = 3,
  build = function(p)
    local sym = cd{4, 3}

    fold(sym)
    p.shape:carve(sym:vec('oox'))
    p.shape:slice(sym:vec('oox') * 1/3)
    unfold{
      expand_colors = true,
      expand_axes = true,
    }

    local oox = sym:vec('oox').unit;

    p.shape:carve(sym, oox)
    p.shape:slice(sym, oox * 1/3)

    p.axes:add(sym:orbit{ -- ?????
      vectors = oox,
      layers = {-1/3, 1/3},
    })
    for _, ax, t in sym:orbit(p.axes[oox], sym:thru(2, 1)) do
      p.twists:add{
        axis = ax,
        transform = t,
        prefix = ax.name,
        inverse = true,
        multipliers = true,
      }
    end




    function add_facet_twists(sym)
    end


    -- for _, axis_vector, twist_transform in sym:orbit('oox', sym:thru{1, 2}) do
    --   local axis = p.axes[axis_vector]
    --   p.twists:add{
    --     axis = axis,
    --     transform = twist_transform,
    --     prefix = axis.name,
    --     inverse = true,
    --     multipliers = true,
    --   }
    -- end

    -- for _, v in sym:orbit('oox') do
    --   p.shape:carve(v.unit)
    --   p.shape:slice(v.unit * 1/3)
    --   p.axes:add()
    -- end

    -- p.shape:carve(cd{4, 3}:orbit('oox'))

    -- for t, v in cd{4, 3}:orbit('oox') do
    --   p.shape:carve(v)
    -- end

    -- cd{4, 3}:exec(function(sym)
    --   p.shape:carve('x')
    -- end)
  end,
})
