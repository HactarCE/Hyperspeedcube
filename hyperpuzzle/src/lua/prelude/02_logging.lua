-- This is a default implementation that may be overwritten by Rust code.
function log_line(args)
  local s = string.format("[%s] [%s]: %s", args.level:upper(), args.file or '<internal>', args.msg)
  print(s)
end

function log(file, level, ...)
  local msg
  if select('#', ...) == 0 then
    msg = ''
  elseif select('#', ...) == 1 then
    msg = tostring(...)
  else
    msg = string.format(...)
  end

  log_line{
    msg = msg,
    file = file,
    level = level,
  }
end

function info(file, ...)
  log(file, 'info', ...)
end

-- Overwrite `warn()`
function warn(file, ...)
  log(file, 'warn', ...)
end

local old_error = error
function error(message)
  log(FILE.name, 'error', message)
  old_error(message)
end

function assert(v, ...)
  if not v then
    error(string.format(...) or "assertion failed!")
  end
end

local ASSERT_PREFIX = debug.getinfo(1, 'S').short_src
local PREFIX_LOGGING = "\t" .. debug.getinfo(1, 'S').short_src
local PREFIX_USER = "\t[string \"user:"
local PREFIX_PRELUDE = "\t[string \"prelude/"

function usertraceback(message)
  message = message:gsub("^%[[^%]]+%]:%d+: ", "")

  local output = ""
  for line in debug.traceback(1):gmatch("[^\r\n]+") do
    if line:sub(1, #PREFIX_LOGGING) == PREFIX_LOGGING then
      output = "" -- delete this line and all prior ones
    elseif line == "\t[C]: in function 'xpcall'" then
      break -- ignore this line and stop parsing
    elseif line:sub(1, #PREFIX_USER) == PREFIX_USER then
      output = output .. "\n\t[file \"" .. line:sub(#PREFIX_USER + 1)
    elseif line:sub(1, #PREFIX_PRELUDE) == PREFIX_PRELUDE then
      output = output .. "\n\t[internal \"" .. line:sub(#PREFIX_PRELUDE + 1)
    else
      output = output .. "\n" .. line
    end
  end

  if message then
    return message .. "\nstack traceback:" .. output
  else
    return "stack traceback:" .. output
  end
end
