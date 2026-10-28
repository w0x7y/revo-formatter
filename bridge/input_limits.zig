const std = @import("std");
const Lexer = @import("../vendor/revo/src/lang/Lexer.zig");
// Resource policy for the pinned grammar, independent of parse validity. The
// lexer and this scan are iterative; no parser, CST walk, document construction
// or recursive cleanup runs until this check succeeds. Interpolations lex inline
// as string frames, so one global pass covers them. The parser still recovers
// after lexical errors, so their tokens count too. The caller owns the arena
// containing lexer results.
const Metrics = struct { tokens: usize = 0, risk: usize = 0, edges: usize = 0, binary: usize = 0, blocks: usize = 0, layout_prefixes: usize = 0, traversal: usize = 0, nesting: usize = 0 };
pub fn check(alloc: std.mem.Allocator, source: []const u8) !void {
    if (source.len > 1024 * 1024) return error.EmbeddedWorkLimit;
    var result: Metrics = .{};
    const lexed = try Lexer.lexAll(alloc, source);
    // Comments are trivia but remain tape tokens for layout.
    result.tokens = lexed.trivia.len;
    if (result.tokens > 4096) return error.TokenLimit;
    // Interpolation opens per enclosing string, innermost last.
    var strings: std.ArrayList(usize) = .empty;
    var nesting: usize = 0;
    for (lexed.tokens, 0..) |token, index| {
        switch (token.type) {
            .eof => continue,
            .string_start, .ml_string_start => try strings.append(alloc, 0),
            .string_end, .ml_string_end => _ = strings.pop(),
            else => {},
        }
        // A complete string is one tape token; its delimiters and parts are not.
        switch (token.type) {
            .string_part, .ml_string_part, .string_end, .ml_string_end, .interp_open, .interp_close => {},
            else => {
                result.tokens += 1;
                if (result.tokens > 4096) return error.TokenLimit;
            },
        }
        switch (token.type) {
            .lparen, .lbracket, .lsquiggly, .interp_open => {
                nesting += 1;
            },
            .rparen, .rbracket, .rsquiggly, .interp_close => {
                if (nesting > 0) nesting -= 1;
            },
            else => {},
        }
        result.nesting = @max(result.nesting, nesting);
        if (result.nesting > 32) return error.NestingLimit;
        switch (token.type) {
            .kw_const, .kw_global, .kw_let, .kw_test, .kw_suite, .kw_declare, .kw_fn, .minus, .kw_not, .kw_if, .kw_unless, .kw_match, .kw_do, .kw_loop, .kw_for, .kw_while, .kw_break, .kw_return, .kw_comp, .kw_import, .kw_spawn, .kw_pub, .kw_type, .attribute, .doc_comment, .caret, .concat, .assign, .plus_assign, .minus_assign, .star_assign, .slash_assign, .percent_assign, .caret_assign, .concat_assign, .lt, .bang, .huh => {
                result.risk += 2;
            },
            else => {},
        }
        // Possible source blocks deepen document construction without any
        // punctuation nesting. Charge every lexical introducer globally,
        // including contextual names and interpolated values.
        if (token.type == .kw_do) result.blocks += 4;
        result.layout_prefixes += switch (token.type) {
            .kw_fn, .kw_if, .kw_unless, .kw_loop, .kw_for, .kw_while, .kw_not => 2,
            else => 0,
        };
        if (token.type == .kw_match) result.binary += 4;
        // Count every pinned Parser infix/logical operator, including forms
        // the layout builder does not reflow: their CSTs still recurse.
        switch (token.type) {
            .plus, .minus, .star, .slash, .floor_div, .percent, .eq, .neq, .lt, .gt, .lte, .gte, .concat, .caret, .kw_and, .kw_or, .kw_orelse, .kw_band, .kw_bor, .kw_bxor, .kw_shl, .kw_shr => {
                result.binary += 1;
            },
            else => {},
        }
        result.edges += switch (token.type) {
            .number, .ident, .atom, .string, .string_start, .string_part, .string_end, .ml_string_start, .ml_string_part, .ml_string_end, .interp_open, .interp_close, .doc_comment, .eof, .rparen, .rbracket, .rsquiggly, .kw_end, .comma, .semicolon => 0,
            .kw_let, .kw_const, .kw_global, .kw_type, .kw_declare, .kw_test, .kw_suite => 2,
            .kw_fn => 3,
            .pipe_forward => 8,
            .dotdot => 4,
            .lparen, .lbracket, .attribute => 2,
            else => 1,
        };
        if (result.risk + 4 * result.nesting > 672) return error.ParserLimit;
        // Block frames compound with recursive prefixes in either token
        // order. A later block also charges every earlier prefix.
        const block_prefixes = if (result.blocks > 0) result.layout_prefixes else 0;
        if (result.binary + result.blocks + block_prefixes + 4 * result.nesting > 800) return error.LayoutLimit;
        if (result.edges > 1536) return error.TreeLimit;
        // Call/method receiver paths deepen CST traversals even when their
        // parentheses are flat. Count possible hugging postfixes lexically;
        // this is admission policy, not a second syntax parser.
        result.traversal += switch (token.type) {
            .dot, .lbracket => 1,
            .string, .string_start, .ml_string_start => 1,
            .pipe_forward => 8,
            .dotdot => 4,
            .kw_fn, .kw_if, .kw_unless, .kw_match, .kw_loop, .kw_for, .kw_while => 1,
            .lparen => if (potentialCall(lexed.tokens, index)) 1 else 0,
            .atom => if (index + 1 < lexed.tokens.len and lexed.tokens[index + 1].type == .lparen and token.end == lexed.tokens[index + 1].start) 1 else 0,
            else => 0,
        };
        if (token.type == .interp_open and strings.items.len > 0) {
            // An interpolated string costs three plus one per open.
            const opens = &strings.items[strings.items.len - 1];
            const cost: usize = if (opens.* == 0) 4 else 1;
            opens.* += 1;
            result.edges += cost;
            if (result.edges > 1536) return error.TreeLimit;
            result.traversal += cost;
        }
        if (result.traversal + result.risk / 2 + result.binary + 4 * result.nesting > 900) return error.TraversalLimit;
    }
}
fn potentialCall(tokens: []const Lexer.Token, index: usize) bool {
    if (index == 0 or tokens[index - 1].end != tokens[index].start) return false;
    const previous = tokens[index - 1].type;
    // Function signatures consume their parentheses before the body; they do
    // not extend a receiver path. A contextual field named fn still does.
    if (previous == .kw_fn and (index < 2 or tokens[index - 2].type != .dot)) return false;
    if (previous == .ident and index >= 2 and tokens[index - 2].type == .kw_fn and (index < 3 or tokens[index - 3].type != .dot)) return false;
    return true;
}
test "input limits: opaque strings and comments do not create syntax depth" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const text = try arena.allocator().alloc(u8, 10000);
    @memset(text, '(');
    for ([_][]const u8{ "'", "##" }) |delimiter| {
        const source = try std.mem.concat(arena.allocator(), u8, &.{ delimiter, text, delimiter });
        try check(arena.allocator(), source);
    }
}

