-- See http://lua-users.org/wiki/SandBoxes

function seal_table(t, members)
  local result = {}
  if members then
    for _, key in ipairs(members) do
      result[key] = t[key]
    end
  else
    for key, value in pairs(t) do
      result[key] = value
    end
  end
  setmetatable(result, {
    __newindex = function()
      error('cannot overwrite builtins')
    end
  })
  return result
end

local old_error = error
function error(...)
  warn(...)
  old_error(...)
end

math.tau = math.pi * 2
math.phi = (1 + math.sqrt(5)) / 2

SANDBOX_ENV = {
  -- Built-in constants
  _VERSION = _VERSION,

  -- Safe built-in functions
  assert = assert,
  error = error,
  ipairs = ipairs,
  next = next,
  pairs = pairs,
  select = select,
  tonumber = tonumber,
  tostring = tostring,
  type = type,

  -- Safe built-in modules
  math = seal_table(math),
  -- removed: string.dump, string.pack, string.packsize, string.unpack
  string = seal_table(string, {'byte', 'char', 'find', 'format', 'gmatch', 'gsub', 'join', 'len', 'lower', 'match', 'rep',
                               'reverse', 'sub', 'upper'}),
  table = seal_table(table),
  utf8 = seal_table(utf8),

  -- Safe custom functions
  pstring = pstring,
  print = print,
  warn = warn,

  -- Library access
  puzzledef = function(...) library.load_object('puzzle', ...) end,
  twistdef = function(...) library.load_object('twist', ...) end,
  shapedef = function(...) library.load_object('shape', ...) end,
}

function make_sandbox(filename)
  -- shallow copy is ok because modules are protected via metatable
  -- (and we do not give users the ability to manipulate/bypass metatable)
  local safe_env = table.shallowcopy(SANDBOX_ENV)
  safe_env._G = safe_env

  assert(type(filename) == 'string')
  safe_env.FILENAME = filename

  return safe_env
end
