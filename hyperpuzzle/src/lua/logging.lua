LOG_LINES = {}

function log(level, ...)
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
    file = FILE.name,
    level = level,
  })
end

function info(...)
  log('info', ...)
end

-- print = info

function warn(...)
  log('warn', ...)
end
