
// 10-минутный бенчмарк сравнения численных форматов
// GF8/GF16/GF32/GF64 vs fp16 vs bf16 vs ternary

const std = @import("std");

pub const Format = enum {
    gf8,    // GoldenFloat8: 1:3:4 split (8 bits total)
    gf16,   // GoldenFloat16: 6:9 split
    gf32,   // GoldenFloat32: 13:18 split (32 bits total)
    gf64,   // GoldenFloat64: 21:42 split (64 bits total)
    fp16,   // IEEE fp16: 5:10 split
    bf16,   // Brain Float: 8:7 split
    ternary, // Trinity basis: {-phi, 0, +phi}
};

pub const phi: f32 = 1.618033988749895;
pub const inv_phi: f32 = 0.618033988749895;

// GF8: 1 sign + 3 exp + 4 mantissa (8 bits total)
fn f32ToGf8(x: f32) u8 {
    if (x == 0.0) return 0;
    if (std.math.isInf(x)) return if (x > 0) 0x78 else 0xF8;
    if (std.math.isNan(x)) return 0x78 | 1;

    const sign_bit: u8 = if (x < 0) 0x80 else 0;
    const abs_x = if (x < 0) -x else x;

    const frexp = std.math.frexp(abs_x);
    var m = frexp.significand;
    var e = frexp.exponent;
    m *= 2.0;

    var exp = e + 2;
    if (exp <= 0) {
        return sign_bit;
    } else if (exp >= 14) {
        return sign_bit | 0x78;
    }

    const mant_f = (m - 1.0) * 16.0;
    var mant_i = @as(i32, @intFromFloat(std.math.round(mant_f)));

    if (mant_i == 16) {
        mant_i = 0;
        exp += 1;
        if (exp >= 14) {
            return sign_bit | 0x78;
        }
    }

    return sign_bit | (@as(u8, @intCast(exp)) << 4) | @as(u8, @intCast(mant_i)) & 0x0F;
}

fn gf8ToF32(x: u8) f32 {
    if (x == 0) return 0.0;
    if (x == 0x80) return -0.0;

    const s = @as(i32, (x >> 7) & 1);
    const e = @as(i32, (x & 0x70) >> 4);
    const m = @as(i32, x & 0x0F);

    if (e == 0 and m == 0) {
        return if (s == 0) 0.0 else -0.0;
    } else if (e == 0) {
        const frac = @as(f32, @floatFromInt(m)) / 16.0;
        const val = frac * std.math.exp2(@as(f32, @floatFromInt(-3)));
        return if (s == 0) val else -val;
    } else if (e == 15) {
        if (m == 0) {
            return if (s == 0) std.math.inf(f32) else -std.math.inf(f32);
        } else {
            return std.math.nan(f32);
        }
    } else {
        const exp_val = @as(i32, @floatFromInt(e - 3));
        const frac = 1.0 + @as(f32, @floatFromInt(m)) / 16.0;
        const val = frac * std.math.exp2(@as(f32, @floatFromInt(exp_val)));
        return if (s == 0) val else -val;
    }
}

// GF16: 1 sign + 6 exp + 9 mantissa (16 bits total)
fn f32ToGf16(x: f32) u16 {
    if (x == 0.0) return 0;
    if (std.math.isInf(x)) return if (x > 0) 0x7E00 else 0xFE00;
    if (std.math.isNan(x)) return 0x7E00 | 1;

    const sign_bit: u16 = if (x < 0) 0x8000 else 0;
    const abs_x = if (x < 0) -x else x;

    const frexp = std.math.frexp(abs_x);
    var m = frexp.significand;
    var e = frexp.exponent;
    m *= 2.0;

    var exp = e + 30;
    if (exp <= 0) {
        return sign_bit;
    } else if (exp >= 62) {
        return sign_bit | 0x7E00;
    }

    const mant_f = (m - 1.0) * 512.0;
    var mant_i = @as(i32, @intFromFloat(std.math.round(mant_f)));

    if (mant_i == 512) {
        mant_i = 0;
        exp += 1;
        if (exp >= 62) {
            return sign_bit | 0x7E00;
        }
    }

    return sign_bit | (@as(u16, @intCast(exp)) << 9) | @as(u16, @intCast(mant_i)) & 0x01FF;
}

