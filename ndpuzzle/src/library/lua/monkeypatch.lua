-- Monkeypatch built-in functions with custom behavior

-- Replace global `type()` function with one that understands our custom types
local lua_builtin_type = type
function type(obj)
  if lua_builtin_type(obj) == 'table' then
    local m = getmetatable(obj)
    if m and m.type then
      return m.type
    end
  end
  return lua_builtin_type(obj)
end

-- Replace global `ipairs()` function with one that understands vectors
local lua_builtin_ipairs = ipairs
function ipairs(t)
  if type(t) == 'vector' then
    local function iter(t, i)
      i = i + 1
      if i > #t then return nil end
      return i, t[i]
    end
    return iter, t, 0
  else
    return lua_builtin_ipairs(t)
  end
end

-- Add global `string.join()` utility function
function string.join(t, connector)
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

-- Applies a function to each key-value in a table and returns a new table with
-- the same keys
function table.map(f, ...)
  local keys = {}
  for i = 1, select('#', ...) do
    local t = select(i, ...)
    for k, _ in pairs(t) do
      keys[k] = true
    end
  end

  local result = {}
  local zipped_args = {}
  for k, _ in pairs(keys) do
    for j = 1, select('#', ...) do
      local arg = select(j, ...)
      zipped_args[j] = arg[k]
    end
    result[k] = f(table.unpack(zipped_args))
  end
  return result
end

-- Add global `table.shallowcopy()` utility function, which produces a shallow
-- copy including metatable (good enough for all our custom types)
function table.shallowcopy(t)
  local result = {}
  for k, v in pairs(t) do
    result[k] = v
  end
  setmetatable(result, getmetatable(t))
  return result
end
