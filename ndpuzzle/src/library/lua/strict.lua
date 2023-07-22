-- Error on access to unassigned global variables
--
-- Modified from strict.lua
-- distributed under the Lua license: http://www.lua.org/license.html

local mt = getmetatable(_G)
if mt == nil then
  mt = {}
  setmetatable(_G, mt)
end

mt.__declared = {}

mt.__newindex = function (t, n, v)
  rawset(t, n, v)
end

mt.__index = function (t, n)
  if not mt.__declared[n] then
    error("variable '"..n.."' is not declared", 2)
  end
  return rawget(t, n)
end
