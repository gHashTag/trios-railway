
// Format comparison: GF16 vs fp16 vs bf16
const std = @import("std");

pub const phi: f32 = 1.618033988749895;
pub const inv_phi: f32 = 0.618033988749895;

// GF16: 6:9 split (16 bits total)
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
    e -= 1;

    var exp = e + 31;
    if (e <= 0) {
        return sign_bit;
    } else if (e >= 63) {
        return sign_bit | 0x7E00;
    }

    const mant_f = (m - 1.0) * 512.0;
    var mant_i = @as(i32, @intFromFloat(std.math.round(mant_f)));

    if (mant_i == 512) {
        mant_i = 0;
        exp += 1;
        if (exp >= 63) {
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

// IEEE fp16: 5:10 split
fn f32ToFp16(x: f32) u16 {
    if (x == 0) return 0;
    if (std.math.isInf(x)) return 0x7C00;

    const sign = @as(i32, (x >> 15) & 0x1);
    const abs_x = if (x < 0) -x else x;

    const frexp = std.math.frexp(abs_x);
    const m_val = frexp.significand * 2.0;
    var e = frexp.exponent - 1;

    e = @min(e, 15);
    if (e <= -10) {
        return sign;
    }

    const mant_f = (m_val - 1.0) * 1024.0;
    var mant_i = @as(i32, @intFromFloat(std.math.round(mant_f)));

    if (mant_i == 1024) {
        mant_i = 1023;
        e += 1;
        if (e >= 31) return 0x7C00;
    }

    return sign | (@as(u16, @intCast(e + 15)) << 10) | @as(u16, @intCast(mant_i)) & 0x03FF;
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

// Ternary: {-1, 0, +1}
fn f32ToTernary(x: f32) i8 {
    if (x > 0.5) return 1;
    if (x < -0.5) return -1;
    return 0;
}

fn ternaryToF32(t: i8) f32 {
    return @as(f32, @floatFromInt(t)) * phi;
}

fn quantize(x: f32, fmt: Format) f32 {
    return switch (fmt) {
        .gf16 => gf16ToF32(f32ToGf16(x)),
        .fp16 => fp16ToF32(f32ToFp16(x)),
        .bf16 => bf16ToF32(f32ToBf16(x)),
        .ternary => ternaryToF32(f32ToTernary(x)),
    };
}

fn formatName(fmt: Format) []const u8 {
    return switch (fmt) {
        .gf16 => "GF16",
        .fp16 => "fp16",
        .bf16 => "bf16",
        .ternary => "GFTernary",
    };
}

fn calcPhiDistance(fmt: Format) f32 {
    return switch (fmt) {
        .gf16 => @abs(6.0 / 9.0 - inv_phi),
        .fp16 => @abs(5.0 / 10.0 - inv_phi),
        .bf16 => @abs(8.0 / 7.0 - inv_phi),
        .ternary => 0.0,
    };
}

pub fn main() !void {
    const test_count: usize = 10000;
    std.debug.print("Random weights: {} values\n", .{test_count});

    // Generate random weights
    var weights: [test_count]f32 = undefined;
    const seed: u32 = 0xF17;
    var counter: u32 = seed;
    for (0..test_count) |i| {
        counter = ((counter *% 1103515245) + 1);
        const temp_u = counter & 0xFFFFFF;
        const x = @as(f32, @floatFromInt(temp_u)) / 16777216.0;
        weights[i] = (x - 0.5) * 0.2;
    }

    const test_values = [_]f32{ -0.017631, -0.013804, 0.052710, 0.085984, 0.029850, 0.001499, -0.085531, 0.026620, 0.068030, 0.011363};
    
    for (0..test_values.len) |i| {
        const orig = test_values[i];
        const gf16_r = gf16ToF32(f32ToGf16(orig));
        const fp16_r = fp16ToF32(f32ToFp16(orig));
        const bf16_r = bf16ToF32(f32ToBf16(orig));
        const ternary_r = ternaryToF32(f32ToTernary(orig));
        
        std.debug.print("{d:.6} | GF16={d:.6} fp16={d:.6} bf16={d:.6} Ternary={d:.6}\n", .{
            orig,
            gf16_r,
            fp16_r,
            bf16_r,
            ternary_r,
        });
    }

    std.debug.print("\nDone!\n", .{});
}
