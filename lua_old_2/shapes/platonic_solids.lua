local function platonic_solid_3d(id, schlalfi, modify_colors_fn)
  shapes:add(id, {
    ndim = 3,
    symmetry = cd(schlalfi),
    build = function(shape)
      shape:carve(cd(schlalfi):vec('oox').unit)

      if modify_colors_fn then
        modify_colors_fn(shape.colors)
      end
    end,
  })
end

platonic_solid_3d('cube', {4, 3}, function(colors)
  colors:rename{'Front', 'Up', 'Right', 'Left', 'Down', 'Back'}
  colors:reorder{'Right', 'Left', 'Up', 'Down', 'Front', 'Back'}
  colors:set_defaults{'red', 'orange', 'white', 'yellow', 'green', 'blue'}
end)

platonic_solid_3d('simplex', {3, 3})
platonic_solid_3d('octahedron', {3, 4})
platonic_solid_3d('icosahedron', {3, 5})
platonic_solid_3d('dodecahedron', {5, 3})
