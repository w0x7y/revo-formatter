const std = @import("std");
const Lexer = @import("../vendor/revo/src/lang/Lexer.zig");
const cst = @import("../vendor/revo/src/lang/cst.zig");

const Pair = union(enum) {
    node: [2]*const cst.CstNode,
    type: [2]*const cst.CstType,
};

/// Exhaustive by type: a new unsupported field type is a compile error.
/// Spans are excluded, and tokens compare only their type and text: their
/// offsets, lines, columns, whitespace flags and trivia indices are source
/// coordinates. Pointers are compared by contents. Nodes and types are queued
/// rather than recursed into, so tree depth never grows the native stack.
pub fn equal(alloc: std.mem.Allocator, comptime T: type, a: T, b: T) !bool {
    var pending: std.ArrayList(Pair) = .empty;
    defer pending.deinit(alloc);
    if (!try shallow(alloc, &pending, T, a, b)) return false;
    while (pending.pop()) |pair| {
        const same = switch (pair) {
            .node => |nodes| try shallow(alloc, &pending, cst.CstNode, nodes[0].*, nodes[1].*),
            .type => |types| try shallow(alloc, &pending, cst.CstType, types[0].*, types[1].*),
        };
        if (!same) return false;
    }
    return true;
}

fn shallow(alloc: std.mem.Allocator, pending: *std.ArrayList(Pair), comptime T: type, a: T, b: T) !bool {
    if (T == Lexer.Span) return true;
    if (T == Lexer.Token) return a.type == b.type and std.mem.eql(u8, a.text, b.text);
    if (T == *cst.CstNode or T == *const cst.CstNode) {
        try pending.append(alloc, .{ .node = .{ a, b } });
        return true;
    }
    if (T == *cst.CstType or T == *const cst.CstType) {
        try pending.append(alloc, .{ .type = .{ a, b } });
        return true;
    }
    switch (@typeInfo(T)) {
        .void => return true,
        .bool, .int, .@"enum" => return a == b,
        .float => return @as(@Int(.unsigned, @bitSizeOf(T)), @bitCast(a)) == @as(@Int(.unsigned, @bitSizeOf(T)), @bitCast(b)),
        .optional => |info| {
            if (a) |av| return if (b) |bv| shallow(alloc, pending, info.child, av, bv) else false;
            return b == null;
        },
        .pointer => |info| switch (info.size) {
            .one => return shallow(alloc, pending, info.child, a.*, b.*),
            .slice => {
                if (a.len != b.len) return false;
                for (a, b) |av, bv| if (!try shallow(alloc, pending, info.child, av, bv)) return false;
                return true;
            },
            else => @compileError("unsupported CST pointer " ++ @typeName(T)),
        },
        .array => |info| {
            for (a, b) |av, bv| if (!try shallow(alloc, pending, info.child, av, bv)) return false;
            return true;
        },
        .@"struct" => |info| {
            inline for (info.field_names, info.field_types) |name, Field| if (!try shallow(alloc, pending, Field, @field(a, name), @field(b, name))) return false;
            return true;
        },
        .@"union" => |info| {
            const Tag = info.tag_type orelse @compileError("untagged CST union");
            if (@as(Tag, a) != @as(Tag, b)) return false;
            inline for (info.field_names, info.field_types) |name, Field| {
                if (@as(Tag, a) == @field(Tag, name)) return shallow(alloc, pending, Field, @field(a, name), @field(b, name));
            }
            unreachable;
        },
        else => @compileError("unsupported CST field " ++ @typeName(T)),
    }
}

test "bridge: comparison excludes spans and token coordinates only" {
    const span = Lexer.Span{ .start = 0, .end = 1, .line = 1, .column = 1 };
    const token = Lexer.Token{ .type = .number, .text = "1", .line = 1, .column = 1, .start = 0, .end = 1 };
    var a = cst.CstNode{ .span = span, .kind = .{ .number = .{ .tok = token, .value = 1, .is_float = false } } };
    var b = a;
    b.span = .{ .start = 99, .end = 100, .line = 5, .column = 6 };
    b.kind.number.tok = .{ .type = .number, .text = "1", .line = 5, .column = 6, .start = 99, .end = 100, .space_before = true, .nl_before = true, .newlines_before = 2, .trivia_start = 3, .trivia_len = 1 };
    try std.testing.expect(try equal(std.testing.allocator, *cst.CstNode, &a, &b));
    b.kind.number.tok.text = "1.0";
    try std.testing.expect(!try equal(std.testing.allocator, *cst.CstNode, &a, &b));
    b.kind.number.tok.text = "1";
    b.kind.number.is_float = true;
    try std.testing.expect(!try equal(std.testing.allocator, *cst.CstNode, &a, &b));
    const Other = struct { span: usize };
    try std.testing.expect(!try equal(std.testing.allocator, Other, .{ .span = 1 }, .{ .span = 2 }));
}

test "bridge: float bits and pointer contents are preserved" {
    try std.testing.expect(!try equal(std.testing.allocator, f64, 0.0, -0.0));
    const nan: f64 = @bitCast(@as(u64, 0x7ff8000000000001));
    try std.testing.expect(try equal(std.testing.allocator, f64, nan, nan));
    try std.testing.expect(!try equal(std.testing.allocator, f64, nan, @bitCast(@as(u64, 0x7ff8000000000002))));
    var first = [_]u8{ 1, 2 };
    var second = [_]u8{ 1, 2 };
    try std.testing.expect(try equal(std.testing.allocator, []const u8, &first, &second));
    second[1] = 3;
    try std.testing.expect(!try equal(std.testing.allocator, []const u8, &first, &second));
}
