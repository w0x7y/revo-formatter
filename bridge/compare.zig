const std = @import("std");
const ast = @import("../vendor/revo/src/lang/ast.zig");

/// Exhaustive by type: a new unsupported field type is a compile error.
/// Span is the only excluded type; pointers are compared by contents.
pub fn equal(comptime T: type, a: T, b: T) bool {
    if (T == ast.Span) return true;
    switch (@typeInfo(T)) {
        .void => return true,
        .bool, .int, .@"enum" => return a == b,
        .float => return @as(@Int(.unsigned, @bitSizeOf(T)), @bitCast(a)) == @as(@Int(.unsigned, @bitSizeOf(T)), @bitCast(b)),
        .optional => |info| {
            if (a) |av| return if (b) |bv| equal(info.child, av, bv) else false;
            return b == null;
        },
        .pointer => |info| switch (info.size) {
            .one => return equal(info.child, a.*, b.*),
            .slice => {
                if (a.len != b.len) return false;
                for (a, b) |av, bv| if (!equal(info.child, av, bv)) return false;
                return true;
            },
            else => @compileError("unsupported AST pointer " ++ @typeName(T)),
        },
        .array => |info| {
            for (a, b) |av, bv| if (!equal(info.child, av, bv)) return false;
            return true;
        },
        .@"struct" => |info| {
            inline for (info.field_names, info.field_types) |name, Field| if (!equal(Field, @field(a, name), @field(b, name))) return false;
            return true;
        },
        .@"union" => |info| {
            const Tag = info.tag_type orelse @compileError("untagged AST union");
            if (@as(Tag, a) != @as(Tag, b)) return false;
            inline for (info.field_names, info.field_types) |name, Field| {
                if (@as(Tag, a) == @field(Tag, name)) return equal(Field, @field(a, name), @field(b, name));
            }
            unreachable;
        },
        else => @compileError("unsupported AST field " ++ @typeName(T)),
    }
}

test "bridge: comparison excludes only exact Span and retains synthetic block" {
    const span = ast.Span{ .start = 0, .end = 1, .line = 1, .column = 1 };
    var a = ast.Node{ .span = span, .expr = .{ .number = .{ .value = 1 } } };
    var b = a;
    b.span = .{ .start = 99, .end = 100, .line = 5, .column = 6 };
    try std.testing.expect(equal(*ast.Node, &a, &b));
    b.synthetic_block = true;
    try std.testing.expect(!equal(*ast.Node, &a, &b));
    const Other = struct { span: usize };
    try std.testing.expect(!equal(Other, .{ .span = 1 }, .{ .span = 2 }));
}

test "bridge: float bits and pointer contents are preserved" {
    try std.testing.expect(!equal(f64, 0.0, -0.0));
    const nan: f64 = @bitCast(@as(u64, 0x7ff8000000000001));
    try std.testing.expect(equal(f64, nan, nan));
    try std.testing.expect(!equal(f64, nan, @bitCast(@as(u64, 0x7ff8000000000002))));
    var first = [_]u8{ 1, 2 };
    var second = [_]u8{ 1, 2 };
    try std.testing.expect(equal([]const u8, &first, &second));
    second[1] = 3;
    try std.testing.expect(!equal([]const u8, &first, &second));
}
