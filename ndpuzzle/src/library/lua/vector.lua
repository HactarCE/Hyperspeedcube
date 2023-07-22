Vector = {}

AXES = {
   'X',   'Y',   'Z',   'W',   'U',   'V',   'R',   'S',
  x = 1, y = 2, z = 3, w = 4, u = 5, v = 6, r = 7, s = 8,
  X = 1, Y = 2, Z = 3, W = 4, U = 5, V = 6, R = 7, S = 8,
}

-----------------------
-- Utility functions --
-----------------------

local function to_vector_index(key)
  if type(key) == 'number' and isinteger(key) and key > 0 then
    return key
  elseif AXES[key] then
    return AXES[key]
  else
    error('expected vector index (positive integer or axis name); got ' .. tostring(key))
  end
end

local function assert_valid_vector_ndim(i)
  assert(isinteger(i) and i >= 0, 'vector index must be a nonnegative integer')
end

local function assert_vectors_for_binop(u, v)
  assert(type(u) == 'vector' and type(v) == 'vector', 'cannot add ' .. type(u) .. ' and ' .. type(v))
end

local function set_vector_mag2(v, new_mag2)
  for i = 1, #v do
    v[i] = v[i] * (new_mag2 / v.mag2)
  end
end

local function extend_vector(v, new_ndim)
  for i = #v + 1, new_ndim do
    rawset(v, i, 0)
  end
end
local function truncate_vector(v, new_ndim)
  for i = new_ndim + 1, #v do
    rawset(v, i, nil)
  end
end

-------------
-- Methods --
-------------

function Vector:mag2()
  assert(type(self) == 'vector')
  sum = 0
  for _, self in ipairs(self) do
    sum = sum + self * self
  end
  return sum
end
function Vector:mag()
  return math.sqrt(Vector:mag2(self))
end

function Vector:normalized(new_len)
  assert(type(self) == 'vector')
  if new_len == nil then
    return self / self:mag()
  else
    assert(type(new_len) == 'number')
    return self * (new_len / self:mag())
  end
end

function Vector:ndim()
  assert(type(self) == 'vector')
  return #self
end

function Vector:set_ndim(new_ndim)
  assert(type(self) == 'vector')
  assert_valid_vector_ndim(new_ndim)

  extend_vector(self, new_ndim)
  truncate_vector(self, new_ndim)
end
function Vector:at_ndim(new_ndim)
  assert(type(self) == 'vector')
  assert_valid_vector_ndim(new_ndim)

  local result = Vector:new()
  for i = 1, new_ndim do
    rawset(result, i, self[i])
  end
  return result
end

---------------
-- Metatable --
---------------

local vector_metatable = {
  type = 'vector',

  __add = function(u, v)
    assert(type(u) ~= 'number' and type(v) ~= 'number', 'cannot add vector and number')
    if type(u) == 'multivector' or type(v) == 'multivector' then
      return mvec(u) + mvec(v)
    end
    return vec(table.map(function(a, b) return a + b end, vec(u), vec(v)))
  end,
  __sub = function(u, v)
    assert(type(u) ~= 'number' and type(v) ~= 'number', 'cannot add vector and number')
    if type(u) == 'multivector' or type(v) == 'multivector' then
      return mvec(u) - mvec(v)
    end
    return vec(table.map(function(a, b) return a - b end, vec(u), vec(v)))
  end,
  __mul = function(v, a)
    if type(v) ~= 'vector' and type(a) == 'vector' then
      return a * v -- swap arguments
    end
    assert(type(a) == 'number')
    return vec(table.map(function(x) return x * a end, vec(v)))
  end,
  __div = function(v, a)
    assert(type(a) == 'number')
    return v * (1 / a)
  end,
  __unm = function(v)
    return vec(table.map(function(x) return -x end, vec(v)))
  end,

  __pow = function(u, v)
    return mvec(u) ^ mvec(v)
  end,
  __band = function(u, v)
    return mvec(u) & mvec(v)
  end,
  __bor = function(u, v)
    return mvec(u) | mvec(v)
  end,
  __shl = function(u, v)
    return mvec(u) << mvec(v)
  end,
  __shr = function(u, v)
    return mvec(u) >> mvec(v)
  end,

  __eq = function(u, v)
    for i = 1, math.max(#u, #v) do
      if not approx_eq(u[i], v[i]) then
        return false
      end
    end
    return true
  end,

  __index = function(v, key)
    return Vector[key] or rawget(v, to_vector_index(key)) or 0
  end,
  __newindex = function(v, key, value)
    assert(type(value) == 'number', 'vector component must be number; got ' .. tostring(value))

    extend_vector(v, key)
    rawset(v, to_vector_index(key), value)
  end,

  __tostring = function(v)
    local a = table.shallowcopy(v)
    setmetatable(a, nil)
    return '[' .. string.join(v) .. ']'
  end
}

-----------------
-- Constructor --
-----------------

function vec(...)
  local args = {...}
  local first_arg = args[1]
  if type(first_arg) == 'vector' then
    return first_arg
  end

  if type(first_arg) == 'table' then
    assert(#args == 1, 'when first argument to vec() is table, no other arguments are allowed')
    args = first_arg
  end

  result = {}
  setmetatable(result, vector_metatable)
  for k, v in pairs(args) do
    result[k] = v
  end
  return result
end

-----------
-- Tests --
-----------

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
  assert(tostring(v) == '[3, 4, 0]')
  assert(vec(3, 0, 4) ~= vec(3, 4))

  -- Test constructing vector from list
  local v = vec {10, 20, 30}
  assert(tostring(v) == '[10, 20, 30]')
  assert(#v == 3)

  -- Test empty vector
  local v = vec()
  assert(tostring(v) == '[]')
  assert(#v == 0)
  local v = vec {}
  assert(tostring(v) == '[]')
  assert(#v == 0)

  -- Test addition
  assert(tostring(vec(1, 2) + vec(10, 20)) == '[11, 22]')
  assert(tostring(vec(1, 2, 0) + vec(10, 20)) == '[11, 22, 0]')
  assert(tostring(vec(1, 2) + vec(10, 20, 0)) == '[11, 22, 0]')
  assert(tostring(vec(1, 0, 2) + vec(10, 20)) == '[11, 20, 2]')
  assert(tostring(vec(1, 2) + vec(10, 0, 20)) == '[11, 2, 20]')

  -- Test subtraction
  assert(tostring(vec(1, 2) - vec(10, 20)) == '[-9, -18]')
  assert(tostring(vec(1, 2, 0) - vec(10, 20)) == '[-9, -18, 0]')
  assert(tostring(vec(1, 2) - vec(10, 20, 0)) == '[-9, -18, 0]')
  assert(tostring(vec(1, 0, 2) - vec(10, 20)) == '[-9, -20, 2]')
  assert(tostring(vec(1, 2) - vec(10, 0, 20)) == '[-9, 2, -20]')

  -- Test scaling
  assert(tostring(vec(1, 2, 3) * 10) == '[10, 20, 30]')
  assert(vec(10, 20, 30) / 10 == vec(1, 2, 3))
end
