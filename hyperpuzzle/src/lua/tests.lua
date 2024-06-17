function test_vector_construct_6d()
  local v = vec(3, 4, 5)
  assert(v.x == 3)
  assert(v.y == 4)
  assert(v.z == 5)
  assert(v.w == 0)
  assert(v.u == 0)
  assert(v.v == 0)
end

function test_vector_ops_3d()
  local v = vec(3, 4, 5)
  assert(v.x == 3)
  assert(v.y == 4)
  assert(v.z == 5)
  assert(v.w == nil)
  assert(v.u == nil)
  assert(v.v == nil)

  local v = vec{3, 4, 0}
  assert(v == v)
  assert(v == vec(3, 4))
  assert(vec(3, 4) == v)
  assert(tostring(v) == 'vec(3, 4, 0)')
  assert(vec(3, 0, 4) ~= vec(3, 4))

  -- Test constructing vector from list
  local v = vec{10, 20, 30}
  assert(tostring(v) == 'vec(10, 20, 30)')

  -- Test empty vector
  local v = vec()
  assert(tostring(v) == 'vec(0, 0, 0)')
  local v = vec{}
  assert(tostring(v) == 'vec(0, 0, 0)')

  -- Test addition
  assert(tostring(vec(1, 2) + vec(10, 20)) == 'vec(11, 22, 0)')
  assert(tostring(vec(1, 2, 0) + vec(10, 20)) == 'vec(11, 22, 0)')
  assert(tostring(vec(1, 2) + vec(10, 20, 0)) == 'vec(11, 22, 0)')
  assert(tostring(vec(1, 0, 2) + vec(10, 20)) == 'vec(11, 20, 2)')
  assert(tostring(vec(1, 2) + vec(10, 0, 20)) == 'vec(11, 2, 20)')

  -- Test subtraction
  assert(tostring(vec(1, 2) - vec(10, 20)) == 'vec(-9, -18, 0)')
  assert(tostring(vec(1, 2, 0) - vec(10, 20)) == 'vec(-9, -18, 0)')
  assert(tostring(vec(1, 2) - vec(10, 20, 0)) == 'vec(-9, -18, 0)')
  assert(tostring(vec(1, 0, 2) - vec(10, 20)) == 'vec(-9, -20, 2)')
  assert(tostring(vec(1, 2) - vec(10, 0, 20)) == 'vec(-9, 2, -20)')

  -- Test scaling
  assert(tostring(vec(1, 2, 3) * 10) == 'vec(10, 20, 30)')
  assert(tostring(10 * vec(1, 2, 3)) == 'vec(10, 20, 30)')
  assert(vec(10, 20, 30) / 10 == vec(1, 2, 3))

  -- Test dot product
  assert(vec(1, 2, 3):dot(vec(4, -3, 0, 1)) == -2)

  -- Test cross product
  assert(tostring(vec(1, 2, 3):cross(vec(4, -3, 0, 1))) == 'vec(9, 12, -11)')

  -- Test projection
  assert(tostring(vec(1, 2, 3):projected_to('y')) == 'vec(0, 2, 0)')

  -- Test rejection
  assert(tostring(vec(1, 2, 3):rejected_from('y')) == 'vec(1, 0, 3)')

  -- -- Test constructing a multivector (CGA)
  -- assert(tostring(mvec{xy = 3, s = 2, z = -1}) == '2 + 3xy + -1z')
  -- assert(tostring(vec(mvec{xy = 3, s = 2, z = -1})) == 'vec(0, 0, -1)')
end

function test_blade_iteration_3d()
  local j = 0
  for i, x in pairs(vec(0, 20)) do
    j = j + 1
    if j == 1 then assert(i == 1 and x == 0) end
    if j == 2 then assert(i == 2 and x == 20) end
    if j == 3 then assert(i == 3 and x == 0) end
  end
  assert(j == 3)

  local j = 0
  for i, x in pairs(point(0, 20) * 6) do
    j = j + 1
    if j == 1 then assert(i == 1 and x == 0) end
    if j == 2 then assert(i == 2 and x == 20) end
    if j == 3 then assert(i == 3 and x == 0) end
  end
  assert(j == 3)

  local is_success, err = pcall(pairs, plane('x'))
  assert(not is_success)
end

function test_plane_construction_3d()
  local expected

  -- plane through point (2, -3, 6) with normal vector (2/7, -3/7, 6/7)
  expected = plane{normal = vec(2/7, -3/7, 6/7), point = vec(2, -3, 6)}
  assert(expected == plane(vec(2, -3, 6)))
  assert(expected == plane(point(2, -3, 6)))

  -- plane through point (1, 0, 0) perpendicular to the X axis
  expected = plane{normal = vec(1, 0, 0), point = vec(1, 0, 0)}
  assert(expected == plane('x'))
  assert(expected == plane{pole = vec(1)})
  assert(expected == plane{normal = vec(3), distance = 1})

  -- plane through point (1, 0, 0) perpendicular to the X axis, but facing the other way
  expected = plane{normal = -vec(1, 0, 0), point = vec(1, 0, 0)}
  assert(expected == -plane('x'))
  assert(expected == plane(-vec('x'), -1))

  -- plane through the origin with normal vector (0, 0, 1)
  expected = plane{normal = vec(0, 0, 1), point = vec(0, 0, 0)}
  assert(expected == plane('z', 0))
  assert(expected == plane{normal = 'z', point = vec()})
  assert(expected == plane{normal = 'z', distance = 0})
end

function test_bc3_mirror_vectors_5d()
  local mirrors = cd'bc3'.mirror_vectors
  assert(#mirrors == 3)
  assert(mirrors[1] == vec(1))
  assert(mirrors[2] == vec(-1, 1) / sqrt(2))
  assert(mirrors[3] == vec(0, -1, 1) / sqrt(2))
end

function test_symmetry_thru_3d()
  local sym = cd'bc3'
  local a = sym:thru(1, 2, 3)
  local b = sym:thru(1) * sym:thru(2) * sym:thru(3)
  assert(a == b)
end

function test_get_ndim_3d()
  assert(3, vec(1).ndim)
  assert(3, cd'bc2'.ndim)
  assert(3, cd'bc2':thru(1).ndim)
end
