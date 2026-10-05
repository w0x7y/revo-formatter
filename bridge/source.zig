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
    var collector: Collector = .{ .alloc = alloc, .source = source, .index = try TokenIndex.init(alloc, lexed) };
    // A synthetic module list is a real list of source statements; a grouped
    // import is also synthetic, but its span is only the `import` keyword.
    if (root.expr == .block and root.synthetic_block and !collector.groupedImport(root)) {
        for (root.expr.block) |child| try collector.statement(child);
    } else if (root.expr != .block or !root.synthetic_block) {
        try collector.statement(root);
    }
    return collector.result.items;
}
// Raw lexer coordinates are authoritative for AST endpoints. In particular,
// doc-comment spans here must not be expanded to the output token envelopes.
const TokenIndex = struct {
    const Entry = struct { offset: usize, token_index: usize };

    lexed: []const Lexer.Token,
    starts: []const Entry,
    ends: []const Entry,
    bars: []const Entry,
    opaque_starts: []const Entry,
    opaque_max_ends: []const usize,
    block_ends: []const ?usize,

    fn lessThan(_: void, a: Entry, b: Entry) bool {
        return a.offset < b.offset or (a.offset == b.offset and a.token_index < b.token_index);
    }

    fn init(alloc: std.mem.Allocator, lexed: []const Lexer.Token) !TokenIndex {
        const starts = try alloc.alloc(Entry, lexed.len);
        const ends = try alloc.alloc(Entry, lexed.len);
        const block_ends = try alloc.alloc(?usize, lexed.len);
        @memset(block_ends, null);
        var open_blocks: std.ArrayList(usize) = .empty;
        for (lexed, 0..) |token, i| {
            starts[i] = .{ .offset = token.start, .token_index = i };
            ends[i] = .{ .offset = token.end, .token_index = i };
            switch (token.type) {
                .kw_do => try open_blocks.append(alloc, i),
                .kw_end => if (open_blocks.pop()) |opener| {
                    block_ends[opener] = token.end;
                },
                else => {},
            }
        }
        std.mem.sort(Entry, starts, {}, lessThan);
        std.mem.sort(Entry, ends, {}, lessThan);
        var bars: std.ArrayList(Entry) = .empty;
        var opaque_starts: std.ArrayList(Entry) = .empty;
        var opaque_max_ends: std.ArrayList(usize) = .empty;
        var max_end: usize = 0;
        for (starts) |entry| {
            const token = lexed[entry.token_index];
            if (token.type == .bar) try bars.append(alloc, entry);
            switch (token.type) {
                .string, .multiline_string, .backtick_string => {
                    try opaque_starts.append(alloc, entry);
                    max_end = @max(max_end, token.end);
                    try opaque_max_ends.append(alloc, max_end);
                },
                else => {},
            }
        }
        return .{
            .lexed = lexed,
            .starts = starts,
            .ends = ends,
            .bars = bars.items,
            .opaque_starts = opaque_starts.items,
            .opaque_max_ends = opaque_max_ends.items,
            .block_ends = block_ends,
        };
    }

    fn lowerBound(entries: []const Entry, offset: usize) usize {
        var left: usize = 0;
        var right = entries.len;
        while (left < right) {
            const mid = left + (right - left) / 2;
            if (entries[mid].offset < offset) left = mid + 1 else right = mid;
        }
        return left;
    }

    fn exact(entries: []const Entry, offset: usize) bool {
        const i = lowerBound(entries, offset);
        return i < entries.len and entries[i].offset == offset;
    }

    fn groupedImport(self: *const TokenIndex, node: *ast.Node) bool {
        var i = lowerBound(self.starts, node.span.start);
        // Duplicate starts and empty tokens can share a lexical endpoint.
        while (i < self.starts.len and self.starts[i].offset == node.span.start) : (i += 1) {
            const token = self.lexed[self.starts[i].token_index];
            if (token.end == node.span.end and token.type == .kw_import) return true;
        }
        return false;
    }

    fn concreteBlockEnd(self: *const TokenIndex, node: *ast.Node) ?usize {
        if (node.synthetic_block) return null;
        const i = lowerBound(self.starts, node.span.start);
        if (i == self.starts.len or self.starts[i].offset != node.span.start) return null;
        const token_index = self.starts[i].token_index;
        if (self.lexed[token_index].type != .kw_do) return null;
        const end = self.block_ends[token_index] orelse return null;
        return if (node.span.end <= end) end else null;
    }

    fn insideOpaque(self: *const TokenIndex, offset: usize) bool {
        // Find the last start <= offset without offset + 1 overflowing at EOF.
        var left: usize = 0;
        var right = self.opaque_starts.len;
        while (left < right) {
            const mid = left + (right - left) / 2;
            if (self.opaque_starts[mid].offset <= offset) left = mid + 1 else right = mid;
        }
        // Prefix maxima retain containment even for overlapping raw spans.
        return left > 0 and offset < self.opaque_max_ends[left - 1];
    }

    fn armBar(self: *const TokenIndex, after: usize, before: usize) ?usize {
        const i = lowerBound(self.bars, after);
        if (i == self.bars.len or self.bars[i].offset >= before) return null;
        return self.bars[i].token_index;
    }
};

