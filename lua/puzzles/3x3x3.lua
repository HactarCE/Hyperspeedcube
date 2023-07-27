puzzledef{
  name = '3x3x3',
  ndim = 3,
  build = function(s)
    print("Hello from Lua!")
    for _, v in ipairs{
      vec(1, 0, 0),
      vec(-1, 0, 0),
      vec(0, 1, 0),
      vec(0, -1, 0),
      vec(0, 0, 1),
      vec(0, 0, -1),
    } do
      print("Slicing plane %s", v)
      s:carve(plane{pole = v})
    end
    return s
  end,
}
