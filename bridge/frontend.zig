const std = @import("std");
const Parser = @import("../vendor/revo/src/lang/Parser.zig");
const Lexer = @import("../vendor/revo/src/lang/Lexer.zig");
const source_metadata = @import("source.zig");
const compare = @import("compare.zig");
const input_limits = @import("input_limits.zig");
const allocator = std.heap.c_allocator;
pub const Buffer = extern struct { ptr: ?[*]u8 = null, len: usize = 0 };
const Failure = struct { message: []const u8, offset: usize };
const Response = struct { syntax: ?Failure = null, validation: ?[]const u8 = null, equivalent: ?bool = null, tokens: ?[]const Token = null, regions: ?[]const Token = null };
const Token = source_metadata.Range;
fn response(value: Response) Buffer {
    const bytes = std.json.Stringify.valueAlloc(allocator, value, .{}) catch return .{};
    return .{ .ptr = bytes.ptr, .len = bytes.len };
}
export fn revo_free(buffer: Buffer) void {
    if (buffer.ptr) |ptr| allocator.free(ptr[0..buffer.len]);
}
// Lexer-only admission check. It must run before any parser or AST traversal.
export fn revo_preflight(ptr: [*]const u8, len: usize) Buffer {
    var arena = std.heap.ArenaAllocator.init(allocator);
    defer arena.deinit();
    input_limits.check(arena.allocator(), ptr[0..len]) catch |err| return response(.{ .validation = switch (err) {
        error.EmbeddedWorkLimit => "input complexity limit exceeded: decoded lexer work (1048576 bytes)",
        error.NestingLimit => "input complexity limit exceeded: combined nesting (32)",
        error.TokenLimit => "input complexity limit exceeded: expanded tokens (4096)",
        error.ParserLimit => "input complexity limit exceeded: parser score (672)",
        error.LayoutLimit => "input complexity limit exceeded: layout score (800)",
        error.TreeLimit => "input complexity limit exceeded: syntax tree score (1536)",
        error.TraversalLimit => "input complexity limit exceeded: AST traversal score (900)",
        else => @errorName(err),
    } });
    return response(.{});
}
fn failure(parsed: Parser.ParseResult) ?Failure {
    return switch (parsed) {
        .ok => null,
        .err => |err| blk: {
            var offset: usize = 0;
            for (err.report.parts) |part| {
                if (part == .span) {
                    offset = part.span.span.start;
                    break;
                }
            }
            break :blk .{ .message = err.report.message, .offset = offset };
        },
    };
}
// A few upstream syntax failures escape the report API as errors (for example
// anonymous proc declarations). Keep those in the syntax category; allocation
// failure remains a bridge/validation failure.
fn parse(alloc: std.mem.Allocator, source: []const u8) !Parser.ParseResult {
    return Parser.parseSourceReport(alloc, source, .{ .repl_mode = false }) catch |err| switch (err) {
        error.OutOfMemory => return err,
        else => .{ .err = .{ .kind = .UnexpectedToken, .report = .{ .message = @errorName(err) } } },
    };
}
export fn revo_analyze(ptr: [*]const u8, len: usize) Buffer {
    return analyze(ptr[0..len]) catch |err| response(.{ .validation = @errorName(err) });
}
fn analyze(source: []const u8) !Buffer {
    var arena = std.heap.ArenaAllocator.init(allocator);
    defer arena.deinit();
    const alloc = arena.allocator();
    const parsed = try parse(alloc, source);
    if (failure(parsed)) |err| return response(.{ .syntax = err });
    const lexed = try Lexer.lexAt(alloc, source, .{});
    return response(.{ .tokens = try source_metadata.tokens(alloc, source, lexed), .regions = try source_metadata.regions(alloc, source, lexed, parsed.ok) });
}
export fn revo_equivalent(a: [*]const u8, a_len: usize, b: [*]const u8, b_len: usize) Buffer {
    return equivalent(a[0..a_len], b[0..b_len]) catch |err| response(.{ .validation = @errorName(err) });
}
fn equivalent(a: []const u8, b: []const u8) !Buffer {
    var arena = std.heap.ArenaAllocator.init(allocator);
    defer arena.deinit();
    const original = try parse(arena.allocator(), a);
    if (failure(original)) |err| return response(.{ .syntax = err });
    const candidate = try parse(arena.allocator(), b);
    if (failure(candidate)) |err| return response(.{ .syntax = err });
    return response(.{ .equivalent = compare.equal(@TypeOf(original.ok), original.ok, candidate.ok) });
}
