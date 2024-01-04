-- See http://lua-users.org/wiki/SandBoxes

SANDBOX_ENV = {
  -- Built-in constants
  _VERSION = _VERSION,

  -- Safe built-in functions
  ipairs = ipairs,
  next = next,
  pairs = pairs,
  select = select,
  tonumber = tonumber,
  tostring = tostring,

  -- Safe built-in modules
  math = math,
  string = string,
  table = table,
  utf8 = utf8,

  -- Safe custom functions
  assert = assert,
  error = error,
  warn = function(...) warn(FILE.name, ...) end,
  info = function(...) info(FILE.name, ...) end,
  pstring = pstring,
  print = function(...) info(FILE.name, ...) end,
  pprint = function(...) info(FILE.name, pstring(...)) end,
  type = type,

  -- Safe utility functions
  collect = collect,
  iter = iter,

  -- Library access
  puzzledef = puzzledef,
  require = require,

  -- Rust code will inject more entries into this table
}

-- Prevent modifications to globals
READ_ONLY_METATABLE = {__newindex = function() error('cannot overwrite bulitins') end}
setmetatable(math, READ_ONLY_METATABLE)
setmetatable(string, READ_ONLY_METATABLE)
setmetatable(table, READ_ONLY_METATABLE)
setmetatable(utf8, READ_ONLY_METATABLE)
setmetatable(SANDBOX_ENV, READ_ONLY_METATABLE)

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
