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
