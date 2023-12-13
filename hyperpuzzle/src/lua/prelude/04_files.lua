FILE_CONTENTS = {}
FILE_OUTPUTS = {}
PUZZLES = {}

function set_file_contents(filename, contents)
  if FILE_CONTENTS[filename] == nil then
    if contents == nil then return end
    info(nil, "Adding file %q", filename)
  elseif contents == nil then
    info(nil, "Removing file %q", filename)
  else
    info(nil, "Updating file %q", filename)
  end
  FILE_CONTENTS[filename] = contents
  unload_file(filename)
end
function remove_all_files(filename)
  info(nil, "Removing all files ...")
  while next(FILE_CONTENTS) ~= nil do
    set_file_contents(next(FILE_CONTENTS), nil)
  end
end

function unload_file(filename, log_prefix)
  log_prefix = log_prefix or '  '
  local file_output = FILE_OUTPUTS[filename]
  if file_output ~= nil then
    -- Unload puzzles
    for puzzle_name in pairs(file_output.puzzles) do
      PUZZLES[puzzle_name] = nil
      info(nil, log_prefix .. "Unloaded puzzle %q", puzzle_name)
    end

    -- Unload dependencies
    local deps = file_output.dependencies
    if #deps > 0 then
      info(nil, log_prefix .. "Unloading dependencies of file %q ...", filename)
    end
    for _, dependency in ipairs(deps) do
      clear_file_output(dependency, log_prefix .. '  ')
    end

    FILE_OUTPUTS[filename] = nil
    info(nil, log_prefix .. "Unloaded file %q", filename)
  end
end
function load_file(filename)
  -- If we haven't loaded the file yet, then load it.
  if FILE_OUTPUTS[filename] == nil then
    -- Check that the file we want to load exists
    local file_contents = FILE_CONTENTS[filename]
    assert(file_contents, "missing file %q", filename)

    -- If we're in the middle of loading another file, then put that on pause.
    local old_file = FILE

    -- Set the global `FILE` variable for access by logging.
    FILE = {}
    FILE.name = filename
    FILE.puzzles = {}
    FILE.environment = make_sandbox(filename)
    FILE.dependencies = {}

    -- Execute the file.
    info(nil, "Loading file %q", filename)
    local error
    local is_success, load_result = pcall(load, file_contents, "user:" .. filename, 't', FILE.environment)
    if is_success then
      local chunk = load_result
      is_success, error = xpcall(chunk, usertraceback)
    else
      error = load_result
    end

    -- Does any puzzle conflict?
    for puzzle_name in pairs(FILE.puzzles) do
      if not is_success then break end
      if PUZZLES[puzzle_name] then
        is_success = false
        error = string.format(
          "redefinition of puzzle %q (previously defined in %q)",
          puzzle_name, PUZZLES[puzzle_name].filename
        )
      end
    end

    -- Save the output of the file.
    if not is_success then
      FILE.error = error
      warn(nil, "Error loading file %q:\n%s", FILE.name, FILE.error)
    end
    for puzzle_name, puzzle in pairs(FILE.puzzles) do
      PUZZLES[puzzle_name] = puzzle
      info(FILE.name, "Loaded puzzle %q", puzzle_name)
    end
    FILE_OUTPUTS[filename] = FILE

    -- Resume loading the previous file.
    FILE = old_file
  end

  return FILE_OUTPUTS[filename]
end

function unload_all_files()
  FILE_OUTPUTS = {}
  PUZZLES = {}
  info(nil, "Unloaded all files")
end
function load_all_files()
  for filename in pairs(FILE_CONTENTS) do
    load_file(filename)
  end
end

function require(filename)
  assert(type(filename) == 'string', "expected string")

  -- Automatically append `.lua`
  if filename:sub(-4) ~= '.lua' then
    filename = filename .. '.lua'
  end

  local file_output = load_file(filename)

  -- Error if the dependency failed to load.
  if file_output.error then
    error(string.format("Unable to load %q as required by %q", filename, FILE.name))
  end

  -- Return the environment of the dependency.
  return file_output.environment
end

function get_puzzles_from_file(filename)
  if FILE_OUTPUTS[filename] then
    return FILE_OUTPUTS[filename].puzzles
  else
    return {}
  end
end

function get_file_containing_puzzle(puzzle_name)
  if PUZZLES[puzzle_name] then
    return PUZZLES[puzzle_name].filename
  end
end

function get_puzzle(puzzle_name)
  return PUZZLES[puzzle_name]
end

function puzzledef(data)
  assert(type(data) == 'table', "expected table")
  assert(type(data.id) == 'string', "'id' is required and must be a string")
  data.file = FILE
  if FILE.puzzles[data.id] then
    error(string.format("redefinition of puzzle %q", data.id))
  else
    FILE.puzzles[data.id] = data
  end
end
