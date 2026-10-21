const std = @import("std");
const c = @cImport({ @cInclude("stdio.h"); });

pub fn main(init: std.process.Init) void {
    const allocator = std.heap.page_allocator;
    const args = init.minimal.args.toSlice(allocator) catch std.process.exit(2);
    defer allocator.free(args);
    if (args.len != 2) std.process.exit(2);
    const rounds = std.fmt.parseInt(i64, args[1], 10) catch std.process.exit(2);
    if (rounds < 0 or rounds > 4096) std.process.exit(2);
    var checksum: i64 = 0;
    var round: i64 = 0;
    while (round < rounds) : (round += 1) {
        var values: std.ArrayList(i64) = .empty;
        defer values.deinit(allocator);
        var index: i64 = 0;
        while (index < 32) : (index += 1) {
            values.append(allocator, @rem(round * 17 + index * 13, 1009)) catch std.process.exit(3);
        }
        for (values.items) |value| checksum += value;
    }
    _ = c.printf("checksum=%lld processed=%lld\n", @as(c_longlong, checksum), @as(c_longlong, rounds * 32));
}
