PRETTY_TRACEBACK = true

-- This is a default implementation that may be overwritten by Rust code.
function log_line(args)
  local s
  if args.file == nil then
    s = string.format("[%s] %s", args.level:upper(), args.msg)
  else
    s = string.format("[%s] [%s] %s", args.level:upper(), args.file, args.msg)
  end
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

local PREFIX_LOGGING = debug.getinfo(1, 'S').short_src
local PREFIX_USER = "[string \"user:"
local PREFIX_PRELUDE = "[string \"prelude/"

function usertraceback(message)
  if not PRETTY_TRACEBACK then
    return debug.traceback(message)
  end

  if type(message) == 'string' then
    message = message:gsub("^%[[^%]]+%]:%d+: ", "")
  end

  local output = ""
  for line in debug.traceback(1):gmatch("[^\r\n]+") do
    if line:sub(2, #PREFIX_LOGGING + 1) == PREFIX_LOGGING then
      output = "" -- delete this line and all prior ones
    elseif line == "\t[C]: in function 'xpcall'" then
      break -- ignore this line and stop parsing
    elseif line:sub(2, #PREFIX_USER + 1) == PREFIX_USER then
      output = output .. "\n\t[file \"" .. line:sub(#PREFIX_USER + 2)
    elseif line:sub(2, #PREFIX_PRELUDE + 1) == PREFIX_PRELUDE then
      output = output .. "\n\t[internal \"" .. line:sub(#PREFIX_PRELUDE + 2)
    else
      output = output .. "\n" .. line
    end
  end

  if message then
    return tostring(message) .. "\nstack traceback:" .. output
  else
    return "stack traceback:" .. output
  end
end
