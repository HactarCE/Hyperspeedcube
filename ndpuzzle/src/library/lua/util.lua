-- Global utility functions

EPSILON = 0.0001

-- Approximate equality function
function approx_eq(a, b)
  if type(a) == 'number' and type(b) == 'number' then
    return math.abs(a - b) <= EPSILON
  else
    return a == b
  end
end

function toboolean(b)
  if b then
    return true
  else
    return false
  end
end

-- Returns whether a number is an exact integer
function isinteger(n)
  return type(n) == 'number' and n == math.floor(n)
end

function izip(f, ...)
  if select('#', ...) == 1 then return ipairs(...) end
  local function iter(args, i)
    i = i + 1
    local v = {}
    for j, v in ipairs(args) do
      v[j] = args[j][i]
    end
    return table.unpack(v)
  end

  return iter, {...}, 0
end

-- Returns whether `s` is a string that is an identifier
local function isidentifier(s)
  return toboolean(type(s) == 'string' and s:match(""))
end

-- Smart stringifier that prints contents of tables and avoids recursion issues
function pstring(t, indent, exclude)
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
          result = result .. '[' .. pstring(k, indent, exclude) .. ']'
        end

        -- print value
        result = result .. ' = ' .. pstring(v, indent, exclude) .. ',\n'
      end
    end

    result = result .. old_indent .. '}'
    return result
  else
    return tostring(t)
  end
end
function pprint(...)
  local args = {}
  for i, arg in ipairs{...} do
    table.insert(args, pstring(arg))
  end
  print(table.unpack(args))
end



-----------
-- TESTS --
-----------

function test_approx_eq()
  assert(approx_eq(2, 2))
  assert(not approx_eq(2, -2))
  assert(not approx_eq(2, 2.1))
  assert(not approx_eq(2, 2 + EPSILON * 2))
  assert(approx_eq(2, 2 + EPSILON / 2))
end

function test_isinteger()
  assert(isinteger(0))
  assert(isinteger(5))
  assert(isinteger(-5))
  assert(not isinteger(1.5))
  assert(not isinteger(1.1))
  assert(not isinteger(1.9))
end
