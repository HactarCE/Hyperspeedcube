library = {
  files = {},
  objects = {},
}

function library.start_loading_file(filename)
  assert(OBJECTS_DEFINED_IN_FILE == nil, 'file loading already in progress')
  FILENAME = filename
  OBJECTS_DEFINED_IN_FILE = {}
end

function library.load_object(obj_type, obj)
  if type(obj) ~= 'table' then
    error('expected table')
  end
  if type(obj.name) ~= 'string' then
    error('"name" is required and must be a string')
  end
  obj.filename = FILENAME
  obj.type = obj_type
  obj.id = string.format('%s[%q]', obj.type, obj.name)
  if OBJECTS_DEFINED_IN_FILE[obj.id] then
    warn('ignoring redefinition of %s', obj.id)
  else
    OBJECTS_DEFINED_IN_FILE[obj.id] = obj
  end
end

function library.finish_loading_file(filename)
  -- Unload old file if already loaded
  if library.files[filename] then
    warn('replacing already-loaded file %q', filename)
    library.unload_file(filename)
  end

  info('loading file %q', filename)
  library.files[filename] = OBJECTS_DEFINED_IN_FILE
  for id, obj in pairs(OBJECTS_DEFINED_IN_FILE) do
    if library.objects[id] then
      warn(
        'unable to load %s because it is already loaded from %q',
        id, library.objects[id].filename
      )
      OBJECTS_DEFINED_IN_FILE[id] = nil
    else
      library.objects[id] = obj
      info('loaded %s', id)
    end
  end
  info('loaded file %q', filename)

  OBJECTS_DEFINED_IN_FILE = nil
end

function library.unload_file(filename)
  if library.files[filename] then
    for id in pairs(library.files[filename]) do
      local old_file = library.objects[id].filename
      library.objects[id] = nil
      info('unloaded %s from %q', id, old_file)
    end
    library.files[filename] = nil
    info('unloaded file %q', filename)
  end
end
