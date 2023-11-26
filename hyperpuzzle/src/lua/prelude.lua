-- See http://lua-users.org/wiki/SandBoxes



-------------------------------
-- PRETTY PRINTING UTILITIES --
-------------------------------

local reserved_words_list = {
  'and', 'break', 'do', 'else', 'elseif', 'end', 'false', 'for', 'function', 'goto', 'if',
  'in', 'local', 'nil', 'not', 'or', 'repeat', 'return', 'then', 'true', 'until', 'while',
}
local reserved_words_set = {}
for _, s in ipairs(reserved_words_list) do
  reserved_words_set[s] = true
end

-- Returns whether `s` is a string that is an identifier
local function isidentifier(s)
  return type(s) == 'string'
         and s:match("[%a_][%w_]*")
         and not reserved_words_set[s]
end

-- Smart stringifier that prints contents of tables and avoids recursion issues
local function pstring_internal(t, indent, exclude)
  if type(t) == 'string' then
    -- print as string literal
    return string.format('%q', t)
  elseif type(t) == 'table' then
    -- default arguments
    local old_indent = indent or ''
    local indent = old_indent .. '  '
    exclude = exclude or {}

    local result = tostring(t)
    if exclude[t] then
      -- if we've already printed this table, then don't print it again
      -- (guard against infinite recursion)
      return result
    else
      -- don't print this table in the future
      exclude[t] = true
    end

    result = result .. ' {';

    if next(t) ~= nil then
      -- if table is nonempty, print newline
      result = result .. '\n'

      for k, v in pairs(t) do
        result = result .. indent

        -- print key
        if isidentifier(k) then
          result = result .. k
        else
          result = result .. '[' .. pstring_internal(k, indent, exclude) .. ']'
        end

        -- print value
        result = result .. ' = ' .. pstring_internal(v, indent, exclude) .. ',\n'
      end
    end

    result = result .. old_indent .. '}'
    return result
  else
    return tostring(t)
  end
end

function pstring(...)
  return pstring_internal(...)
end

function pprint(...)
  local args = {}
  for i, arg in ipairs{...} do
    table.insert(args, pstring(arg))
  end
  print(table.unpack(args))
end



--------------------------
-- OTHER MONKEYPATCHING --
--------------------------

-- Add global `string.join()` utility function
function string.join(connector, t)
  connector = connector or ', '
  local result = ''
  for i, v in ipairs(t) do
    if i > 1 then
      result = result .. connector
    end
    result = result .. tostring(v)
  end
  return result
end

local old_error = error
function error(message, level)
  warn(message)
  old_error(message, (level or 1) + 1)
end

function assert(v, message)
  if not v then
    error(message or "assertion failed!", 2)
  end
end

-- Remove randomness
math.random = nil
math.randomseed = nil
-- Add extra constants
math.tau = math.pi * 2
math.phi = (1 + math.sqrt(5)) / 2



------------------------
-- LIBRARY MANAGEMENT --
------------------------

LIBRARY = {
  -- key = puzzle ID
  -- value = puzzle definition
  puzzles = {},

  -- key = filename
  -- value = table of exports
  exports = {}
}

function start_file(filename)
  FILE = {}
  FILE.name = filename
  FILE.puzzles = {}
  FILE.env = make_sandbox(filename)
end

function unload_file(filename)
  if LIBRARY.files[filename] then
    for id in pairs(LIBRARY.files[filename]) do
      local old_file = LIBRARY.objects[id].filename
      LIBRARY.objects[id] = nil
      info('unloaded %s from %q', id, old_file)
    end
    LIBRARY.files[filename] = nil
    info('unloaded file %q', filename)
  end
end

function finish_loading_file(sandbox)
  -- Unload old file if already loaded
  if LIBRARY.files[FILE.name] then
    warn('replacing already-loaded file %q', FILE.name)
    unload_file(FILE.name)
  end

  LIBRARY.files[FILE.name] = FILE.puzzles
  for id, obj in pairs(FILE.puzzles) do
    if LIBRARY.puzzles[id] then
      warn(
        'unable to load puzzle %q because it is already loaded from %q',
        id, LIBRARY.puzzles[id].FILE.name
      )
      FILE.puzzles[id] = nil
    else
      LIBRARY.puzzles[id] = obj
      info('loaded %s', id)
    end
  end
  info('loaded file %q', FILE.name)

  FILE = nil
end

function puzzledef(data)
  if type(data) ~= 'table' then
    error('expected table')
  end
  if type(data.id) ~= 'string' then
    error('"id" is required and must be a string')
  end
  data.filename = FILENAME
  if FILE.puzzles[data.id] then
    error('redefinition of puzzle %q', data.id)
  else
    info("loaded puzzle %q", data.id)
    FILE.puzzles[data.id] = data
  end
end



-------------------
-- SANDBOX SETUP --
-------------------

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
  warn = warn,
  pstring = pstring,
  print = print,
  pprint = pprint,
  type = type,

  -- Library access
  puzzledef = puzzledef,
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
