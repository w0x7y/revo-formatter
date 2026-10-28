const std = @import("std");
const Lexer = @import("../vendor/revo/src/lang/Lexer.zig");
const cst = @import("../vendor/revo/src/lang/cst.zig");
pub const Range = struct { kind: []const u8, start: usize, end: usize };

/// The interleaved tape of significant tokens and comment trivia, in source
/// order. A complete quoted string, including its interpolations and any
/// comments inside them, is one opaque range.
pub fn tokens(alloc: std.mem.Allocator, source: []const u8, lexed: Lexer.LexOutput) ![]const Range {
    var significant: std.ArrayList(Range) = .empty;
    var depth: usize = 0;
    var open: Range = undefined;
    for (lexed.tokens) |token| {
        switch (token.type) {
            .eof => continue,
            .string_start, .ml_string_start => {
                if (depth == 0) open = .{ .kind = if (token.type == .string_start) "string" else "multiline_string", .start = token.start, .end = token.end };
                depth += 1;
            },
            .string_end, .ml_string_end => {
                if (depth == 0) return error.InvalidStringRange;
                depth -= 1;
                if (depth == 0) try significant.append(alloc, .{ .kind = open.kind, .start = open.start, .end = token.end });
            },
            else => if (depth == 0) try significant.append(alloc, .{ .kind = @tagName(token.type), .start = token.start, .end = token.end }),
        }
    }
    if (depth != 0) return error.InvalidStringRange;

    var result: std.ArrayList(Range) = .empty;
    var next: usize = 0;
    for (significant.items) |range| {
        while (next < lexed.trivia.len and lexed.trivia[next].start < range.start) : (next += 1) try appendTrivia(alloc, &result, lexed.trivia[next]);
        try result.append(alloc, range);
    }
    while (next < lexed.trivia.len) : (next += 1) try appendTrivia(alloc, &result, lexed.trivia[next]);

    var previous_end: usize = 0;
    for (result.items) |range| {
        if (range.start < previous_end or range.start >= range.end or range.end > source.len) return error.InvalidTokenRange;
        previous_end = range.end;
    }
    return result.items;
}

fn appendTrivia(alloc: std.mem.Allocator, result: *std.ArrayList(Range), token: Lexer.Token) !void {
    // Comments inside an interpolation belong to the opaque string.
    if (result.items.len > 0 and token.start < result.items[result.items.len - 1].end) return;
    try result.append(alloc, .{ .kind = @tagName(token.type), .start = token.start, .end = token.end });
}

/// Layout hints from exact CST token extents. Statements are immediate
/// children of statement lists; blocks are source `do ... end` pairs. Nodes
/// inside opaque strings never produce hints.
pub fn regions(alloc: std.mem.Allocator, tape: []const Range, root: *cst.CstNode) ![]const Range {
    var collector: Collector = .{ .alloc = alloc, .tape = tape };
    try collector.visit(root);
    return collector.result.items;
}

const Extent = struct { start: usize, end: usize };
const Item = union(enum) { node: *cst.CstNode, type: *cst.CstType };

