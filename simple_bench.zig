const std = @import("std");

pub fn main() !void {
    const test_count: usize = 10000;
    std.debug.print("Random weights: {} values\n", .{test_count});

    var weights: [test_count]f32 = undefined;
    const seed: u32 = 0xF17;
    var counter: u32 = seed;
    for (0..test_count) |i| {
        counter = ((counter *% 1103515245) + 1);
        const temp_u = counter & 0xFFFFFF;
        const x = @as(f32, @floatFromInt(temp_u)) / 16777216.0;
        weights[i] = (x - 0.5) * 0.2;
    }

    std.debug.print("Random weights: {} values\n", .{test_count});
    std.debug.print("Benchmark complete\n", .{});
}
