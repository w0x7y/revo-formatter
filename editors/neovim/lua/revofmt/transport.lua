local M = {}
-- Match the pinned CLI's admission limit; stderr is diagnostic text only.
local stdout_limit, stderr_limit = 262144, 65536

function M.start(config, source, on_exit)
  local stdout, stderr, sizes = {}, {}, { stdout = 0, stderr = 0 }
  local pipes, eof = {}, {}
  local process, timer, raw, completed
  local function close(handle)
    if handle and not handle:is_closing() then handle:close() end
  end
  local function cleanup()
    if timer then timer:stop(); close(timer) end
    for _, pipe in pairs(pipes) do close(pipe) end
  end
  local function settle(reason, canceled)
    if completed then return end
    -- Completion owns the deadline, independently of descendant pipe EOF.
    local status = raw or {}
    completed = {
      code = status.code,
      stdout = table.concat(stdout),
      stderr = table.concat(stderr),
      error = reason or (status.code == 124 and 'formatter timed out' or nil)
        or ((status.signal or 0) ~= 0 and ('formatter terminated by signal ' .. status.signal) or nil),
    }
    if reason and process then process:kill(9) end
    cleanup()
    if on_exit and not canceled then on_exit(completed) end
  end
  local function finish()
    if raw and eof.stdout and eof.stderr then settle() end
  end
  local function collect(name, chunks, limit)
    return function(err, data)
      if completed then return end
      if err then settle('could not read ' .. name .. ': ' .. tostring(err)); return end
      if not data then
        eof[name] = true
        close(pipes[name])
        finish()
        return
      end
      sizes[name] = sizes[name] + #data
      if sizes[name] > limit then
        settle(name .. ' exceeded the formatter output limit')
        return
      end
      chunks[#chunks + 1] = data
    end
  end
  -- Public vim.system objects cannot close their output pipes. Own the public
  -- libuv handles so inherited pipes cannot keep a settled request alive.
  local ok, spawned, spawn_error = pcall(function()
    for _, name in ipairs({ 'stdin', 'stdout', 'stderr' }) do
      pipes[name] = assert(vim.uv.new_pipe(false))
    end
    timer = assert(vim.uv.new_timer())
    return vim.uv.spawn(config.executable, {
      args = { '--indent-width', tostring(config.indent_width), '--line-width', tostring(config.line_width), '-' },
      stdio = { pipes.stdin, pipes.stdout, pipes.stderr },
    }, function(code, signal)
      raw = { code = code, signal = signal }
      close(process)
      process = nil
      close(pipes.stdin)
      finish()
    end)
  end)
  if not ok or not spawned then
    cleanup()
    return nil, 'could not start ' .. config.executable .. ': ' .. tostring(ok and spawn_error or spawned)
  end
  process = spawned
  local function check(ok_value, err, action)
    if not ok_value and not completed then settle('could not ' .. action .. ': ' .. tostring(err)) end
  end
  local reading, read_error = pipes.stdout:read_start(collect('stdout', stdout, stdout_limit))
  check(reading, read_error, 'read stdout')
  if not completed then
    reading, read_error = pipes.stderr:read_start(collect('stderr', stderr, stderr_limit))
    check(reading, read_error, 'read stderr')
  end
  if not completed then
    local writing, write_error = pipes.stdin:write(source, function(err)
      if completed or pipes.stdin:is_closing() then return end
      -- A child may reject input and exit without reading it; retain its stderr.
      if err and not err:find('EPIPE', 1, true) then settle('could not write stdin: ' .. err); return end
      if err then close(pipes.stdin); return end
      local shutting_down = pipes.stdin:shutdown(function() close(pipes.stdin) end)
      if not shutting_down then close(pipes.stdin) end
    end)
    check(writing, write_error, 'write stdin')
  end
  if not completed then
    local started, timer_error = timer:start(config.timeout_ms, 0, function() settle('formatter timed out') end)
    check(started, timer_error, 'start formatter deadline')
  end
  return {
    wait = function()
      if not completed then vim.wait(config.timeout_ms, function() return completed ~= nil end, 5, true) end
      if not completed then settle('formatter timed out') end
      return completed
    end,
    cancel = function() settle('formatter request was canceled', true) end,
  }
end

return M
