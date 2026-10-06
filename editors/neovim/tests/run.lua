-- Run from the repository root; no personal configuration is loaded.
local root = vim.fn.getcwd()
vim.opt.runtimepath:prepend(root .. '/editors/neovim')
local real = vim.env.REVOFMT_BIN or (root .. '/target/debug/revofmt')
local fixture = root .. '/editors/neovim/tests/fixture.py'
local errors, tests, failed = {}, 0, 0
vim.notify = function(message) errors[#errors + 1] = message end
local function equal(actual, expected)
  assert(vim.deep_equal(actual, expected), 'expected ' .. vim.inspect(expected) .. ', got ' .. vim.inspect(actual))
end
local function test(name, body)
  tests = tests + 1
  errors = {}
  local ok, err = pcall(body)
  if ok then print('PASS ' .. name) else failed = failed + 1; print('FAIL ' .. name .. ': ' .. err) end
end
local function plugin(opts)
  local fmt = require('revofmt')
  fmt.setup(vim.tbl_extend('force', { executable = real }, opts or {}))
  return fmt
end
local function buffer(lines, opts)
  local buf = vim.api.nvim_create_buf(true, false)
  vim.api.nvim_set_current_buf(buf)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  vim.bo[buf].fileformat = 'unix'
  vim.bo[buf].endofline = false
  for key, value in pairs(opts or {}) do vim.bo[buf][key] = value end
  return buf
end
-- Independent serialization: assertions compare bytes at the editor boundary.
local function bytes(buf)
  local sep = vim.bo[buf].fileformat == 'dos' and '\r\n' or '\n'
  return table.concat(vim.api.nvim_buf_get_lines(buf, 0, -1, false), sep) .. (vim.bo[buf].endofline and sep or '')
end
local function format(fmt, buf)
  local ok, err = fmt.format({ bufnr = buf, async = false })
  assert(ok, err)
end
local function drain(ms)
  vim.wait(ms or 450, function() return false end, 10)
end
local function controlled(mode, opts)
  vim.env.REVOFMT_TEST_MODE = mode
  return plugin(vim.tbl_extend('force', { executable = fixture }, opts or {}))
end

test('formats the current unsaved source through the real CLI', function()
  local fmt = plugin()
  local buf = buffer({ 'let x=1' })
  format(fmt, buf)
  equal(bytes(buf), 'let x = 1\n')
  local tick = vim.api.nvim_buf_get_changedtick(buf)
  format(fmt, buf)
  equal(bytes(buf), 'let x = 1\n')
  equal(vim.api.nvim_buf_get_changedtick(buf), tick)
end)
test('recognizes both suffixes and registers one command after repeated setup', function()
  plugin(); plugin()
  for _, suffix in ipairs({ 'rv', 'revo' }) do
    local buf = buffer({ '' })
    vim.api.nvim_buf_set_name(buf, vim.fn.tempname() .. '.' .. suffix)
    vim.api.nvim_exec_autocmds('BufRead', { buffer = buf })
    equal(vim.bo[buf].filetype, 'revo')
  end
  local buf = buffer({ 'let x=1' })
  vim.cmd('RevoFormat')
  assert(vim.wait(2000, function() return bytes(buf) == 'let x = 1\n' end, 10))
end)
test('defaults to asynchronous manual formatting', function()
  local fmt = controlled('delay')
  local buf = buffer({ 'let x=1' })
  assert(fmt.format({ bufnr = buf }))
  equal(bytes(buf), 'let x=1')
  assert(vim.wait(2000, function() return bytes(buf) == 'let x = 1\n' end, 10))
end)
test('keeps canonical empty buffers untouched', function()
  local fmt = plugin({ executable = '/missing/formatter' })
  local buf = buffer({ '' })
  local tick = vim.api.nvim_buf_get_changedtick(buf)
  format(fmt, buf)
  equal(bytes(buf), '')
  equal(vim.api.nvim_buf_get_changedtick(buf), tick)
end)
test('preserves Unix, DOS and mixed opaque literal endings', function()
  local fmt = plugin()
  local cases = {
    { { 'let x=1' }, 'unix', 'let x = 1\n' },
    { { 'let x=1' }, 'dos', 'let x = 1\r\n' },
    { { "let x='first\r", "second'" }, 'unix', "let x = 'first\r\nsecond'\n" },
    { { "let x='first", "second'\r", '# end\r' }, 'unix', "let x = 'first\nsecond'\r\n# end\r\n" },
    { { "let x='first", "second'" }, 'dos', "let x = 'first\r\nsecond'\r\n" },
    { { "let x='first\r", "second'" }, 'dos', "let x = 'first\r\r\nsecond'\r\n" },
  }
  for _, case in ipairs(cases) do
    local buf = buffer(case[1], { fileformat = case[2], endofline = true })
    format(fmt, buf); equal(bytes(buf), case[3])
    format(fmt, buf); equal(bytes(buf), case[3])
  end
end)
test('rejects output that DOS buffers cannot represent exactly', function()
  local fmt = controlled('unrepresentable')
  local buf = buffer({ 'let x=1' }, { fileformat = 'dos' })
  local ok = fmt.format({ bufnr = buf, async = false })
  assert(not ok); equal(bytes(buf), 'let x=1')
  assert(table.concat(errors):find('represent', 1, true))
end)
test('retains views and changes only one undoable region', function()
  local fmt = plugin()
  local buf = buffer({ '# before', 'let x=1', '# after' }, { endofline = true })
  -- A completed editor command gives formatting a distinct undo boundary.
  vim.cmd('let &undolevels = &undolevels')
  vim.api.nvim_win_set_cursor(0, { 3, 2 })
  local view = vim.fn.winsaveview()
  local edits = {}
  vim.api.nvim_buf_attach(buf, false, { on_lines = function(_, _, _, first, last, new_last)
    edits[#edits + 1] = { first, last, new_last }
  end })
  format(fmt, buf)
  equal(bytes(buf), '# before\nlet x = 1\n# after\n')
  equal(edits, { { 1, 2, 2 } })
  equal(vim.fn.winsaveview(), view)
  vim.cmd('undo')
  equal(bytes(buf), '# before\nlet x=1\n# after\n')
end)
test('does not apply a result after an intervening edit', function()
  local fmt = controlled('delay')
  local buf = buffer({ 'let x=1' })
  fmt.format({ bufnr = buf })
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, { 'let x=2' })
  drain(); equal(bytes(buf), 'let x=2')
end)
test('supersedes overlapping requests even with identical changedticks', function()
  local fmt = controlled('delay')
  local buf = buffer({ 'let x=1' })
  fmt.format({ bufnr = buf })
  plugin({ executable = '/missing/formatter' }).format({ bufnr = buf })
  drain(); equal(bytes(buf), 'let x=1')
end)
test('does not apply to deleted or unloaded buffers', function()
  for _, action in ipairs({ 'wipeout', 'unload' }) do
    local fmt = controlled('delay')
    local buf = buffer({ 'let x=1' })
    fmt.format({ bufnr = buf })
    vim.api.nvim_buf_delete(buf, { force = true, unload = action == 'unload' })
    drain()
    assert(not vim.api.nvim_buf_is_loaded(buf))
  end
end)
test('guards serialization options and modifiability', function()
  local options = { fileformat = 'dos', endofline = true, fixendofline = false,
    binary = true, fileencoding = 'latin1', bomb = true, modifiable = false }
  for key, value in pairs(options) do
    local fmt = controlled('delay')
    local buf = buffer({ 'let x=1' })
    fmt.format({ bufnr = buf })
    vim.bo[buf][key] = value
    drain()
    equal(vim.api.nvim_buf_get_lines(buf, 0, -1, false), { 'let x=1' })
  end
end)
test('rejects unsupported buffer representations before subprocess work', function()
  for key, value in pairs({ binary = true, fileencoding = 'latin1', fileformat = 'mac', bomb = true }) do
    local fmt = plugin()
    local buf = buffer({ 'let x=1' }, { [key] = value })
    assert(not fmt.format({ bufnr = buf, async = false }))
    equal(vim.api.nvim_buf_get_lines(buf, 0, -1, false), { 'let x=1' })
  end
end)
test('reports missing executable, CLI syntax rejection and stderr', function()
  for _, case in ipairs({
    { '/missing/formatter', 'let x=1', 'missing' },
    { real, 'let x =', 'unexpected token' },
    { fixture, 'let x=1', 'controlled syntax failure' },
  }) do
    errors = {}
    vim.env.REVOFMT_TEST_MODE = 'stderr'
    local fmt = plugin({ executable = case[1] })
    local buf = buffer({ case[2] })
    assert(not fmt.format({ bufnr = buf, async = false }))
    equal(bytes(buf), case[2])
    assert(table.concat(errors):lower():find(case[3], 1, true), table.concat(errors))
  end
end)
test('bounds synchronous and asynchronous timeouts without modifying buffers', function()
  for _, async in ipairs({ false, true }) do
    local fmt = controlled('timeout', { timeout_ms = 40 })
    local buf = buffer({ 'let x=1' })
    local start = vim.uv.hrtime()
    local ok = fmt.format({ bufnr = buf, async = async })
    if async then drain(200) else assert(not ok) end
    assert((vim.uv.hrtime() - start) / 1e6 < 700)
    equal(bytes(buf), 'let x=1')
    assert(table.concat(errors):find('timed out', 1, true), table.concat(errors))
  end
end)
-- Each inherited-pipe case owns a bounded child even when an assertion fails.
local function inherited_pipes(body)
  local pid_path = vim.fn.tempname()
  vim.env.REVOFMT_TEST_CHILD_PID = pid_path
  -- uv.walk can crash on Neovim 0.10's native handles. Observe actual
  -- transport allocations instead, retaining their normal libuv behavior.
  local owned, constructors = {}, {}
  for _, name in ipairs({ 'new_pipe', 'new_timer', 'spawn' }) do
    local original = vim.uv[name]
    constructors[name] = original
    vim.uv[name] = function(...)
      local handle, extra, detail = original(...)
      if handle then owned[#owned + 1] = handle end
      return handle, extra, detail
    end
  end
  local function cleaned_up()
    for _, handle in ipairs(owned) do
      if not handle:is_closing() then return false end
    end
    return true
  end
  local fmt = controlled('inherited-pipes', { timeout_ms = 150 })
  local ok, err = pcall(body, fmt, cleaned_up, pid_path)
  for name, original in pairs(constructors) do vim.uv[name] = original end
  local spawned = vim.fn.filereadable(pid_path) == 1
  if spawned and vim.fn.filereadable(pid_path .. '.done') == 0 then
    vim.uv.kill(tonumber(vim.fn.readfile(pid_path)[1]), 9)
  end
  vim.fn.delete(pid_path)
  vim.fn.delete(pid_path .. '.done')
  vim.env.REVOFMT_TEST_CHILD_PID = nil
  drain(60)
  assert(ok, err)
  assert(spawned, 'fixture wrapper never spawned its inherited-pipe descendant')
end

test('returns a bounded synchronous timeout when a wrapper exits with inherited pipes', function()
  inherited_pipes(function(fmt, cleaned_up)
    local buf = buffer({ 'let x=1' })
    local tick, start = vim.api.nvim_buf_get_changedtick(buf), vim.uv.hrtime()
    local ok, err = fmt.format({ bufnr = buf, async = false })
    assert(not ok and err:find('timed out', 1, true), tostring(err))
    assert((vim.uv.hrtime() - start) / 1e6 < 500, 'timeout waited for descendant EOF')
    equal(bytes(buf), 'let x=1')
    equal(vim.api.nvim_buf_get_changedtick(buf), tick)
    assert(vim.wait(100, cleaned_up, 5), 'timeout left transport handles open')
  end)
end)
test('manual async timeout settles before inherited pipe EOF and ignores late output', function()
  inherited_pipes(function(_, cleaned_up)
    local buf = buffer({ 'let x=1' })
    local tick, start = vim.api.nvim_buf_get_changedtick(buf), vim.uv.hrtime()
    vim.cmd('RevoFormat')
    assert(vim.wait(500, function() return #errors > 0 end, 5), 'no timeout notification before descendant EOF')
    assert((vim.uv.hrtime() - start) / 1e6 < 500)
    assert(errors[1]:find('timed out', 1, true), errors[1])
    assert(vim.wait(100, cleaned_up, 5), 'timeout left transport handles open')
    drain(900) -- Let the descendant publish late stdout/stderr and close its pipes.
    equal(#errors, 1)
    equal(bytes(buf), 'let x=1')
    equal(vim.api.nvim_buf_get_changedtick(buf), tick)
  end)
end)
for _, canceled in ipairs({ false, true }) do
  test(canceled and 'canceled inherited-pipe transport releases handles without completion'
    or 'inherited-pipe transport completes exactly once at its deadline', function()
    inherited_pipes(function(_, cleaned_up, pid_path)
      local calls, result = 0, nil
      local operation, err = require('revofmt.transport').start({
        executable = fixture, indent_width = 2, line_width = 80, timeout_ms = 150,
      }, 'let x=1', function(value) calls = calls + 1; result = value end)
      assert(operation, err)
      if canceled then
        assert(vim.wait(100, function() return vim.fn.filereadable(pid_path) == 1 end, 5))
        drain(20) -- The wrapper exits while the descendant keeps its pipes open.
        operation.cancel()
      else
        assert(vim.wait(500, function() return calls > 0 end, 5), 'transport callback waited for pipe EOF')
        assert(result.error:find('timed out', 1, true))
      end
      assert(vim.wait(100, cleaned_up, 5), 'settled transport left handles open')
      drain(900)
      equal(calls, canceled and 0 or 1)
      if not canceled then equal(result.stdout, ''); equal(result.stderr, '') end
    end)
  end)
end
test('inherited-pipe timeout still saves the unchanged user source', function()
  inherited_pipes(function(_, cleaned_up)
    local path = vim.fn.tempname() .. '.rv'
    local buf = buffer({ 'let x=1' })
    vim.api.nvim_buf_set_name(buf, path); vim.bo[buf].filetype = 'revo'
    controlled('inherited-pipes', { timeout_ms = 150, format_on_save = true })
    local start = vim.uv.hrtime()
    local ok, err = pcall(vim.cmd, 'write!')
    local contents = vim.fn.filereadable(path) == 1 and vim.fn.readfile(path) or nil
    vim.fn.delete(path)
    assert(ok, err)
    assert((vim.uv.hrtime() - start) / 1e6 < 500, 'save waited for descendant EOF')
    equal(contents, { 'let x=1' })
    equal(bytes(buf), 'let x=1')
    assert(table.concat(errors):find('timed out', 1, true), table.concat(errors))
    assert(vim.wait(100, cleaned_up, 5), 'save timeout left transport handles open')
  end)
end)

test('rejects excessive subprocess output', function()
  local fmt = controlled('oversize')
  local buf = buffer({ 'let x=1' })
  assert(not fmt.format({ bufnr = buf, async = false }))
  equal(bytes(buf), 'let x=1')
  assert(table.concat(errors):find('limit', 1, true), table.concat(errors))
end)
test('save formatting is disabled by default and enabled synchronously', function()
  local path = vim.fn.tempname() .. '.rv'
  local buf = buffer({ 'let x=1' })
  vim.api.nvim_buf_set_name(buf, path); vim.bo[buf].filetype = 'revo'
  plugin(); vim.cmd('write!')
  equal(vim.fn.readfile(path), { 'let x=1' })
  plugin({ format_on_save = true }); plugin({ format_on_save = true })
  vim.cmd('write!')
  equal(vim.fn.readfile(path), { 'let x = 1' })
  equal(bytes(buf), 'let x = 1\n')
  vim.fn.delete(path)
end)
test('failed save formatting retains the buffer and still writes user source', function()
  local path = vim.fn.tempname() .. '.rv'
  local buf = buffer({ 'let x =' })
  vim.api.nvim_buf_set_name(buf, path); vim.bo[buf].filetype = 'revo'
  plugin({ format_on_save = true }); vim.cmd('write!')
  equal(bytes(buf), 'let x =')
  equal(vim.fn.readfile(path), { 'let x =' })
  vim.fn.delete(path)
end)
test('validates options without replacing a working setup', function()
  local fmt = plugin()
  for _, opts in ipairs({ { indent_width = 0 }, { indent_width = 9 }, { line_width = 19 },
    { line_width = 241 }, { timeout_ms = 0 }, { executable = '' }, { format_on_save = 'yes' } }) do
    assert(not pcall(fmt.setup, opts))
  end
  local buf = buffer({ 'let x=1' }); format(fmt, buf); equal(bytes(buf), 'let x = 1\n')
end)
test('rejects literal NUL output without modifying the buffer', function()
  local fmt = plugin()
  local buf = buffer({ "let x='a\0b'" })
  assert(not fmt.format({ bufnr = buf, async = false }))
  equal(vim.api.nvim_buf_get_lines(buf, 0, -1, false), { "let x='a\0b'" })
  assert(table.concat(errors):find('NUL', 1, true))
end)
test('plugin startup retains setup made in the user init file', function()
  local fmt = plugin({ executable = '/missing/configured-revofmt' })
  vim.cmd('runtime plugin/revofmt.lua')
  local buf = buffer({ 'let x=1' })
  assert(not fmt.format({ bufnr = buf, async = false }))
  assert(table.concat(errors):find('/missing/configured-revofmt', 1, true), table.concat(errors))
end)
test('marks an EOL-only formatting change as modified', function()
  local fmt = plugin()
  local buf = buffer({ 'let x = 1' })
  vim.bo[buf].modified = false
  format(fmt, buf)
  equal(bytes(buf), 'let x = 1\n')
  assert(vim.bo[buf].modified, 'adding a final newline must mark the buffer modified')
end)
test('repeated save setup runs once and disabling it removes the hook', function()
  local path = vim.fn.tempname() .. '.revo'
  local buf = buffer({ 'let x=1' })
  vim.api.nvim_buf_set_name(buf, path); vim.bo[buf].filetype = 'revo'
  controlled('delay', { format_on_save = true })
  controlled('delay', { format_on_save = true })
  vim.cmd('write!')
  equal(vim.fn.readfile(path), { 'let x = 1' })
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, { 'let x=1' })
  plugin({ format_on_save = false }); vim.cmd('write!')
  equal(vim.fn.readfile(path), { 'let x=1' })
  vim.fn.delete(path)
end)
test('passes configured layout arguments to the real CLI', function()
  local fmt = plugin({ indent_width = 4, line_width = 24 })
  local buf = buffer({ 'print(first_argument, second_argument)' })
  format(fmt, buf)
  equal(bytes(buf), 'print(\n    first_argument,\n    second_argument\n)\n')
end)
test('executes a path containing spaces and shell punctuation directly', function()
  local path = vim.fn.tempname() .. ' formatter; direct'
  assert(vim.uv.fs_symlink(fixture, path))
  local fmt = controlled('delay', { executable = path })
  local buf = buffer({ 'let x=1' })
  format(fmt, buf); equal(bytes(buf), 'let x = 1\n')
  vim.fn.delete(path)
end)
test('admits input before starting a subprocess', function()
  local fmt = plugin({ executable = '/missing/formatter' })
  local source = string.rep('x', 262145)
  local buf = buffer({ source })
  assert(not fmt.format({ bufnr = buf, async = false }))
  equal(bytes(buf), source)
  assert(table.concat(errors):find('input byte limit', 1, true))
end)
for _, async in ipairs({ false, true }) do
  test('refuses signal-terminated output in ' .. (async and 'async' or 'sync') .. ' mode', function()
    local fmt = controlled('signal')
    local buf = buffer({ 'let x=1' })
    local tick = vim.api.nvim_buf_get_changedtick(buf)
    local ok = fmt.format({ bufnr = buf, async = async })
    if async then
      assert(ok)
      assert(vim.wait(2000, function() return #errors > 0 end, 10))
    else
      assert(not ok, 'a process killed by a signal must fail even when its exit code is zero')
    end
    equal(bytes(buf), 'let x=1')
    equal(vim.api.nvim_buf_get_changedtick(buf), tick)
    assert(table.concat(errors):find('signal', 1, true))
  end)
end
print(string.format('%d tests, %d failures', tests, failed))
vim.cmd(failed == 0 and 'qa!' or 'cquit 1')