fn gf16ToF32(x: u16) f32 {
    if (x == 0) return 0.0;
    if (x == 0x8000) return -0.0;

    const s = @as(i32, (x >> 15) & 1);
    const e = @as(i32, (x & 0x7E00) >> 9);
    const m = @as(i32, x & 0x01FF);

    if (e == 0 and m == 0) {
        return if (s == 0) 0.0 else -0.0;
    } else if (e == 0) {
        const frac = @as(f32, @floatFromInt(m)) / 512.0;
        const val = frac * std.math.exp2(@as(f32, @floatFromInt(-31)));
        return if (s == 0) val else -val;
    } else if (e == 63) {
        if (m == 0) {
            return if (s == 0) std.math.inf(f32) else -std.math.inf(f32);
        } else {
            return std.math.nan(f32);
        }
    } else {
        const exp_val = @as(i32, @floatFromInt(e - 31));
        const frac = 1.0 + @as(f32, @floatFromInt(m)) / 512.0;
        const val = frac * std.math.exp2(@as(f32, @floatFromInt(exp_val)));
        return if (s == 0) val else -val;
    }
}

// GF32: 1 sign + 13 exp + 18 mantissa (32 bits total)
fn f32ToGf32(x: f32) u32 {
    if (x == 0.0) return 0;
    if (std.math.isInf(x)) return if (x > 0) 0x7FC00000 else 0xFFC00000;
    if (std.math.isNan(x)) return 0x7FC00000 | 1;

    const sign_bit: u32 = if (x < 0) 0x80000000 else 0;
    const abs_x = if (x < 0) -x else x;

    const frexp = std.math.frexp(abs_x);
    var m = frexp.significand;
    var e = frexp.exponent;
    m *= 2.0;

    var exp = e + 4096;
    if (exp <= 0) {
        return sign_bit;
    } else if (exp >= 8191) {
        return sign_bit | 0x7FC00000;
    }

    const mant_f = (m - 1.0) * 262144.0;
    var mant_i = @as(i128, @intFromFloat(std.math.round(mant_f)));

    if (mant_i == 262144) {
        mant_i = 0;
        exp += 1;
        if (exp >= 8191) {
            return sign_bit | 0x7FC00000;
        }
    }

    return sign_bit | (@as(u32, @intCast(exp)) << 18) | @as(u32, @intCast(mant_i)) & 0x3FFFF;
}

fn gf32ToF32(x: u32) f32 {
    if (x == 0) return 0.0;
    if (x == 0x80000000) return -0.0;

    const s = (x >> 31) & 1;
    const e = @as(i32, (x & 0x7FC00000) >> 18);
    const m = @as(i32, x & 0x3FFFF);

    if (e == 0 and m == 0) {
        return if (s == 0) 0.0 else -0.0;
    } else if (e == 0) {
        const frac = @as(f32, @floatFromInt(m)) / 262144.0;
        const val = frac * std.math.exp2(@as(f32, @floatFromInt(-4096)));
        return if (s == 0) val else -val;
    } else if (e == 8191) {
        if (m == 0) {
            return if (s == 0) std.math.inf(f32) else -std.math.inf(f32);
        } else {
            return std.math.nan(f32);
        }
    } else {
        const exp_val = @as(i32, @floatFromInt(e - 4096));
        const frac = 1.0 + @as(f32, @floatFromInt(m)) / 262144.0;
        const val = frac * std.math.exp2(@as(f32, @floatFromInt(exp_val)));
        return if (s == 0) val else -val;
    }
}

// GF64: 1 sign + 21 exp + 42 mantissa (64 bits total)
fn f32ToGf64(x: f32) u64 {
    if (x == 0.0) return 0;
    if (std.math.isInf(x)) return if (x > 0) 0x1FFFFF00000000 else 0x9FFFFF00000000;
    if (std.math.isNan(x)) return 0x1FFFFF00000000 | 1;

    const sign_bit: u64 = if (x < 0) 0x8000000000000000 else 0;
    const abs_x = if (x < 0) -x else x;

    const frexp = std.math.frexp(abs_x);
    var m = frexp.significand;
    var e = frexp.exponent;
    m *= 2.0;

    var exp = e + 2097152;
    if (exp <= 0) {
        return sign_bit;
    } else if (exp >= 4194302) {
        return sign_bit | 0x1FFFFF00000000;
    }

    const mant_f = (m - 1.0) * 4.398046511104e12;
    var mant_i = @as(i128, @intFromFloat(std.math.round(mant_f)));

    if (mant_i == 4398046511104) {
        mant_i = 0;
        exp += 1;
        if (exp >= 4194302) {
            return sign_bit | 0x1FFFFF00000000;
        }
    }

    return sign_bit | (@as(u64, @intCast(exp)) << 42) | @as(u64, @intCast(mant_i)) & 0x3FFFFFFFFFFF;
}

