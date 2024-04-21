function make_cuboid_shape(p, q, r)
  return {
    ndim = 3,
    symmetry = cd{2, 2},
    build = function(shape)
      shape:carve(vec('x') * p / 2)
      shape:carve(vec('y') * q / 2)
      shape:carve(vec('z') * r / 2)

      shape.colors:rename{'Right', 'Left', 'Up', 'Down', 'Front', 'Back'}
      shape.colors:set_defaults{'red', 'orange', 'white', 'yellow', 'green', 'blue'}
    end,
  }
end
