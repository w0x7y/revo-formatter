local M = {}

function M.encode(lines, fileformat, endofline)
  local delimiter = fileformat == 'dos' and '\r\n' or '\n'
  return table.concat(lines, delimiter) .. (endofline and delimiter or '')
end

function M.decode(source, fileformat)
  if source:find('\0', 1, true) then
    return nil, 'formatter output contains NUL bytes that this buffer cannot represent'
  end
  local lines, start = {}, 1
  local endofline = source:sub(-1) == '\n'
  while true do
    local newline = source:find('\n', start, true)
    if not newline then break end
    local line = source:sub(start, newline - 1)
    -- Remove only a DOS delimiter's CR. Any earlier CR belongs to the source.
    if fileformat == 'dos' and line:sub(-1) == '\r' then line = line:sub(1, -2) end
    lines[#lines + 1] = line
    start = newline + 1
  end
  if not endofline then lines[#lines + 1] = source:sub(start) end
  if #lines == 0 then lines = { '' } end
  if M.encode(lines, fileformat, endofline) ~= source then
    return nil, 'formatter output cannot be represented exactly with this buffer fileformat'
  end
  return { lines = lines, endofline = endofline }
end

return M
