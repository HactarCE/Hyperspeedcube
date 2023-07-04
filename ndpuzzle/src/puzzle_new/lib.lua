-- Monkeypatch `type()` function to work for custom types.
local lua_builtin_type = type
type = function(obj)
    if lua_builtin_type(obj) == 'table' then
        local m = getmetatable(obj)
        if m.type then
            return m.type
        end
    end
    return lua_builtin_type(obj)
end

function approx_eq(a, b)
    if type(a) == 'number' and type(b) == 'number' then
        math.abs(a - b) <= EPSILON
    else
        return a == b
    end
end

util = require('util')
vector = require('vector')
multivector = require('multivector')

EPSILON = 0.0001
AXES = {
    'X', 'Y', 'Z', 'W', 'U', 'V', 'R', 'S',
    x=1, y=2, z=3, w=4, u=5, v=6, r=7, s=8,
    X=1, Y=2, Z=3, W=4, U=5, V=6, R=7, S=8,
}

function mv(arg)
    if type(arg) == 'vector' then
        return arg:to_multivector()
    elseif type(arg) == 'number' then
        return
    elseif type(arg)
end

v{x=1, w=1}
v{w=1}
