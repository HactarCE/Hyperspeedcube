common = require('common')

puzzledef{
  id = 'hmt',
  name = "Halpern-Meier Tetrahedron",
  aliases = {
    "Jing's Pyraminx",
  },
  ndim = 3,

  build = function()
    common.carve_and_slice_face_turning({3, 3}, 0.5)
    common.colors.tetrahedron()

    -- define_facets(common.facets.dodecahedron())
    -- define_axes(common.axes.dodecahedral{D, -D}) -- TODO: check cut depth
    -- define_twists(common.symmetric_twists_3d({5, 3}, 'F', 'U', 'R'))
    -- define_twist_directions(common.twist_directions_2d(5))
  end,
}
