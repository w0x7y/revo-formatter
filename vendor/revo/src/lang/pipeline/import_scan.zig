const std = @import("std");

const revo = @import("revo");
const VM = revo.VM;

const ast = @import("../ast.zig");
const Node = ast.Node;
const Parser = @import("../Parser.zig");

pub const ImportCache = struct {
    sources: std.StringHashMap([]const u8),

    pub fn init(alloc: std.mem.Allocator) ImportCache {
        return .{ .sources = std.StringHashMap([]const u8).init(alloc) };
    }

    pub fn lookup(self: *const ImportCache, resolved: []const u8) ?[]const u8 {
        return self.sources.get(resolved);
    }
};

/// walk AST and pre-load imported modules (best-effort, OOM propagates, others
/// are deferred to runtime where the import native fn handles them)
pub fn preloadImports(vm: *VM, root: *Node, alloc: std.mem.Allocator, cache: *ImportCache) !void {
    var inject_nodes = try std.ArrayList(*Node).initCapacity(alloc, 8);
    defer inject_nodes.deinit(alloc);

    var visited = std.StringHashMap(void).init(alloc);
    defer visited.deinit();

    // separate visited for submod macro extraction, keyed by qualified prefix + path
    // so that re-exports through different parents both get extracted
    var visited_sub = std.StringHashMap(void).init(alloc);
    defer visited_sub.deinit();

    try walkAndProcessImports(vm, root, alloc, &inject_nodes, &visited, &visited_sub, cache);

    if (inject_nodes.items.len > 0 and root.expr == .block) {
        const items = root.expr.block;
        var new_items = try std.ArrayList(*Node).initCapacity(alloc, items.len + inject_nodes.items.len);
        for (inject_nodes.items) |n| new_items.appendAssumeCapacity(n);
        for (items) |item| new_items.appendAssumeCapacity(item);
        root.expr.block = try new_items.toOwnedSlice(alloc);
    }
}

fn walkAndProcessImports(
    vm: *VM,
    node: *Node,
    alloc: std.mem.Allocator,
    inject_nodes: *std.ArrayList(*Node),
    visited: *std.StringHashMap(void),
    visited_sub: *std.StringHashMap(void),
    cache: *ImportCache,
) !void {
    var visitor = ImportWalkVisitor{
        .vm = vm,
        .alloc = alloc,
        .inject_nodes = inject_nodes,
        .visited = visited,
        .visited_sub = visited_sub,
        .cache = cache,
    };
    visitor.visit(node);
    if (visitor.failed) |e| return e;
}

/// walkAST visitor matching the old hand recursion exactly
///   only block|decl|binding recurse, so imports under if/match/fn stay missed
///   first error aborts like propagation did, visit just cannot return it
const ImportWalkVisitor = struct {
    vm: *VM,
    alloc: std.mem.Allocator,
    inject_nodes: *std.ArrayList(*Node),
    visited: *std.StringHashMap(void),
    visited_sub: *std.StringHashMap(void),
    cache: *ImportCache,
    failed: ?anyerror = null,

    pub fn visit(self: *@This(), node: *const Node) void {
        if (self.failed != null) return;
        switch (node.expr) {
            .import_stmt => |stmt| processImport(
                self.vm,
                stmt.path,
                stmt.name,
                self.alloc,
                self.inject_nodes,
                self.visited,
                self.visited_sub,
                self.cache,
            ) catch |e| {
                if (self.failed == null) self.failed = e;
            },
            .block, .decl, .binding => ast.walkAST(@This(), self, node),
            else => {},
        }
    }
};

/// resolve module path matching runtime import resolution
pub fn resolveModuleFile(vm: *VM, name: []const u8) !?[]const u8 {
    return revo.resolveImportFile(
        vm.runtime.io,
        vm.runtime.alloc,
        name,
        vm.import_dir,
        vm.project_root,
        vm.package_path.items,
    );
}

pub fn resolveModuleText(vm: *VM, cache: *ImportCache, path: []const u8, alloc: std.mem.Allocator) !?[]const u8 {
    const resolved = try resolveModuleFile(vm, path) orelse return null;
    defer vm.runtime.alloc.free(resolved);

    if (cache.lookup(resolved)) |hit| return hit;

    const source = std.Io.Dir.cwd().readFileAlloc(
        vm.runtime.io,
        resolved,
        alloc,
        std.Io.Limit.unlimited,
    ) catch |err| switch (err) {
        error.OutOfMemory => return err,
        else => return null,
    };

    try cache.sources.put(try alloc.dupe(u8, resolved), source);

    return source;
}