const Collector = struct {
    alloc: std.mem.Allocator,
    source: []const u8,
    index: TokenIndex,
    result: std.ArrayList(Range) = .empty,

    fn add(self: *Collector, kind: []const u8, start: usize, end: usize) !void {
        if (start >= end or end > self.source.len) return;
        // Require lexical endpoints, excluding spans inside decoded strings or
        // rewritten quasiquotes. The traversal also avoids generated block trees.
        if (TokenIndex.exact(self.index.starts, start) and TokenIndex.exact(self.index.ends, end))
            try self.result.append(self.alloc, .{ .kind = kind, .start = start, .end = end });
    }
    fn groupedImport(self: *Collector, node: *ast.Node) bool {
        return self.index.groupedImport(node);
    }
    fn statement(self: *Collector, node: *ast.Node) anyerror!void {
        const block_end = self.index.concreteBlockEnd(node);
        if (node.expr == .block and block_end == null) return;
        try self.add("statement", node.span.start, block_end orelse node.span.end);
        try self.visitNode(node, block_end);
    }
    fn visitNode(self: *Collector, node_: *ast.Node, block_end: ?usize) anyerror!void {
        if (node_.expr == .quasiquote) return;
        // Interpolation expands to calls; all descendants still live inside the
        // opaque original string, so they cannot supply layout boundaries.
        if (self.index.insideOpaque(node_.span.start)) return;
        if (node_.expr == .block) {
            const end = block_end orelse return;
            try self.add("block", node_.span.start, end);
            for (node_.expr.block) |child| try self.statement(child);
            return;
        }
        if (node_.expr == .match_expr) {
            const match = node_.expr.match_expr;
            var after = match.subject.span.end;
            // Subjectless matches have a synthetic subject with the match token's span.
            for (match.arms) |arm| {
                if (self.index.armBar(after, arm.then.span.start)) |index| {
                    if (index + 1 < self.index.lexed.len)
                        try self.add("match_arm", self.index.lexed[index + 1].start, arm.then.span.end);
                }
                after = arm.then.span.end;
            }
        }
        try self.walk(ast.Expr, node_.expr);
    }
    fn walk(self: *Collector, comptime T: type, value: T) anyerror!void {
        if (T == ast.Span) return;
        if (T == *ast.Node) return self.visitNode(value, self.index.concreteBlockEnd(value));
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

test "indexed endpoints and opaque containment retain overlapping empty and EOF spans" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const lexed = [_]Lexer.Token{
        .{ .type = .string, .text = "", .line = 1, .column = 1, .start = 0, .end = 12 },
        .{ .type = .multiline_string, .text = "", .line = 1, .column = 3, .start = 2, .end = 4 },
        .{ .type = .backtick_string, .text = "", .line = 1, .column = 3, .start = 2, .end = 8 },
        .{ .type = .ident, .text = "", .line = 1, .column = 5, .start = 4, .end = 4 },
        .{ .type = .string, .text = "", .line = 1, .column = 16, .start = 15, .end = 15 },
        .{ .type = .eof, .text = "", .line = 1, .column = 21, .start = 20, .end = 20 },
    };
    const index = try TokenIndex.init(arena.allocator(), &lexed);
    for (0..22) |offset| {
        var starts = false;
        var ends = false;
        var inside = false;
        for (lexed) |token| {
            starts = starts or token.start == offset;
            ends = ends or token.end == offset;
            inside = inside or ((token.type == .string or token.type == .multiline_string or token.type == .backtick_string) and token.start <= offset and offset < token.end);
        }
        try std.testing.expectEqual(starts, TokenIndex.exact(index.starts, offset));
        try std.testing.expectEqual(ends, TokenIndex.exact(index.ends, offset));
        try std.testing.expectEqual(inside, index.insideOpaque(offset));
    }
    try std.testing.expect(!index.insideOpaque(std.math.maxInt(usize)));
    var collector: Collector = .{ .alloc = arena.allocator(), .source = "01234567890123456789", .index = index };
    try collector.add("statement", 4, 20);
    try collector.add("statement", 4, 4);
    try collector.add("statement", 4, 21);
    try std.testing.expectEqual(@as(usize, 1), collector.result.items.len);
    try std.testing.expectEqual(@as(usize, 20), collector.result.items[0].end);
    const empty = try TokenIndex.init(arena.allocator(), &.{});
    try std.testing.expect(!TokenIndex.exact(empty.starts, 0));
    try std.testing.expect(!TokenIndex.exact(empty.ends, 0));
    try std.testing.expect(!empty.insideOpaque(0));
    try std.testing.expectEqual(@as(?usize, null), empty.armBar(0, 1));
}

test "indexed bars keep inclusive lower and exclusive upper bounds and lexical successors" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const lexed = try Lexer.lexAt(arena.allocator(), "match x | ## comment ## 1 => 2 | _ => 3", .{});
    const index = try TokenIndex.init(arena.allocator(), lexed);
    for (0..40) |after| {
        for (after..40) |before| {
            var expected: ?usize = null;
            for (lexed, 0..) |token, i| {
                if (token.type == .bar and token.start >= after and token.start < before) {
                    expected = i;
                    break;
                }
            }
            try std.testing.expectEqual(expected, index.armBar(after, before));
        }
    }
    const first = index.armBar(0, 40).?;
    try std.testing.expectEqual(Lexer.TokenType.comment, lexed[first + 1].type);
}

