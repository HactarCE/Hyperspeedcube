-- Returns whether `s` is a string that is an identifier
local function isidentifier(s)
  return type(s) == 'string' and s:match("")
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
