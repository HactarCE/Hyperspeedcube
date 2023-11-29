FILE_CONTENTS = {}
FILE_OUTPUTS = {}
PUZZLES = {}

function set_file_contents(filename, contents)
  info(nil, "Loading file contents")
  FILE_CONTENTS[filename] = contents
  unload_file(filename)
end

function unload_file(filename, log_prefix)
  log_prefix = log_prefix or '  '
  local file_output = FILE_OUTPUTS[filename]
  if file_output then
    -- Unload puzzles
    for puzzle_name in file_output.puzzles do
      PUZZLES[puzzle_name] = nil
      info(nil, log_prefix .. "Unloaded puzzle %q")
    end

    -- Unload dependencies
    local deps = file_output.dependencies
    if #deps > 0 then
      info(nil, log_prefix .. "Unloading dependencies of file %q ...", filename)
    end
    for _, dependency in ipairs(deps) do
      clear_file_output(dependency, log_prefix .. '  ')
    end
  end
  FILE_OUTPUTS[filename] = nil
  info(nil, log_prefix .. "Unloaded file %q", filename)
end
function load_file(filename)
  -- If we haven't loaded the file yet, then load it.
  if not FILE_OUTPUTS[filename] then
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
    local chunk = load(file_contents, filename, 't', FILE.environment)
    local is_success, error = xpcall(chunk, debug.traceback)

    -- Does any puzzle conflict?
    for puzzle_name in pairs(FILE.puzzles) do
      if not is_success then break end
      if PUZZLES[puzzle_name] then
        is_success = false
        error = format(
          "redefinition of puzzle %q (previously defined in %q)",
          puzzle_name, PUZZLES[puzzle_name].filename
        )
      end
    end

    -- Save the output of the file.
    if not is_success then
      FILE.error = error
    end
    for name, puzzle in pairs(FILE.puzzles) do
      PUZZLES[name] = puzzle
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
  info(nil, "Unloaded all files")
end
function load_all_files()
  for filename in pairs(FILE_CONTENTS) do
    -- Ignore errors
    xpcall(require, debug.traceback, filename)
  end
end

function require(filename)
  -- Automatically append `.lua`
  if filename:sub(-4) ~= '.lua' then
    filename = filename .. '.lua'
  end

  local file_output = load_file(filename)

  -- Error if the dependency failed to load.
  if file_output.error then
    error("Unable to load %q as required by %q", filename, FILE.name)
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
  local filename = get_file_containing_puzzle(puzzle_name)
  if not filename then return nil end

  local file_output = FILE_OUTPUTS[filename]
  if not file_output then return nil end

  return file_output.puzzles[puzzle_name]
end

function puzzledef(data)
  assert(type(data) == 'table', "expected table")
  assert(type(data.id) == 'string', "'id' is required and must be a string")
  data.file = FILE
  if FILE.puzzles[data.id] then
    error("redefinition of puzzle %q", data.id)
  else
    FILE.puzzles[data.id] = data
  end
end