test "indexed block closers and grouped imports use raw lexer spans" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const alloc = arena.allocator();
    const lexed = try Lexer.lexAt(alloc, "do do end do end end do", .{});
    const index = try TokenIndex.init(alloc, lexed);
    var node: ast.Node = .{
        .span = .{ .start = 0, .end = 2, .line = 1, .column = 1 },
        .expr = .{ .block = &.{} },
    };
    for ([_]struct { opener: usize, closer: ?usize }{
        .{ .opener = 0, .closer = 20 },
        .{ .opener = 3, .closer = 9 },
        .{ .opener = 10, .closer = 16 },
        .{ .opener = 21, .closer = null },
    }) |pair| {
        node.span.start = pair.opener;
        node.span.end = pair.opener + 2;
        try std.testing.expectEqual(pair.closer, index.concreteBlockEnd(&node));
    }
    node.span.start = 0;
    node.span.end = 21;
    try std.testing.expectEqual(@as(?usize, null), index.concreteBlockEnd(&node));
    node.span.end = 2;
    node.synthetic_block = true;
    try std.testing.expectEqual(@as(?usize, null), index.concreteBlockEnd(&node));

    const imports = [_]Lexer.Token{
        .{ .type = .ident, .text = "", .line = 1, .column = 1, .start = 0, .end = 0 },
        .{ .type = .kw_import, .text = "import", .line = 1, .column = 1, .start = 0, .end = 6 },
        .{ .type = .eof, .text = "", .line = 1, .column = 7, .start = 6, .end = 6 },
    };
    const import_index = try TokenIndex.init(alloc, &imports);
    node.span.end = 6;
    try std.testing.expect(import_index.groupedImport(&node));
    node.span.end = 5;
    try std.testing.expect(!import_index.groupedImport(&node));

    const source = "#! módulo é !#\n#* 文書 *#\nlet x = 1";
    const docs = try Lexer.lexAt(alloc, source, .{});
    const doc_index = try TokenIndex.init(alloc, docs);
    const envelopes = try tokens(alloc, source, docs);
    for (docs[0..2], envelopes[0..2]) |raw, envelope| {
        try std.testing.expect(TokenIndex.exact(doc_index.starts, raw.start));
        try std.testing.expect(TokenIndex.exact(doc_index.ends, raw.end));
        try std.testing.expect(!TokenIndex.exact(doc_index.starts, envelope.start));
        try std.testing.expect(!TokenIndex.exact(doc_index.ends, envelope.end));
    }
}
