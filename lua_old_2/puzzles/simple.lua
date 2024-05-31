sym = cd{4, 3}

puzzles:add('default', {
  name = "default",

  ndim = 3,

  build = function(p)
    for _, v in sym:orbit('oox') do
      p.shape:carve(v)
    end
  end,
})
