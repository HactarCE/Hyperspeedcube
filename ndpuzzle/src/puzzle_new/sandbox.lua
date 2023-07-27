-- See http://lua-users.org/wiki/SandBoxes

SECURE_ENV = {
    -- Constants
    _VERSION = _VERSION,
    _HSC = 'Hyperspeedcube',

    -- Safe built-in functions
    assert = assert,
    error = error,
    ipairs = ipairs,
    next = next,
    pairs = pairs,
    select = select,
    tonumber = tonumber,
    tostring = tostring,
    type = type, -- custom type

    -- Safe built-in modules
    math = seal_table(math, {}),
    string = seal_table(string, {}),
    table = seal_table(table, {}),
    utf8 = seal_table(utf8, {}),

    -- Common math constants and functions, for convenience
    sin = math.sin,
    cos = math.cos,
    tan = math.tan,
    asin = math.asin,
    acos = math.acos,
    atan = math.atan,
    sinh = math.sinh,
    cosh = math.cosh,
    tanh = math.tanh,
    atan2 = math.atan2,
    floor = math.floor,
    ceil = math.ceil,
    sqrt = math.sqrt,
    min = math.min,
    max = math.max,
    abs = math.abs,
    exp = math.exp,
    pi = math.pi,
    tau = math.pi * 2,

    -- Custom types
    vec = vec,
    mvec = mvec,
}
SECURE_ENV._G = SECURE_ENV

-- todo: custom print, error, warn

function make_secure_env()
    -- shallow copy is ok because modules are protected via metatable
    -- (and we do not give users the ability to manipulate/bypass metatable)
    return util.shallowcopy(SECURE_ENV)
end

print('printing!')
warn('warning!')
print('after warning')
error('erroring!')
print('after error')
