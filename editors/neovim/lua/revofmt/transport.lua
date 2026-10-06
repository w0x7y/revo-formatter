local M = {}
-- Match the pinned CLI's admission limit; stderr is diagnostic text only.
local stdout_limit, stderr_limit = 262144, 65536

function M.start(config, source, on_exit)
  local stdout, stderr, sizes = {}, {}, { stdout = 0, stderr = 0 }
  local process, timer, failure, timed_out
  local function stop_timer()
    if timer then timer:stop(); timer:close(); timer = nil end
  end
  local function collect(name, chunks, limit)
    return function(err, data)
      if err then failure = failure or ('could not read ' .. name .. ': ' .. tostring(err)) end
      if not data or failure then return end
      sizes[name] = sizes[name] + #data
      if sizes[name] > limit then
        failure = name .. ' exceeded the formatter output limit'
        if process then process:kill(9) end
        return
      end
      chunks[#chunks + 1] = data
    end
  end
  local function result(raw)
    return {
      code = raw.code,
      stdout = table.concat(stdout),
      stderr = table.concat(stderr),
      error = failure or ((timed_out or raw.code == 124) and 'formatter timed out' or nil)
        or ((raw.signal or 0) ~= 0 and ('formatter terminated by signal ' .. raw.signal) or nil),
    }
  end
  local ok, spawned = pcall(vim.system, {
    config.executable, '--indent-width', tostring(config.indent_width),
    '--line-width', tostring(config.line_width), '-',
  }, {
    stdin = source,
    text = false,
    stdout = collect('stdout', stdout, stdout_limit),
    stderr = collect('stderr', stderr, stderr_limit),
  }, function(raw)
    stop_timer()
    if on_exit then on_exit(result(raw)) end
  end)
  if not ok then return nil, 'could not start ' .. config.executable .. ': ' .. tostring(spawned) end
  process = spawned
  timer = vim.uv.new_timer()
  timer:start(config.timeout_ms, 0, function()
    timed_out = true
    process:kill(9)
    stop_timer()
  end)
  return {
    wait = function()
      local raw = process:wait(config.timeout_ms)
      stop_timer()
      return result(raw)
    end,
    cancel = function()
      process:kill(9)
      stop_timer()
    end,
  }
end

return M
