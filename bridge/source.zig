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
    const block_ends = try alloc.alloc(?usize, lexed.len);
    @memset(block_ends, null);
    var collector: Collector = .{ .alloc = alloc, .source = source, .index = try TokenIndex.init(alloc, lexed), .block_ends = block_ends };
    // A synthetic module list is a real list of source statements; a grouped
    // import is also synthetic, but its span is only the `import` keyword.
    if (root.expr == .block and root.synthetic_block and !collector.groupedImport(root)) {
        for (root.expr.block) |child| _ = try collector.statement(child);
    } else if (root.expr != .block or !root.synthetic_block) {
        _ = try collector.statement(root);
    }
    // Keep the existing hint order. All endpoints are finalized from the same
    // child-first block table, after the single AST traversal has completed.
    for (collector.result.items) |*region| {
        if (std.mem.eql(u8, region.kind, "block") or std.mem.eql(u8, region.kind, "statement") or std.mem.eql(u8, region.kind, "body")) {
            if (collector.blockEnd(region.start)) |end| {
                if (region.end <= end) region.end = end;
            }
        }
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
    arrows: []const Entry,
    opaque_starts: []const Entry,
    opaque_max_ends: []const usize,
    block_closers: []const Entry,

    fn lessThan(_: void, a: Entry, b: Entry) bool {
        return a.offset < b.offset or (a.offset == b.offset and a.token_index < b.token_index);
    }

    fn init(alloc: std.mem.Allocator, lexed: []const Lexer.Token) !TokenIndex {
        const starts = try alloc.alloc(Entry, lexed.len);
        const ends = try alloc.alloc(Entry, lexed.len);
        var block_closers: std.ArrayList(Entry) = .empty;
        for (lexed, 0..) |token, i| {
            starts[i] = .{ .offset = token.start, .token_index = i };
            ends[i] = .{ .offset = token.end, .token_index = i };
        }
        std.mem.sort(Entry, starts, {}, lessThan);
        std.mem.sort(Entry, ends, {}, lessThan);
        var bars: std.ArrayList(Entry) = .empty;
        var arrows: std.ArrayList(Entry) = .empty;
        var opaque_starts: std.ArrayList(Entry) = .empty;
        var opaque_max_ends: std.ArrayList(usize) = .empty;
        var max_end: usize = 0;
        for (starts) |entry| {
            const token = lexed[entry.token_index];
            if (token.type == .kw_end) try block_closers.append(alloc, entry);
            if (token.type == .bar) try bars.append(alloc, entry);
            if (token.type == .fat_arrow) try arrows.append(alloc, entry);
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
            .arrows = arrows.items,
            .opaque_starts = opaque_starts.items,
            .opaque_max_ends = opaque_max_ends.items,
            .block_closers = block_closers.items,
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

    // The body span can omit grouping parentheses. The last arrow before its
    // first lexical node is still the arm's arrow, including nested guards.
    fn armArrow(self: *const TokenIndex, after: usize, body: usize) ?usize {
        const i = lowerBound(self.arrows, body);
        if (i == 0 or self.arrows[i - 1].offset < after) return null;
        return self.arrows[i - 1].token_index;
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
    block_ends: []?usize = &.{},

    fn blockEnd(self: *const Collector, start: usize) ?usize {
        const i = TokenIndex.lowerBound(self.index.starts, start);
        if (i == self.index.starts.len or self.index.starts[i].offset != start) return null;
        return self.block_ends[self.index.starts[i].token_index];
    }

    fn blockOpener(self: *const Collector, node: *ast.Node) ?usize {
        if (node.expr != .block or node.synthetic_block) return null;
        const i = TokenIndex.lowerBound(self.index.starts, node.span.start);
        if (i == self.index.starts.len or self.index.starts[i].offset != node.span.start) return null;
        const opener = self.index.starts[i].token_index;
        if (self.index.lexed[opener].type != .kw_do) return null;
        // Pipe lowering can wrap a source block in a generated block with the
        // same start. Its first binding overlaps the opener; real statements
        // begin strictly inside the source block.
        if (node.expr.block.len > 0 and node.expr.block[0].span.start <= node.span.start) return null;
        return opener;
    }

    fn resolveBlock(self: *Collector, node: *ast.Node, opener: usize, descendants_end: usize) !usize {
        // Empty spans already include `end`. Nonempty spans omit it, possibly
        // through wrappers; child completion carries every descendant closer.
        if (node.expr.block.len == 0) {
            const i = TokenIndex.lowerBound(self.index.ends, node.span.end);
            if (i < self.index.ends.len and self.index.ends[i].offset == node.span.end) {
                const closer = self.index.lexed[self.index.ends[i].token_index];
                if (closer.type == .kw_end) {
                    self.block_ends[opener] = closer.end;
                    return closer.end;
                }
            }
            return error.InvalidBlockEnvelope;
        }
        const i = TokenIndex.lowerBound(self.index.block_closers, descendants_end);
        if (i == self.index.block_closers.len) return error.InvalidBlockEnvelope;
        const end = self.index.lexed[self.index.block_closers[i].token_index].end;
        self.block_ends[opener] = end;
        return end;
    }

    fn add(self: *Collector, kind: []const u8, start: usize, end: usize) !void {
        if (start >= end or end > self.source.len) return;
        // Require lexical endpoints, excluding spans inside decoded strings or
        // rewritten quasiquotes. Synthetic nodes never emit layout hints.
        if (TokenIndex.exact(self.index.starts, start) and TokenIndex.exact(self.index.ends, end))
            try self.result.append(self.alloc, .{ .kind = kind, .start = start, .end = end });
    }
    fn groupedImport(self: *Collector, node: *ast.Node) bool {
        return self.index.groupedImport(node);
    }
    fn statement(self: *Collector, node: *ast.Node) anyerror!usize {
        const emit = node.expr != .block or self.blockOpener(node) != null;
        if (emit) try self.add("statement", node.span.start, node.span.end);
        return self.visitNode(node, emit);
    }
    fn visitNode(self: *Collector, node_: *ast.Node, emit: bool) anyerror!usize {
        if (node_.expr == .quasiquote or self.index.insideOpaque(node_.span.start)) return node_.span.end;
        if (node_.expr == .block) {
            const opener = self.blockOpener(node_);
            // Concrete block facts remain valid beneath generated wrappers and
            // supply Rust's only block-pairing authority. The emit flag still
            // suppresses synthetic statement/header hints in those subtrees.
            if (opener != null) try self.add("block", node_.span.start, node_.span.end);
            var end = node_.span.end;
            for (node_.expr.block) |child| {
                // Keep generated statement/header hints suppressed while walking
                // through to any independently proven source blocks.
                const child_end = if (emit and opener != null) try self.statement(child) else try self.visitNode(child, false);
                end = @max(end, child_end);
            }
            return if (opener) |open| self.resolveBlock(node_, open, end) else end;
        }
        if (!emit) return @max(node_.span.end, try self.walk(ast.Expr, node_.expr, false));
        switch (node_.expr) {
            .fn_expr => |value| try self.headerBody(node_, value.body),
            .if_expr => |value| try self.headerBody(node_, value.then_expr),
            .unless_expr => |value| try self.headerBody(node_, value.then_expr),
            .while_loop => |value| try self.headerBody(node_, value.body),
            .for_loop => |value| try self.headerBody(node_, value.body),
            .unary => |value| if (value.op == .negate) {
                try self.add("unary_sign", node_.span.start, node_.span.end);
            },
            else => {},
        }
        if (node_.expr == .match_expr) {
            try self.add("match_expression", node_.span.start, node_.span.end);
            const match = node_.expr.match_expr;
            var after = match.subject.span.end;
            // Subjectless matches have a synthetic subject with the match token's span.
            for (match.arms, 0..) |arm, arm_index| {
                if (self.index.armBar(after, arm.then.span.start)) |index| {
                    if (arm_index == 0 and TokenIndex.exact(self.index.starts, node_.span.start))
                        try self.result.append(self.alloc, .{ .kind = "match_head", .start = node_.span.start, .end = self.index.lexed[index].start });
                    if (index + 1 < self.index.lexed.len) {
                        const start = self.index.lexed[index + 1].start;
                        try self.add("match_arm", start, arm.then.span.end);
                        if (self.index.armArrow(start, arm.then.span.start)) |arrow|
                            try self.add("match_arm_head", start, self.index.lexed[arrow].end);
                    }
                }
                after = arm.then.span.end;
            }
        }
        return @max(node_.span.end, try self.walk(ast.Expr, node_.expr, true));
    }
    fn headerBody(self: *Collector, node: *ast.Node, body: *ast.Node) !void {
        // Header ends are body starts, rather than AST endpoints. Only real
        // lexical starts can guide layout; generated/opaque nodes cannot.
        if (node.span.start >= body.span.start or self.index.insideOpaque(body.span.start)) return;
        if (!TokenIndex.exact(self.index.starts, node.span.start) or !TokenIndex.exact(self.index.starts, body.span.start)) return;
        try self.result.append(self.alloc, .{ .kind = "header", .start = node.span.start, .end = body.span.start });
        try self.add("body", body.span.start, body.span.end);
    }
    fn walk(self: *Collector, comptime T: type, value: T, emit: bool) anyerror!usize {
        if (T == ast.Span) return value.end;
        if (T == *ast.Node) return self.visitNode(value, emit);
        var end: usize = 0;
        switch (@typeInfo(T)) {
            .pointer => |info| switch (info.size) {
                .one => end = try self.walk(info.child, value.*, emit),
                .slice => if (info.child != u8) {
                    for (value) |item| end = @max(end, try self.walk(info.child, item, emit));
                },
                else => {},
            },
            .optional => |info| if (value) |inner| {
                end = try self.walk(info.child, inner, emit);
            },
            .@"struct" => |info| inline for (info.field_names, info.field_types) |name, Field| {
                end = @max(end, try self.walk(Field, @field(value, name), emit));
            },
            .@"union" => |info| inline for (info.field_names, info.field_types) |name, Field| {
                if (@as(info.tag_type.?, value) == @field(info.tag_type.?, name)) {
                    return self.walk(Field, @field(value, name), emit);
                }
            },
            else => {},
        }
        return end;
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

test "indexed grouped imports and docs use raw lexer spans" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const alloc = arena.allocator();
    var node: ast.Node = .{
        .span = .{ .start = 0, .end = 6, .line = 1, .column = 1 },
        .expr = .{ .block = &.{} },
        .synthetic_block = true,
    };

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
