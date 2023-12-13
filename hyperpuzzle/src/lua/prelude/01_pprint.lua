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
  local outputs = {}
  for i, arg in ipairs{...} do
    table.insert(outputs, pstring_internal(arg))
  end
  return table.unpack(outputs)
end

function pprint(...)
  print(pstring(...))
end
