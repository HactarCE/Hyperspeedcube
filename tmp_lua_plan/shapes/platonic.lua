function platonic_solid(name, symbol)
  shapedef{
    name = name,
    ndim = 3,
    build = function(s)
      local poles = cd(symbol):expand(vec(0, 0, 1))
      for _, v in ipairs(poles) do
        s = s & plane{pole = v}.inside
      end
      return s
    end,
  }
end

platonic_solid('tetrahedron', {3, 3})
platonic_solid('octahedron', {3, 4})
platonic_solid('icosahedron', {3, 5})
platonic_solid('cube', {4, 3})
platonic_solid('dodecahedron', {5, 3})