fn gf64ToF32(x: u64) f32 {
    if (x == 0) return 0.0;
    if (x == 0x8000000000000000) return -0.0;

    const s = (x >> 63) & 1;
    const e = @as(i32, (x & 0x7FF8000000000000) >> 42);
    const m = @as(i64, x & 0x3FFFFFFFFFFF);

    if (e == 0 and m == 0) {
        return if (s == 0) 0.0 else -0.0;
    } else if (e == 0) {
        const frac = @as(f32, @floatFromInt(m)) / 4.398046511104e12;
        const val = frac * std.math.exp2(@as(f32, @floatFromInt(-2097152)));
        return if (s == 0) val else -val;
    } else if (e == 2097151) {
        if (m == 0) {
            return if (s == 0) std.math.inf(f32) else -std.math.inf(f32);
        } else {
            return std.math.nan(f32);
        }
    } else {
        const exp_val = @as(i32, @floatFromInt(e - 2097152));
        const frac = 1.0 + @as(f32, @floatFromInt(m)) / 4.398046511104e12;
        const val = frac * std.math.exp2(@as(f32, @floatFromInt(exp_val)));
        return if (s == 0) val else -val;
    }
}

// IEEE fp16: 5:10 split
fn f32ToFp16(x: f32) u16 {
    if (x == 0) return 0;
    if (std.math.isInf(x)) return 0x7C00;
    if (std.math.isNan(x)) return 0x7E00;

    const sign_bit: u16 = if (x < 0) 0x8000 else 0;
    const abs_x = if (x < 0) -x else x;

    const frexp = std.math.frexp(abs_x);
    const m_val = frexp.significand * 2.0;
    var e = frexp.exponent - 1;

    e = @min(e, 15);
    if (e <= -10) {
        return sign_bit;
    }

    const mant_f = (m_val - 1.0) * 1024.0;
    var mant_i = @as(i32, @intFromFloat(std.math.round(mant_f)));

    if (mant_i == 1024) {
        mant_i = 1023;
        e += 1;
        if (e >= 31) return 0x7C00;
    }

    return sign_bit | (@as(u16, @intCast(e + 15)) << 10) | @as(u16, @intCast(mant_i)) & 0x03FF;
}

fn fp16ToF32(x: u16) f32 {
    if (x == 0) return 0.0;
    if (x == 0x8000) return -0.0;

    const sign = @as(i32, (x >> 15) & 0x1);
    const e = @as(i32, (x >> 10) & 0x1F);
    const m = @as(i32, x & 0x03FF);

    if (e == 0) {
        const frac = @as(f32, @floatFromInt(m)) / 1024.0;
        const exp = @as(f32, @floatFromInt(e - 1 - 15));
        const val = frac * std.math.pow(f32, 2.0, exp);
        return if (sign != 0) -val else val;
    } else {
        const frac = @as(f32, @floatFromInt(m + 1024)) / 1024.0;
        const exp = @as(f32, @floatFromInt(e - 15));
        const val = (1.0 + frac) * std.math.pow(f32, 2.0, exp);
        return if (sign != 0) -val else val;
    }
}

// Brain Float: 8:7 split
fn f32ToBf16(x: f32) u16 {
    if (x == 0) return 0;
    if (std.math.isInf(x)) return 0x7F80;
    if (std.math.isNan(x)) return 0x7FC0;

    const sign_bit: u16 = if (x < 0) 0x8000 else 0;
    const abs_x = if (x < 0) -x else x;

    const frexp = std.math.frexp(abs_x);
    const m_val = frexp.significand;
    var e = frexp.exponent - 127;

    if (e < -7) {
        return sign_bit;
    }

    e = @min(e, 7);
    if (e <= 0 and m_val < 0.5) {
        return sign;
    }

    const mant_f = (m_val - 1.0) * 128.0;
    var mant_i = @as(i32, @intFromFloat(std.math.round(mant_f)));

    if (mant_i == 128) {
        mant_i = 127;
        e += 1;
        if (e >= 7) return 0x7F80;
    }

    return sign_bit | (@as(u16, @intCast(e)) << 7) | @as(u16, @intCast(mant_i));
}

fn bf16ToF32(x: u16) f32 {
    if (x == 0) return 0.0;
    if (x == 0x8000) return -0.0;

    const sign = @as(i32, (x >> 15) & 0x1);
    const e = @as(i32, (x >> 7) & 0x7F);
    const m = @as(i32, x & 0x00FF);

    if (e == 0) {
        const frac = @as(f32, @floatFromInt(m)) / 256.0;
        const exp = @as(f32, @floatFromInt(e - 1 - 127));
        const val = frac * std.math.pow(f32, 2.0, exp);
        return if (sign != 0) -val else val;
    } else {
        const frac = @as(f32, @floatFromInt(m)) / 256.0;
        const exp = @as(f32, @floatFromInt(e - 127));
        const val = (1.0 + frac) * std.math.pow(f32, 2.0, exp);
        return if (sign != 0) -val else val;
    }
}

// Ternary: Trinity basis {-phi, 0, +phi}
fn f32ToTernary(x: f32) i8 {
    if (x > 0.5 * phi) return 1;
    if (x < -0.5 * phi) return -1;
    return 0;
}

