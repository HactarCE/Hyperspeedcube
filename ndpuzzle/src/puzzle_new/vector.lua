require('util')

Vector = {}



-----------------------
-- Utility functions --
-----------------------

local function to_vector_index(key)
    if type(key) == 'number' and util.is_integer(key) and key > 0 then
        return key
    elseif AXES[key] then
        return AXES[key]
    else
        error('expected vector index (positive integer or axis name); got ' .. tostring(key))
    end
end

local function assert_valid_vector_ndim(i)
    assert(util.is_integer(i) and i >= 0, 'vector index must be a nonnegative integer')
end

local function assert_vectors_for_binop(u, v)
    assert(
        type(u) == 'vector' and type(v) == 'vector',
        'cannot add ' .. type(u) .. ' and ' .. type(v),
    )
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



---------------
-- Metatable --
---------------

local vector_metatable = {
    type = 'vector',

    __add = function(u, v)
        if type(u) == 'multivector' or type(v) == 'multivector' then
            return mvec(u) + mvec(v)
        end
        return vec(util.map(function(a, b) return a + b end, vec(u), vec(v)))
    end,
    __sub = function(u, v)
        if type(u) == 'multivector' or type(v) == 'multivector' then
            return mvec(u) - mvec(v)
        end
        return vec(util.map(function(a, b) return a - b end, vec(u), vec(v)))
    end,
    __mul = function(v, a)
        if type(v) != 'vector' and type(a) == 'vector' then
            return a * v -- swap arguments
        end
        assert(type(a) == 'number')
        return vec(util.map(function(x) return x * a end, vec(v)))
    end,
    __div = function(v, a)
        assert(type(a) == 'number')
        return v * (1 / a)
    end,
    __unm = function(v)
        return vec(util.map(function(x) return -x end, vec(v)))
    end,

    __pow =  function(u, v) mvec(u) ^ mvec(v) end,
    __band = function(u, v) mvec(u) & mvec(v) end,
    __bor =  function(u, v) mvec(u) | mvec(v) end,
    __shl =  function(u, v) mvec(u) << mvec(v) end,
    __shr =  function(u, v) mvec(u) >> mvec(v) end,

    __eq = function(u, v)
        for i in 1, math.max(#u, #v) do
            if not approx_eq(u[i], v[i]) then
                return false
            end
        end
        return true
    end,

    __index = function(v, key)
        return rawget(v, to_vector_index(key))
    end,
    __newindex = function(v, key, value)
        assert(type(value) == 'number', 'vector component must be number; got ' .. tostring(value))

        extend_vector(v, key)
        rawset(v, to_vector_index(key), value)
    end,

    __tostring = function(v)
        return '[' .. util.join(v) .. ']'
    end,
}
vector_metatable.__index = Vector



-------------
-- Methods --
-------------

function Vector:new(...)
    local args = {...}
    first_arg = args[0]
    if type(first_arg) == 'vector' then
        return first_arg
    end

    if type(first_arg) == 'table' then
        local t = first_arg
        first_arg = nil
    else
        result = {}
        setmetatable(result, vector_metatable)
        for k, v in pairs(args) do
            result[k] = v
        end
        return result
    end
end

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



-----------------
-- Constructor --
-----------------

vec = Vector:new
