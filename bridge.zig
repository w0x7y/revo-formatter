// Root module keeps bridge and unchanged vendor files within one module boundary.
comptime {
    _ = @import("bridge/frontend.zig");
}
test {
    _ = @import("bridge/compare.zig");
}
