local codec = require('revofmt.codec')
local transport = require('revofmt.transport')
local M = {}
local defaults = {
  executable = 'revofmt', indent_width = 2, line_width = 80,
  timeout_ms = 5000, format_on_save = false,
}
local config = vim.deepcopy(defaults)
local pending = {}
local option_names = {
  'fileformat', 'endofline', 'fixendofline', 'binary', 'fileencoding', 'bomb', 'modifiable',
}

local function fail(message)
  vim.notify('revofmt: ' .. message, vim.log.levels.ERROR)
  return false, message
end

local function options(buf)
  local values = {}
  for _, name in ipairs(option_names) do values[name] = vim.bo[buf][name] end
  return values
end

local function cancel(buf)
  local operation = pending[buf]
  pending[buf] = nil
  if operation and operation.process then operation.process.cancel() end
end

local function apply(buf, before, after)
  local first, last_before, last_after = 0, #before, #after.lines
  while first < last_before and first < last_after and before[first + 1] == after.lines[first + 1] do
    first = first + 1
  end
  while last_before > first and last_after > first and before[last_before] == after.lines[last_after] do
    last_before, last_after = last_before - 1, last_after - 1
  end
  local views = {}
  for _, win in ipairs(vim.fn.win_findbuf(buf)) do
    views[win] = vim.api.nvim_win_call(win, vim.fn.winsaveview)
  end
  if first ~= last_before or first ~= last_after then
    local replacement = {}
    for i = first + 1, last_after do replacement[#replacement + 1] = after.lines[i] end
    vim.api.nvim_buf_set_lines(buf, first, last_before, false, replacement)
  end
  if vim.bo[buf].endofline ~= after.endofline then
    vim.bo[buf].endofline = after.endofline
    vim.bo[buf].modified = true
  end
  for win, view in pairs(views) do
    if vim.api.nvim_win_is_valid(win) and vim.api.nvim_win_get_buf(win) == buf then
      vim.api.nvim_win_call(win, function() vim.fn.winrestview(view) end)
    end
  end
end

-- Returns true for a completed synchronous operation or a started async one.
-- Errors return false, message and are also shown through vim.notify.
function M.format(opts)
  opts = opts or {}
  local buf = opts.bufnr or vim.api.nvim_get_current_buf()
  if buf == 0 then buf = vim.api.nvim_get_current_buf() end
  cancel(buf)
  if not vim.api.nvim_buf_is_loaded(buf) then return fail('buffer is not loaded') end
  local snapshot = options(buf)
  if not snapshot.modifiable then return fail('buffer is not modifiable') end
  if snapshot.binary or snapshot.bomb
    or (snapshot.fileencoding ~= '' and snapshot.fileencoding:lower() ~= 'utf-8')
    or (snapshot.fileformat ~= 'unix' and snapshot.fileformat ~= 'dos') then
    return fail('requires a nonbinary UTF-8 Unix or DOS buffer without a BOM')
  end
  local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  -- Public buffer lines cannot distinguish canonical empty source from one LF.
  if #lines == 1 and lines[1] == '' then return true end
  local source = codec.encode(lines, snapshot.fileformat, snapshot.endofline)
  if #source > 262144 then return fail('source exceeds the CLI input byte limit') end
  -- A fresh request token acts as the generation even when changedtick is equal.
  local operation = {}
  pending[buf] = operation
  local tick = vim.api.nvim_buf_get_changedtick(buf)
  local function finish(result)
    if pending[buf] ~= operation then return false, 'formatter request was superseded or canceled' end
    pending[buf] = nil
    if not vim.api.nvim_buf_is_loaded(buf) then return false, 'buffer is no longer loaded' end
    if vim.api.nvim_buf_get_changedtick(buf) ~= tick or not vim.deep_equal(options(buf), snapshot) then
      return fail('discarded stale result because the buffer or its options changed')
    end
    if result.error then return fail(result.error .. (result.stderr ~= '' and ': ' .. result.stderr or '')) end
    if result.code ~= 0 then
      return fail('formatter exited with code ' .. tostring(result.code) .. ': ' .. result.stderr)
    end
    local decoded, err = codec.decode(result.stdout, snapshot.fileformat)
    if not decoded then return fail(err) end
    apply(buf, lines, decoded)
    return true
  end
  local async = opts.async ~= false
  local process, err = transport.start(config, source, async and function(result)
    vim.schedule(function() finish(result) end)
  end or nil)
  if not process then pending[buf] = nil; return fail(err) end
  operation.process = process
  if async then return true end
  return finish(process.wait())
end

function M.setup(opts)
  if vim.fn.has('nvim-0.10') == 0 then error('revofmt requires Neovim >=0.10') end
  local next_config = vim.tbl_extend('force', defaults, opts or {})
  for name, limits in pairs({ indent_width = { 1, 8 }, line_width = { 20, 240 }, timeout_ms = { 1, 2147483647 } }) do
    local value = next_config[name]
    if type(value) ~= 'number' or value % 1 ~= 0 or value < limits[1] or value > limits[2] then
      error('revofmt: invalid ' .. name)
    end
  end
  if type(next_config.executable) ~= 'string' or next_config.executable == '' or next_config.executable:find('\0', 1, true) then
    error('revofmt: executable must be a nonempty path or command name')
  end
  if type(next_config.format_on_save) ~= 'boolean' then error('revofmt: format_on_save must be boolean') end
  config = next_config
  vim.filetype.add({ extension = { rv = 'revo', revo = 'revo' } })
  vim.api.nvim_create_user_command('RevoFormat', function() M.format() end, { desc = 'Format the whole Revo buffer', force = true })
  local group = vim.api.nvim_create_augroup('Revofmt', { clear = true })
  vim.api.nvim_create_autocmd({ 'BufRead', 'BufNewFile' }, {
    group = group, pattern = { '*.rv', '*.revo' },
    callback = function(event)
      if vim.bo[event.buf].filetype == '' then vim.bo[event.buf].filetype = 'revo' end
    end,
  })
  vim.api.nvim_create_autocmd({ 'BufWipeout', 'BufUnload' }, { group = group, callback = function(event) cancel(event.buf) end })
  if config.format_on_save then
    vim.api.nvim_create_autocmd('BufWritePre', {
      group = group,
      callback = function(event)
        if vim.bo[event.buf].filetype == 'revo' then M.format({ bufnr = event.buf, async = false }) end
      end,
    })
  end
  vim.g.loaded_revofmt = true
end

return M
