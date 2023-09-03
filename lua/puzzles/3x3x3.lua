puzzledef{
  name = '3x3x3',
  ndim = 3,
  build = function(s)
    print("Hello from Lua!")

    local faces = {
      vec(1, 0, 0),
      vec(-1, 0, 0),
      vec(0, 1, 0),
      vec(0, -1, 0),
      vec(0, 0, 1),
      vec(0, 0, -1),
    }

    for _, v in ipairs(faces) do
      print("Carving plane %s", v)
      s:carve(plane{pole = v})
    end

    for i, v in ipairs(faces) do
      if i % 2 == 0 then
        print("Slicing plane %s", v/3)
        s:slice(plane{v, distance = 0})
      end
    end

    return s
  end,
}
