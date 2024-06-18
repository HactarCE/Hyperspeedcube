-- See http://lua-users.org/wiki/SandBoxes

SANDBOX_ENV = {
  -- Built-in constants
  _VERSION = _VERSION,

  -- Safe built-in functions
  ipairs = ipairs,
  next = next,
  pairs = pairs,
  pcall = pcall,
  select = select,
  tonumber = tonumber,
  tostring = tostring,
  unpack = unpack,

  -- Safe built-in modules
  math = math,
  string = string,
  table = table,
  utf8 = utf8,

  -- Unpack most of the math module
  abs = math.abs,
  acos = math.acos,
  asin = math.asin,
  atan = math.atan,
  degree = math.degree,
  ceil = math.ceil,
  cos = math.cos,
  exp = math.exp,
  floor = math.floor,
  fmod = math.fmod,
  log = math.log,
  max = math.max,
  min = math.min,
  modf = math.modf,
  pi = math.pi,
  tau = math.tau,
  phi = math.phi,
  sin = math.sin,
  sqrt = math.sqrt,
  tan = math.tan,
  -- Including some custom Rust functions
  round = math.round,

  -- Safe custom functions
  assert = assert,
  error = error,
  warn = warn,
  pstring = pstring,
  print = print,
  pprint = pprint,
  type = type,
  setmetatable = function(table, metatable)
    -- Make a new table with the given metatable, which is much safer
    local t = {}
    for k, v in pairs(table) do
      t[k] = v
    end
    setmetatable(t, metatable)
    return t
  end,

  -- Rust code will inject more entries into this table
}

-- Prevent modifications to globals
READ_ONLY_METATABLE = {__newindex = function() error('table is sealed') end}
function seal(t) setmetatable(t, READ_ONLY_METATABLE) end
seal(math)
seal(string)
seal(table)
seal(utf8)
seal(SANDBOX_ENV)

function make_sandbox(filename)
  -- Construct a new table so that it's easy to see what globals have been added
  -- by the user
  local sandbox = {}
  sandbox._G = sandbox

  -- `__index` is ok because modules are protected via metatable
  -- (and we do not give users the ability to manipulate/bypass metatable)
  setmetatable(sandbox, {__index = SANDBOX_ENV})

  assert(type(filename) == 'string')
  sandbox.FILENAME = filename

  return sandbox
end
