const std = @import("std");
const c = @cImport({ @cInclude("stdio.h"); });

pub fn main(init: std.process.Init) void {
    const allocator = std.heap.page_allocator;
    const args = init.minimal.args.toSlice(allocator) catch std.process.exit(2);
    defer allocator.free(args);
    if (args.len != 10) std.process.exit(2);
    const size = std.fmt.parseInt(i64, args[1], 10) catch std.process.exit(2);
    if (size < 0 or size > 5000000) std.process.exit(2);
    var checksum: i64 = 0;
    var i: i64 = 0;
    while (i < size) : (i += 1) {
        const token_index: usize = @intCast(@rem(i, 8) + 2);
        const value = std.fmt.parseInt(i64, args[token_index], 10) catch std.process.exit(2);
        checksum += value;
    }
    _ = c.printf("checksum=%lld processed=%lld\n", @as(c_longlong, checksum), @as(c_longlong, size));
}
