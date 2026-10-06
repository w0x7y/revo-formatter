if vim.g.loaded_revofmt then return end
vim.g.loaded_revofmt = true
require('revofmt').setup()
