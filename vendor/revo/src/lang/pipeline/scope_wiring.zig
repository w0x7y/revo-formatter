const std = @import("std");

const ast = @import("../ast.zig");
const Node = ast.Node;

/// build @exports[:name] = name
fn buildSetExport(alloc: std.mem.Allocator, span: ast.Span, name: []const u8) !*Node {
    const exports_ref = try ast.allocNode(alloc, span, .{ .ident = "@exports" });
    const key = try ast.allocNode(alloc, span, .{ .atom = name });
    const index = try ast.allocNode(alloc, span, .{ .index = .{ .object = exports_ref, .key = key } });
    const value = try ast.allocNode(alloc, span, .{ .ident = name });
    return ast.allocNode(alloc, span, .{ .assign_expr = .{ .target = index, .value = value } });
}

/// extract exported name from a pub decl or import_stmt
fn pubName(item: *Node) ?[]const u8 {
    return switch (item.expr) {
        .decl => |d| if (d.pub_) switch (d.inner.expr) {
            .binding => |b| if (b.target.expr == .ident) b.target.expr.ident else null,
            // type aliases are compile-time only, not runtime values
            // so they cannot be exported in the runtime exports table
            else => null,
        } else null,
        .import_stmt => |is| if (is.pub_) is.name else null,
        else => null,
    };
}

/// copy a node without its pub_ flag, leaves non-pub nodes unchanged
fn clearPub(item: *Node, alloc: std.mem.Allocator) !*Node {
    return switch (item.expr) {
        .decl => |d| ast.allocNode(alloc, item.span, .{ .decl = .{
            .inner = d.inner,
            .kind = d.kind,
            .pub_ = false,
        } }),
        .import_stmt => |is| ast.allocNode(alloc, item.span, .{ .import_stmt = .{
            .name = is.name,
            .path = is.path,
            .pub_ = false,
        } }),
        else => item,
    };
}

/// wrap AST for module scope: build exports table from pub decls
/// only top-level decls export; a pub nested in a bare block has no
/// scope to export from, so it stays local and unexported
pub fn wrapModule(alloc: std.mem.Allocator, root: *Node) !*Node {
    const is_single = root.expr != .block;
    const items: []const *Node = if (is_single) &[_]*Node{root} else root.expr.block;

    var has_pub = false;
    for (items) |item| {
        if (pubName(item) != null) {
            has_pub = true;
            break;
        }
    }
    if (!has_pub) return root;

    const span = root.span;

    // const @exports = {}
    const exports_ident = try ast.allocNode(alloc, span, .{ .ident = "@exports" });
    const exports_table = try ast.allocNode(alloc, span, .{ .table = &.{} });
    const exports_binding = try ast.allocNode(alloc, span, .{ .binding = .{
        .target = exports_ident,
        .value = exports_table,
    } });
    const exports_decl = try ast.allocNode(alloc, span, .{ .decl = .{
        .inner = exports_binding,
        .kind = .@"const",
    } });

    var new_items = try std.ArrayList(*Node).initCapacity(alloc, items.len * 2 + 2); // upper bound: each item + export
    try new_items.append(alloc, exports_decl);

    for (items) |item| {
        if (pubName(item)) |name| {
            try new_items.append(alloc, try clearPub(item, alloc));
            try new_items.append(alloc, try buildSetExport(alloc, span, name));
        } else {
            try new_items.append(alloc, item);
        }
    }

    const final_exports = try ast.allocNode(alloc, span, .{ .ident = "@exports" });
    try new_items.append(alloc, final_exports);

    const result = try ast.allocNode(alloc, span, .{ .block = try new_items.toOwnedSlice(alloc) });
    result.synthetic_block = true;
    return result;
}

/// any fn_expr whose body contains pub decls gets its body wrapped
/// so calling the closure returns @exports - fn as module
const PubWrapCtx = struct {
    pub fn walk(_: PubWrapCtx, alloc: std.mem.Allocator, node: *Node, ctx: PubWrapCtx) std.mem.Allocator.Error!*Node {
        const walked = try ast.walkExpr(alloc, node, PubWrapCtx, ctx);
        if (walked.expr == .fn_expr) {
            const f = &walked.expr.fn_expr;
            if (bodyHasPub(f.body)) f.body = try wrapModule(alloc, f.body);
        }
        return walked;
    }
};

pub fn wrapPubFunctions(alloc: std.mem.Allocator, node: *Node) !*Node {
    return PubWrapCtx.walk(.{}, alloc, node, .{});
}

fn bodyHasPub(node: *Node) bool {
    return switch (node.expr) {
        .block => |items| for (items) |item| {
            if (pubName(item) != null) break true;
        } else false,
        else => pubName(node) != null,
    };
}