test "input limits: interpolated values share the admission budget" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const depth = try arena.allocator().alloc(u8, 3000);
    @memset(depth, '(');
    const interpolation = try std.mem.concat(arena.allocator(), u8, &.{ "\"#{", depth, "1}\"" });
    try std.testing.expectError(error.NestingLimit, check(arena.allocator(), interpolation));
    var repeated: std.ArrayList(u8) = .empty;
    for (0..200) |_| try repeated.appendSlice(arena.allocator(), "not ");
    const unary = try std.mem.concat(arena.allocator(), u8, &.{ "\"#{", repeated.items, "1} #{", repeated.items, "1}\"" });
    try std.testing.expectError(error.ParserLimit, check(arena.allocator(), unary));
}

test "input limits: lexer work is bounded before lexing" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const source = try arena.allocator().alloc(u8, 1024 * 1024 + 1);
    @memset(source, 'a');
    try std.testing.expectError(error.EmbeddedWorkLimit, check(arena.allocator(), source));
}

test "input limits: ordinary blocks are rejected before recursive layout" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    for ([_]usize{ 300, 336, 201 }) |count| {
        var source: std.ArrayList(u8) = .empty;
        for (0..count) |_| try source.appendSlice(arena.allocator(), "do\n");
        try source.appendSlice(arena.allocator(), "1");
        for (0..count) |_| try source.appendSlice(arena.allocator(), "\nend");
        try std.testing.expectError(error.LayoutLimit, check(arena.allocator(), source.items));
    }
}
