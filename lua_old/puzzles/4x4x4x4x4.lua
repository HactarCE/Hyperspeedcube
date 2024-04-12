common = require('common')

puzzledef{
  id = '4x4x4x4x4',
  name = "4^5",
  ndim = 5,

  build = function()
    for v in cd{4, 3, 3, 3}:expand('oooox') do
      local v = v:normalized(1)
      carve(v)
      slice{normal = v, distance = 0}
      slice{normal = v, distance = 1/2}
      add_color(v)
    end
  end,
}