const Collector = struct {
    alloc: std.mem.Allocator,
    tape: []const Range,
    result: std.ArrayList(Range) = .empty,
    pending: std.ArrayList(Item) = .empty,

    fn lowerBound(self: *const Collector, offset: usize) usize {
        var left: usize = 0;
        var right = self.tape.len;
        while (left < right) {
            const mid = left + (right - left) / 2;
            if (self.tape[mid].start < offset) left = mid + 1 else right = mid;
        }
        return left;
    }

    /// Tape index of the token starting exactly at offset.
    fn startIndex(self: *const Collector, offset: usize) ?usize {
        const i = self.lowerBound(offset);
        return if (i < self.tape.len and self.tape[i].start == offset) i else null;
    }

    fn endsToken(self: *const Collector, offset: usize) bool {
        const i = self.lowerBound(offset);
        return i > 0 and self.tape[i - 1].end == offset;
    }

    fn add(self: *Collector, kind: []const u8, start: usize, end: usize) !void {
        // Only exact tape endpoints guide layout; offsets inside strings do not.
        if (start < end and self.startIndex(start) != null and self.endsToken(end))
            try self.result.append(self.alloc, .{ .kind = kind, .start = start, .end = end });
    }

    /// Lexical extent of every token the node owns. Upstream token walks
    /// recurse per node, so queue nodes and types to bound native stack use.
    fn extent(self: *Collector, node: *cst.CstNode) !?Extent {
        self.pending.clearRetainingCapacity();
        try self.pending.append(self.alloc, .{ .node = node });
        var span: ?Extent = null;
        while (self.pending.pop()) |item| switch (item) {
            .node => |value| try self.scan(cst.CstNode, value.*, &span),
            .type => |value| try self.scan(cst.CstType, value.*, &span),
        };
        return span;
    }

    fn scan(self: *Collector, comptime T: type, value: T, span: *?Extent) !void {
        if (T == Lexer.Span) return;
        if (T == Lexer.Token) {
            span.* = if (span.*) |old| .{ .start = @min(old.start, value.start), .end = @max(old.end, value.end) } else .{ .start = value.start, .end = value.end };
            return;
        }
        if (T == *cst.CstNode) return self.pending.append(self.alloc, .{ .node = value });
        if (T == *cst.CstType) return self.pending.append(self.alloc, .{ .type = value });
        switch (@typeInfo(T)) {
            .pointer => |info| switch (info.size) {
                .one => try self.scan(info.child, value.*, span),
                .slice => if (info.child != u8) {
                    for (value) |item| try self.scan(info.child, item, span);
                },
                else => {},
            },
            .optional => |info| if (value) |inner| try self.scan(info.child, inner, span),
            .@"struct" => |info| inline for (info.field_names, info.field_types) |name, Field| {
                try self.scan(Field, @field(value, name), span);
            },
            .@"union" => |info| inline for (info.field_names, info.field_types) |name, Field| {
                if (@as(info.tag_type.?, value) == @field(info.tag_type.?, name)) return self.scan(Field, @field(value, name), span);
            },
            else => {},
        }
    }

    fn statements(self: *Collector, list: []*cst.CstNode) !void {
        for (list) |statement| {
            if (try self.extent(statement)) |span| try self.add("statement", span.start, span.end);
            try self.visit(statement);
        }
    }

    fn visit(self: *Collector, node: *cst.CstNode) anyerror!void {
        switch (node.kind) {
            // Interpolated values are opaque string contents.
            .interpolated => return,
            .root => |value| return self.statements(value.stmts),
            .block => |value| return self.statements(value.stmts),
            .do_block => |value| {
                try self.add("block", value.do_tok.start, value.end_tok.end);
                return self.statements(value.stmts);
            },
            .fn_def => |value| try self.headerBody(node, value.body),
            .fn_method => |value| try self.headerBody(node, value.body),
            .if_expr => |value| try self.headerBody(node, value.then),
            .while_loop => |value| try self.headerBody(node, value.body),
            .for_loop => |value| try self.headerBody(node, value.body),
            .unary => |value| if (value.op == .negate) {
                try self.add("unary_sign", value.op_tok.start, value.op_tok.end);
            },
            .match => |value| try self.match(node, value),
            else => {},
        }
        try self.walk(cst.CstKind, node.kind);
    }

    fn headerBody(self: *Collector, node: *cst.CstNode, body: *cst.CstNode) !void {
        // A named fn's binding owns its keyword, so the header starts at the
        // node span, which still begins at that keyword.
        const span = try self.extent(body) orelse return;
        const start = node.span.start;
        if (start >= span.start or self.startIndex(start) == null or self.startIndex(span.start) == null) return;
        try self.result.append(self.alloc, .{ .kind = "header", .start = start, .end = span.start });
        try self.add("body", span.start, span.end);
    }

    fn match(self: *Collector, node: *cst.CstNode, value: @FieldType(cst.CstKind, "match")) !void {
        const span = try self.extent(node) orelse return;
        try self.add("match_expression", span.start, span.end);
        for (value.bars, value.arms, 0..) |bar, arm, i| {
            const index = self.startIndex(bar.start) orelse continue;
            if (i == 0 and self.startIndex(span.start) != null)
                try self.result.append(self.alloc, .{ .kind = "match_head", .start = span.start, .end = bar.start });
            // The arm begins at the next tape token, which can be a comment.
            if (index + 1 >= self.tape.len) continue;
            const start = self.tape[index + 1].start;
            const then = try self.extent(arm.then) orelse continue;
            try self.add("match_arm", start, then.end);
            try self.add("match_arm_head", start, arm.arrow_tok.end);
        }
    }

    fn walk(self: *Collector, comptime T: type, value: T) anyerror!void {
        if (T == *cst.CstNode) return self.visit(value);
        if (T == Lexer.Token or T == Lexer.Span or T == *cst.CstType) return;
        switch (@typeInfo(T)) {
            .pointer => |info| switch (info.size) {
                .one => try self.walk(info.child, value.*),
                .slice => if (info.child != u8) {
                    for (value) |item| try self.walk(info.child, item);
                },
                else => {},
            },
            .optional => |info| if (value) |inner| try self.walk(info.child, inner),
            .@"struct" => |info| inline for (info.field_names, info.field_types) |name, Field| {
                try self.walk(Field, @field(value, name));
            },
            .@"union" => |info| inline for (info.field_names, info.field_types) |name, Field| {
                if (@as(info.tag_type.?, value) == @field(info.tag_type.?, name)) return self.walk(Field, @field(value, name));
            },
            else => {},
        }
    }
};

