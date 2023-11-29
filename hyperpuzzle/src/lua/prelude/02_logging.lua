local old_warn = warn
local old_print = print

-- This is a default implementation that may be overwritten by Rust code.
function log_line(args)
  s = format("[%s] %s: %s", args.level:upper(), args.file, args.msg)
  if args.level == 'warn' then
    old_print(args.file)
  end
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

  table.insert(LOG_LINES, {
    msg = msg,
    file = file,
    level = level,
  })
end

function info(file, ...)
  log(file, 'info', ...)
end

print = info

function warn(file, ...)
  log(file, 'warn', ...)
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
