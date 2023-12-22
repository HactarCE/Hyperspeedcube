common = require('common')

function duoprism_5d(id, name, a, b)
  puzzledef{
    id = id,
    name = name,
    ndim = 5,
    build = function()
      for i in cd(a):expand('oox') do
        local v = vec(i.x, i.y, i.z)
        carve(v)
        add_color(v)
        slice(v / 2)
      end
      for j in cd(b):expand('ox') do
        local v = vec(0, 0, 0, j.x, j.y)
        carve(v)
        add_color(v)
        slice(v / 2)
      end
    end,
  }
end

duoprism_5d(
  'tetrahedron_square_duoprism',
  "Tetrahedron-Square Duoprism",
  {3, 3}, {4}
)