/// strict cached read off a resolved path, borrowed from arena
fn readResolvedCached(vm: *VM, cache: *ImportCache, resolved: []const u8, alloc: std.mem.Allocator) ![]const u8 {
    if (cache.lookup(resolved)) |hit| return hit;

    const source = try std.Io.Dir.cwd().readFileAlloc(
        vm.runtime.io,
        resolved,
        alloc,
        std.Io.Limit.unlimited,
    );
    try cache.sources.put(try alloc.dupe(u8, resolved), source);

    return source;
}

/// read, parse, and extract macros/procs from a module for compile-time use
/// does NOT compile or cache the module!!! runtime `import` handles that!
/// extraction populates the expander env with qualified names (mod_name.macro!)
fn processImport(
    vm: *VM,
    path: []const u8,
    mod_name: []const u8,
    alloc: std.mem.Allocator,
    inject_nodes: *std.ArrayList(*Node),
    visited: *std.StringHashMap(void),
    visited_sub: *std.StringHashMap(void),
    cache: *ImportCache,
) !void {
    if (visited.contains(path)) return;
    try visited.put(path, {});

    // non-OOM errors are deferred to runtime,,, preload is best-effort
    const source = (try resolveModuleText(vm, cache, path, alloc)) orelse return;

    const module_ast = Parser.parseSource(alloc, source, .{}) catch return;

    extractPubDefs(module_ast, mod_name, alloc, inject_nodes) catch return;
    extractPubImportsOneLevel(vm, module_ast, mod_name, alloc, inject_nodes, visited_sub, cache) catch return;
}

/// extract pub macros and procs from a module AST, qualified with prefix
/// injects them as named nodes into out for the parent scope
fn extractPubDefs(node: *Node, prefix: []const u8, alloc: std.mem.Allocator, out: *std.ArrayList(*Node)) !void {
    switch (node.expr) {
        .block => |items| {
            for (items) |item| try extractPubDefs(item, prefix, alloc, out);
        },
        .decl => |d| {
            if (d.pub_) {
                switch (d.inner.expr) {
                    .proc_macro => |pm| {
                        if (std.mem.endsWith(u8, pm.name, "!")) {
                            const qualified = try alloc.print( "{s}.{s}", .{ prefix, ast.bareMacroName(pm.name) });
                            const proc_node = try ast.allocNode(alloc, d.inner.span, .{ .proc_macro = .{
                                .name = qualified,
                                .param = .{ .name = pm.param.name, .name_span = pm.param.name_span },
                                .body = pm.body,
                            } });
                            try out.append(alloc, proc_node);
                        }
                    },
                    .type_alias => |t| {
                        const ta_node = try ast.allocNode(alloc, d.inner.span, .{
                            .type_alias = .{ .name = t.name, .name_span = t.name_span, .type_expr = t.type_expr },
                        });
                        try out.append(alloc, ta_node);
                    },
                    else => {},
                }
            }
            try extractPubDefs(d.inner, prefix, alloc, out);
        },
        else => {},
    }
}

/// extract one level of pub imports;;; loads submods and extracts their macros
/// but does NOT recurse into submod's own pub imports (breaks the inference cycle)
fn extractPubImportsOneLevel(
    vm: *VM,
    node: *Node,
    prefix: []const u8,
    alloc: std.mem.Allocator,
    inject_nodes: *std.ArrayList(*Node),
    visited_sub: *std.StringHashMap(void),
    cache: *ImportCache,
) !void {
    switch (node.expr) {
        .block => |items| {
            for (items) |item| try extractPubImportsOneLevel(vm, item, prefix, alloc, inject_nodes, visited_sub, cache);
        },
        .import_stmt => |stmt| {
            if (stmt.pub_) {
                // key by qualified prefix + path so different parents with same sub-path
                // both get their macros extracted
                const dedup_key = try alloc.print( "{s}.{s}.{s}", .{ prefix, stmt.name, stmt.path });
                defer alloc.free(dedup_key);
                if (visited_sub.contains(dedup_key)) return;
                try visited_sub.put(dedup_key, {});

                const sub_prefix = try alloc.print( "{s}.{s}", .{ prefix, stmt.name });
                defer alloc.free(sub_prefix);

                const resolved = try resolveModuleFile(vm, stmt.path) orelse return;
                defer vm.runtime.alloc.free(resolved);

                const source = try readResolvedCached(vm, cache, resolved, alloc);

                const sub_ast = try Parser.parseSource(alloc, source, .{});
                try extractPubDefs(sub_ast, sub_prefix, alloc, inject_nodes);
            }
        },
        .decl => |d| try extractPubImportsOneLevel(vm, d.inner, prefix, alloc, inject_nodes, visited_sub, cache),
        else => {},
    }
}