fn analyzeForTest(alloc: std.mem.Allocator, source: []const u8) !struct { tape: []const Range, regions: []const Range } {
    const Parser = @import("../vendor/revo/src/lang/Parser.zig");
    const parsed = try Parser.parseSourceReport(alloc, source, .{});
    const tape = try tokens(alloc, source, try Lexer.lexAll(alloc, source));
    return .{ .tape = tape, .regions = try regions(alloc, tape, parsed.ok) };
}

test "bridge: strings, interpolations and their comments are one opaque tape token" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const source = "## a ##\nlet x = \"p #{f(\"q #{1}\") ## c ##} r\" # tail\nlet y = \"\"\"\n  #{y}\n  \"\"\" 'raw'";
    const result = try analyzeForTest(arena.allocator(), source);
    const expected = [_][]const u8{ "comment", "kw_let", "ident", "assign", "string", "comment", "kw_let", "ident", "assign", "multiline_string", "string" };
    try std.testing.expectEqual(expected.len, result.tape.len);
    for (expected, result.tape) |kind, range| try std.testing.expectEqualStrings(kind, range.kind);
    try std.testing.expectEqualStrings("\"p #{f(\"q #{1}\") ## c ##} r\"", source[result.tape[4].start..result.tape[4].end]);
    for (result.regions) |region| {
        try std.testing.expect(region.start < result.tape[4].start or region.start >= result.tape[4].end);
    }
}

test "bridge: blocks, statements and match hints use exact token extents" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const source = "#* doc *#\nfn f(x) do\n  x\nend\nmatch y | ## c ## 1 => 2 | _ => -3";
    const result = try analyzeForTest(arena.allocator(), source);
    const expected = [_]struct { []const u8, []const u8 }{
        .{ "statement", "#* doc *#\nfn f(x) do\n  x\nend" },
        .{ "header", "fn f(x) " },
        .{ "body", "do\n  x\nend" },
        .{ "block", "do\n  x\nend" },
        .{ "statement", "x" },
        .{ "statement", "match y | ## c ## 1 => 2 | _ => -3" },
        .{ "match_expression", "match y | ## c ## 1 => 2 | _ => -3" },
        .{ "match_head", "match y " },
        .{ "match_arm", "## c ## 1 => 2" },
        .{ "match_arm_head", "## c ## 1 =>" },
        .{ "match_arm", "_ => -3" },
        .{ "match_arm_head", "_ =>" },
        .{ "unary_sign", "-" },
    };
    try std.testing.expectEqual(expected.len, result.regions.len);
    for (expected, result.regions) |want, region| {
        try std.testing.expectEqualStrings(want[0], region.kind);
        try std.testing.expectEqualStrings(want[1], source[region.start..region.end]);
    }
}
