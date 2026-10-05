const std = @import("std");
const Lexer = @import("../vendor/revo/src/lang/Lexer.zig");
const ast = @import("../vendor/revo/src/lang/ast.zig");
pub const Range = struct { kind: []const u8, start: usize, end: usize };

pub fn tokens(alloc: std.mem.Allocator, source: []const u8, lexed: []const Lexer.Token) ![]const Range {
    var result: std.ArrayList(Range) = .empty;
    var line_starts: std.ArrayList(usize) = .empty;
    try line_starts.append(alloc, 0);
    for (source, 0..) |byte, index| if (byte == '\n') {
        try line_starts.append(alloc, index + 1);
    };
    for (lexed) |token| {
        if (token.type == .eof) continue;
        var start = token.start;
        var end = token.end;
        if (token.type == .doc_comment or token.type == .module_doc) {
            if (token.line == 0 or token.line > line_starts.items.len or token.column == 0) return error.InvalidCommentCoordinates;
            start = std.math.add(usize, line_starts.items[token.line - 1], token.column - 1) catch return error.InvalidCommentCoordinates;
            end = std.math.add(usize, token.end, 2) catch return error.InvalidCommentCoordinates;
            if (start > token.start or token.start > token.end or end > source.len or end - start < 4) return error.InvalidCommentEnvelope;
            const delimiter: u8 = if (token.type == .doc_comment) '*' else '!';
            if (source[start] != '#' or source[start + 1] != delimiter or source[end - 2] != delimiter or source[end - 1] != '#') return error.InvalidCommentEnvelope;
        }
        if (start >= end or end > source.len) return error.InvalidTokenRange;
        try result.append(alloc, .{ .kind = @tagName(token.type), .start = start, .end = end });
    }
    return result.items;
}

pub fn regions(alloc: std.mem.Allocator, source: []const u8, lexed: []const Lexer.Token, root: *ast.Node) ![]const Range {
    var collector: Collector = .{ .alloc = alloc, .source = source, .tokens = lexed };
    // A synthetic module list is a real list of source statements; a grouped
    // import is also synthetic, but its span is only the `import` keyword.
    if (root.expr == .block and root.synthetic_block and !collector.groupedImport(root)) {
        for (root.expr.block) |child| try collector.statement(child);
    } else if (root.expr != .block or !root.synthetic_block) {
        try collector.statement(root);
    }
    return collector.result.items;
}
const Collector = struct {
    alloc: std.mem.Allocator,
    source: []const u8,
    tokens: []const Lexer.Token,
    result: std.ArrayList(Range) = .empty,

    fn add(self: *Collector, kind: []const u8, start: usize, end: usize) !void {
        if (start >= end or end > self.source.len) return;
        // Require lexical endpoints, excluding spans inside decoded strings or
        // rewritten quasiquotes. The traversal also avoids generated block trees.
        var starts = false;
        var ends = false;
        for (self.tokens) |token| {
            starts = starts or token.start == start;
            ends = ends or token.end == end;
        }
        if (starts and ends) try self.result.append(self.alloc, .{ .kind = kind, .start = start, .end = end });
    }
    fn groupedImport(self: *Collector, node: *ast.Node) bool {
        for (self.tokens) |token| if (token.start == node.span.start and token.end == node.span.end and token.type == .kw_import) return true;
        return false;
    }
    fn concreteBlockEnd(self: *Collector, node: *ast.Node) ?usize {
        if (node.synthetic_block) return null;
        var depth: usize = 0;
        var begun = false;
        for (self.tokens) |token| {
            if (token.start < node.span.start) continue;
            if (!begun) {
                if (token.start != node.span.start or token.type != .kw_do) return null;
                begun = true;
            }
            if (token.type == .kw_do) depth += 1;
            if (token.type == .kw_end) {
                depth -= 1;
                if (depth == 0) return if (node.span.end <= token.end) token.end else null;
            }
        }
        return null;
    }
    fn statement(self: *Collector, node: *ast.Node) anyerror!void {
        if (node.expr == .block and self.concreteBlockEnd(node) == null) return;
        try self.add("statement", node.span.start, self.concreteBlockEnd(node) orelse node.span.end);
        try self.visitNode(node);
    }
    fn visitNode(self: *Collector, node_: *ast.Node) anyerror!void {
        if (node_.expr == .quasiquote) return;
        // Interpolation expands to calls; all descendants still live inside the
        // opaque original string, so they cannot supply layout boundaries.
        for (self.tokens) |token| {
            if ((token.type == .string or token.type == .multiline_string or token.type == .backtick_string) and node_.span.start >= token.start and node_.span.start < token.end) return;
        }
        if (node_.expr == .block) {
            const end = self.concreteBlockEnd(node_) orelse return;
            try self.add("block", node_.span.start, end);
            for (node_.expr.block) |child| try self.statement(child);
            return;
        }
        if (node_.expr == .match_expr) {
            const match = node_.expr.match_expr;
            var after = match.subject.span.end;
            // Subjectless matches have a synthetic subject with the match token's span.
            for (match.arms) |arm| {
                for (self.tokens, 0..) |token, index| {
                    if (token.start < after or token.start >= arm.then.span.start or token.type != .bar) continue;
                    if (index + 1 < self.tokens.len) try self.add("match_arm", self.tokens[index + 1].start, arm.then.span.end);
                    break;
                }
                after = arm.then.span.end;
            }
        }
        try self.walk(ast.Expr, node_.expr);
    }
    fn walk(self: *Collector, comptime T: type, value: T) anyerror!void {
        if (T == ast.Span) return;
        if (T == *ast.Node) return self.visitNode(value);
        switch (@typeInfo(T)) {
            .pointer => |info| switch (info.size) {
                .one => try self.walk(info.child, value.*),
                .slice => if (info.child != u8) {
                    for (value) |item| try self.walk(info.child, item);
                },
                else => {},
            },
            .optional => |info| if (value) |inner| {
                try self.walk(info.child, inner);
            },
            .@"struct" => |info| inline for (info.field_names, info.field_types) |name, Field| {
                try self.walk(Field, @field(value, name));
            },
            .@"union" => |info| inline for (info.field_names, info.field_types) |name, Field| {
                if (@as(info.tag_type.?, value) == @field(info.tag_type.?, name)) {
                    try self.walk(Field, @field(value, name));
                    return;
                }
            },
            else => {},
        }
    }
};
