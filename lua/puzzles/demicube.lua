common = require('common')

puzzledef{
  id = 'demicube',
  name = "Demicube",
  ndim = 5,
  build = function()
    for v in cd{4, 3, 3, 3}:expand('oooox') do
      v = v:normalized(5/3)
      carve(v)
      add_color(v)
    end
    for _, x in ipairs{-1, 1} do
      for _, y in ipairs{-1, 1} do
        for _, z in ipairs{-1, 1} do
          for _, w in ipairs{-1, 1} do
            v = vec(x, y, z, w, x*y*z*w)
            carve(v)
            add_color(v)
          end
        end
      end
    end
  end,
}

puzzledef{
  id = 'cursed_demicube',
  name = "Cursed Face-Turning Demicube",
  ndim = 5,
  build = function()
    for v in cd{4, 3, 3, 3}:expand('oooox') do
      v = v:normalized(5/3)
      carve(v)
      add_color(v)
    end
    for _, x in ipairs{-1, 1} do
      for _, y in ipairs{-1, 1} do
        for _, z in ipairs{-1, 1} do
          for _, w in ipairs{-1, 1} do
            v = vec(x, y, z, w, x*y*z*w)
            carve(v)
            slice(v * 0.8)
            add_color(v)
          end
        end
      end
    end
  end,
}
