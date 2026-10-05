const std = @import("std");
const Lexer = @import("../vendor/revo/src/lang/Lexer.zig");
// Resource policy for the pinned grammar, independent of parse validity. All
// counters are shared across decoded fragments. The lexer and the worklist are
// iterative; no parser, AST walk, document construction or recursive cleanup runs
// until this check succeeds. The caller owns the arena containing lexer results.
const Fragment = struct { source: []const u8, depth: usize };
const Metrics = struct { tokens: usize = 0, risk: usize = 0, edges: usize = 0, binary: usize = 0, traversal: usize = 0, nesting: usize = 0, work: usize = 0 };
pub fn check(alloc: std.mem.Allocator, source: []const u8) !void {
    var result: Metrics = .{ .work = source.len };
    var pending: std.ArrayList(Fragment) = .empty;
    try pending.append(alloc, .{ .source = source, .depth = 0 });
    while (pending.pop()) |fragment| {
        if (result.work > 1024 * 1024) return error.EmbeddedWorkLimit;
        if (fragment.depth > 32) return error.NestingLimit;
        const lexed = try Lexer.lexReportAt(alloc, fragment.source, .{});
        // The parser uses the same lexer before recursing into this fragment.
        // A lexical failure cannot reach its parser; keep the normal diagnostic.
        if (lexed == .err) continue;
        var nesting = fragment.depth;
        for (lexed.ok, 0..) |token, index| {
            if (token.type == .eof) continue;
            result.tokens += 1;
            if (result.tokens > 4096) return error.TokenLimit;
            switch (token.type) {
                .lparen, .lbracket, .lsquiggly => {
                    nesting += 1;
                },
                .rparen, .rbracket, .rsquiggly => {
                    if (nesting > fragment.depth) nesting -= 1;
                },
                else => {},
            }
            result.nesting = @max(result.nesting, nesting);
            if (result.nesting > 32) return error.NestingLimit;
            switch (token.type) {
                .kw_const, .kw_global, .kw_let, .kw_test, .kw_suite, .kw_declare, .kw_proc, .kw_fn, .minus, .kw_not, .kw_if, .kw_unless, .kw_match, .kw_do, .kw_loop, .kw_for, .kw_while, .kw_break, .kw_return, .kw_comp, .kw_import, .kw_spawn, .kw_pub, .kw_type, .attribute, .doc_comment, .caret, .concat, .assign, .plus_assign, .minus_assign, .star_assign, .slash_assign, .percent_assign, .caret_assign, .concat_assign, .lt, .bang, .huh => {
                    result.risk += 2;
                },
                else => {},
            }
            if (token.type == .kw_match) result.binary += 4;
            // Count every pinned Parser infix/logical operator, including forms
            // the layout builder does not reflow: their ASTs still recurse.
            switch (token.type) {
                .plus, .minus, .star, .slash, .floor_div, .percent, .eq, .neq, .lt, .gt, .lte, .gte, .concat, .caret, .kw_and, .kw_or, .kw_orelse, .kw_band, .kw_bor, .kw_bxor, .kw_shl, .kw_shr => {
                    result.binary += 1;
                },
                else => {},
            }
            result.edges += switch (token.type) {
                .number, .ident, .atom, .string, .multiline_string, .comment, .doc_comment, .module_doc, .eof, .rparen, .rbracket, .rsquiggly, .kw_end, .comma, .semicolon => 0,
                .kw_let, .kw_const, .kw_global, .kw_type, .kw_declare, .kw_test, .kw_suite => 2,
                .kw_fn, .kw_proc => 3,
                .pipe_forward => 8,
                .dotdot => 4,
                .lparen, .lbracket, .attribute, .backtick_string => 2,
                else => 1,
            };
            if (result.risk + 4 * result.nesting > 672) return error.ParserLimit;
            if (result.binary + 4 * result.nesting > 800) return error.LayoutLimit;
            if (result.edges > 1536) return error.TreeLimit;
            // Call/method receiver paths deepen AST traversals even when their
            // parentheses are flat. Count possible hugging postfixes lexically;
            // this is admission policy, not a second syntax parser.
            result.traversal += switch (token.type) {
                .dot, .lbracket => 1,
                .string, .multiline_string => 1,
                .pipe_forward => 8,
                .dotdot => 4,
                .kw_fn, .kw_proc, .kw_if, .kw_unless, .kw_match, .kw_loop, .kw_for, .kw_while => 1,
                .lparen => if (potentialCall(lexed.ok, index)) 1 else 0,
                .atom => if (index + 1 < lexed.ok.len and lexed.ok[index + 1].type == .lparen and token.end == lexed.ok[index + 1].start) 1 else 0,
                else => 0,
            };
            if (result.traversal + result.risk / 2 + result.binary + 4 * result.nesting > 900) return error.TraversalLimit;
            if (token.interp_opens.len > 0) {
                result.edges += 3 + token.interp_opens.len;
                if (result.edges > 1536) return error.TreeLimit;
                result.traversal += 3 + token.interp_opens.len;
                if (result.traversal + result.risk / 2 + result.binary + 4 * result.nesting > 900) return error.TraversalLimit;
            }
            if (token.type == .backtick_string) {
                // Match Parser.parseQuasiquote's splice substitution, including
                // decoded nested strings. Source metadata keeps these opaque.
                var rewritten: std.ArrayList(u8) = .empty;
                var i: usize = 0;
                var counter: usize = 0;
                while (i < token.text.len) {
                    if (rewritten.items.len + 32 > 1024 * 1024 - result.work) return error.EmbeddedWorkLimit;
                    if (token.text[i] == '%' and i + 1 < token.text.len and Lexer.isIdentStart(token.text[i + 1])) {
                        i += 2;
                        while (i < token.text.len and Lexer.isIdentContinue(token.text[i])) : (i += 1) {}
                        var buf: [32]u8 = undefined;
                        try rewritten.appendSlice(alloc, try std.fmt.bufPrint(&buf, "__qq_{d}", .{counter}));
                        counter += 1;
                    } else {
                        try rewritten.append(alloc, token.text[i]);
                        i += 1;
                    }
                }
                try enqueue(alloc, &pending, &result, rewritten.items, nesting + 1);
            }
            var literal_start: usize = 0;
            // Use decoded indices recorded by the upstream lexer, and the same
            // quote/escape boundaries and format suffixes as its parser.
            for (token.interp_opens) |open| {
                if (open.idx < literal_start) continue;
                const end = interpolationEnd(token.text, open.idx) orelse break;
                var body = token.text[open.idx + 1 .. end];
                const trailing = std.mem.trimEnd(u8, body, " \t\r\n");
                if (trailing.len >= 2 and trailing[trailing.len - 2] == ':' and std.mem.indexOfScalar(u8, "v?p", trailing[trailing.len - 1]) != null) body = body[0 .. trailing.len - 2];
                try enqueue(alloc, &pending, &result, body, nesting + 1);
                literal_start = end + 1;
            }
        }
    }
}
fn potentialCall(tokens: []const Lexer.Token, index: usize) bool {
    if (index == 0 or tokens[index - 1].end != tokens[index].start) return false;
    const previous = tokens[index - 1].type;
    // Function signatures consume their parentheses before the body; they do
    // not extend a receiver path. A contextual field named fn/proc still does.
    if ((previous == .kw_fn or previous == .kw_proc) and (index < 2 or tokens[index - 2].type != .dot)) return false;
    if (previous == .ident and index >= 2 and tokens[index - 2].type == .kw_fn and (index < 3 or tokens[index - 3].type != .dot)) return false;
    return true;
}
fn enqueue(alloc: std.mem.Allocator, pending: *std.ArrayList(Fragment), result: *Metrics, source: []const u8, depth: usize) !void {
    if (depth > 32) return error.NestingLimit;
    if (result.work + source.len > 1024 * 1024) return error.EmbeddedWorkLimit;
    result.work += source.len;
    try pending.append(alloc, .{ .source = source, .depth = depth });
}
fn interpolationEnd(raw: []const u8, start: usize) ?usize {
    var depth: usize = 1;
    var quote: u8 = 0;
    var escaped = false;
    var i = start + 1;
    while (i < raw.len) : (i += 1) {
        const c = raw[i];
        if (quote != 0) {
            if (escaped) {
                escaped = false;
            } else if (c == '\\') {
                escaped = true;
            } else if (c == quote) {
                quote = 0;
            }
            continue;
        }
        if (c == '"' or c == '\'' or c == '`') {
            quote = c;
        } else if (c == '{') {
            depth += 1;
        } else if (c == '}') {
            depth -= 1;
            if (depth == 0) return i;
        }
    }
    return null;
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

test "input limits: decoded bodies share the admission budget" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const depth = try arena.allocator().alloc(u8, 3000);
    @memset(depth, '(');
    const interpolation = try std.mem.concat(arena.allocator(), u8, &.{ "\"#{", depth, "1}\"" });
    const quasiquote = try std.mem.concat(arena.allocator(), u8, &.{ "`", depth, "1`" });
    try std.testing.expectError(error.NestingLimit, check(arena.allocator(), interpolation));
    try std.testing.expectError(error.NestingLimit, check(arena.allocator(), quasiquote));
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
