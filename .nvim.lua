-- Project config, run by ~/.config/nvim/init.lua as a function with arguments (plugin, lsp).
local _plugin, lsp = ...

-- rust-analyzer for the whole Cargo workspace: Cargo.lock exists only at the workspace root,
-- so every crate (peggen, peggen-core, peggen-macs, peggen-examples) shares one client.
lsp("rust_analyzer", {
    cmd = { "rust-analyzer" },
    filetypes = { "rust" },
    root_markers = { "Cargo.lock", ".git" },
    settings = {
        ["rust-analyzer"] = {
            -- expand #[peggen] so the generated Match / Choice / Build impls resolve
            procMacro = { enable = true },
        },
    },
})