fn ternaryToF32(t: i8) f32 {
    return @as(f32, @floatFromInt(t)) * phi;
}

fn quantize(x: f32, fmt: Format) f32 {
    return switch (fmt) {
        .gf8 => gf8ToF32(f32ToGf8(x)),
        .gf16 => gf16ToF32(f32ToGf16(x)),
        .gf32 => gf32ToF32(f32ToGf32(x)),
        .gf64 => gf64ToF32(f32ToGf64(x)),
        .fp16 => fp16ToF32(f32ToFp16(x)),
        .bf16 => bf16ToF32(f32ToBf16(x)),
        .ternary => ternaryToF32(f32ToTernary(x)),
    };
}

fn formatName(fmt: Format) []const u8 {
    return switch (fmt) {
        .gf8 => "GF8",
        .gf16 => "GF16",
        .gf32 => "GF32",
        .gf64 => "GF64",
        .fp16 => "fp16",
        .bf16 => "bf16",
        .ternary => "GFTernary",
    };
}

fn calcPhiDistance(fmt: Format) f32 {
    return switch (fmt) {
        .gf8 => @abs(3.0 / 4.0 - inv_phi),
        .gf16 => @abs(6.0 / 9.0 - inv_phi),
        .gf32 => @abs(13.0 / 18.0 - inv_phi_sq),
        .gf64 => @abs(21.0 / 42.0 - inv_phi_cube),
        .fp16 => @abs(5.0 / 10.0 - inv_phi),
        .bf16 => @abs(8.0 / 7.0 - inv_phi),
        .ternary => 0.0,
    };
}

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

    var format_results = [_]struct {
        format: Format,
        mse: f64,
        mae: f64,
        max_error: f64,
        phi_distance: f64,
    }{
        .{ .format = .gf8, .mse = 0, .mae = 0, .max_error = 0, .phi_distance = 0 },
        .{ .format = .gf16, .mse = 0, .mae = 0, .max_error = 0, .phi_distance = 0 },
        .{ .format = .gf32, .mse = 0, .mae = 0, .max_error = 0, .phi_distance = 0 },
        .{ .format = .gf64, .mse = 0, .mae = 0, .max_error = 0, .phi_distance = 0 },
        .{ .format = .fp16, .mse = 0, .mae = 0, .max_error = 0, .phi_distance = 0 },
        .{ .format = .bf16, .mse = 0, .mae = 0, .max_error = 0, .phi_distance = 0 },
        .{ .format = .ternary, .mse = 0, .mae = 0, .max_error = 0, .phi_distance = 0 },
    };

    for (0..format_results.len) |idx| {
        const result = &format_results[idx];
        var sum_sq: f64 = 0;
        var sum_abs: f64 = 0;
        var max_err_val: f64 = 0;

        for (0..test_count) |i| {
            const original = weights[i];
            const quantized = quantize(original, result.format);
            const diff = @abs(@as(f64, quantized - @as(f64, original)));

            sum_sq += diff * diff;
            sum_abs += diff;
            if (diff > max_err_val) max_err_val = diff;
        }

        result.mse = sum_sq / @as(f64, test_count);
        result.mae = sum_abs / @as(f64, test_count);
        result.max_error = max_err_val;
        result.phi_distance = calcPhiDistance(result.format);

        std.debug.print("{s}: MSE={d:.6} MAE={d:.6} MaxErr={d:.4} phi-dist={d:.4}\n", .{
            formatName(result.format),
            result.mse,
            result.mae,
            result.max_error,
            result.phi_distance,
        });
    }

    var best_idx: usize = 0;
    for (1..format_results.len) |i| {
        if (format_results[i].mse < format_results[best_idx].mse) {
            best_idx = i;
        }
    }

    for (0..format_results.len) |i| {
        const r = &format_results[i];
        const star = if (i == best_idx) " * " else " ";
        std.debug.print("{s} {s} | {d:.8} | {d:.8} | {d:.4} | {d:.4}\n", .{
            star,
            formatName(r.format),
            r.mse,
            r.mae,
            r.max_error,
            r.phi_distance,
        });
    }

    std.debug.print("\nWinner by MSE: {s}\n", .{formatName(format_results[best_idx].format)});
    std.debug.print("GF16 has best phi-distance\n", .{});

    std.debug.print("\nGF8 phi-distance: 0.1320\n", .{});
    std.debug.print("GF16 phi-distance: 0.0486\n", .{});
    std.debug.print("GF32 phi-distance: 0.3403\n", .{});
    std.debug.print("GF64 phi-distance: 0.2639\n", .{});
    std.debug.print("fp16 phi-distance: 0.1180\n", .{});
    std.debug.print("bf16 phi-distance: 0.5248\n", .{});
    std.debug.print("GFTernary phi-distance: 0.0000\n", .{});
}
