function test_vector_ops()
  local v = vec(3, 4, 5)
  assert(v.x == 3)
  assert(v.y == 4)
  assert(v.z == 5)
  assert(v.w == 0)
  assert(v.u == 0)
  assert(v.v == 0)

  local v = vec(3, 4, 0)
  assert(v == v)
  assert(v == vec(3, 4))
  assert(vec(3, 4) == v)
  assert(#v == 3)
  assert(tostring(v) == '(3, 4, 0)')
  assert(vec(3, 0, 4) ~= vec(3, 4))

  -- Test constructing vector from list
  local v = vec{10, 20, 30}
  assert(tostring(v) == '(10, 20, 30)')
  assert(#v == 3)

  -- Test empty vector
  local v = vec()
  assert(tostring(v) == '()')
  assert(#v == 0)
  local v = vec{}
  assert(tostring(v) == '()')
  assert(#v == 0)

  -- Test addition
  assert(tostring(vec(1, 2) + vec(10, 20)) == '(11, 22)')
  assert(tostring(vec(1, 2, 0) + vec(10, 20)) == '(11, 22, 0)')
  assert(tostring(vec(1, 2) + vec(10, 20, 0)) == '(11, 22, 0)')
  assert(tostring(vec(1, 0, 2) + vec(10, 20)) == '(11, 20, 2)')
  assert(tostring(vec(1, 2) + vec(10, 0, 20)) == '(11, 2, 20)')

  -- Test subtraction
  assert(tostring(vec(1, 2) - vec(10, 20)) == '(-9, -18)')
  assert(tostring(vec(1, 2, 0) - vec(10, 20)) == '(-9, -18, 0)')
  assert(tostring(vec(1, 2) - vec(10, 20, 0)) == '(-9, -18, 0)')
  assert(tostring(vec(1, 0, 2) - vec(10, 20)) == '(-9, -20, 2)')
  assert(tostring(vec(1, 2) - vec(10, 0, 20)) == '(-9, 2, -20)')

  -- Test scaling
  assert(tostring(vec(1, 2, 3) * 10) == '(10, 20, 30)')
  assert(tostring(10 * vec(1, 2, 3)) == '(10, 20, 30)')
  assert(vec(10, 20, 30) / 10 == vec(1, 2, 3))

  -- Test dot product
  assert(vec(1, 2, 3):dot{4, -3, 0, 1} == -2)

  -- Test cross product
  assert(tostring(vec(1, 2, 3):cross(vec(4, -3, 0, 1))) == '(9, 12, -11)')

  -- Test projection
  assert(tostring(vec(1, 2, 3):projected_to('y')) == '(0, 2)')

  -- Test rejection
  assert(tostring(vec(1, 2, 3):rejected_from('y')) == '(1, 0, 3)')

  -- Test constructing a multivector
  assert(tostring(mvec{xy = 3, s = 2, z = -1}) == '2 + 3xy + -1z')
  assert(tostring(vec(mvec{xy = 3, s = 2, z = -1})) == '(0, 0, -1)')
end
