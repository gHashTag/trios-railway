//! Format quantizer for all 12 precision formats
//!
//! Uses existing format implementations from trios-trainer-igla.
//! Each format implements the FormatQuantizer trait.

use trios_trainer_igla::phi_numbers::PrecisionFormat;
use trios_trainer_igla::phi_numbers::gf4::GF4;
use trios_trainer_igla::phi_numbers::gf20::GF20;
use trios_trainer_igla::phi_numbers::gf24::GF24;
use trios_trainer_igla::phi_numbers::gf32::GF32;
use trios_trainer_igla::phi_numbers::gf64::GF64;
use trios_trainer_igla::phi_numbers::gf8::GF8;
use trios_trainer_igla::phi_numbers::phi_numbers::gf12::GF12;
use trios_trainer_igla::phi_numbers::gf16::GF16;
use trios_trainer_igla::phi_numbers::precision_format::{GF4a, GF6a, FP16, BF16, FP32, Quantize};

/// Trait for format-specific quantization
pub trait FormatQuantizer: Send + Sync {
    /// Name of this format
    fn name(&self) -> &'static str;

    /// Bit width of this format
    fn bit_width(&self) -> u8;

    /// Quantize a single f64 value
    fn quantize(&self, value: f64) -> f64;

    /// Quantize a batch of values
    fn quantize_batch(&self, values: &[f64]) -> Vec<f64> {
        values.iter().map(|&v| self.quantize(v)).collect()
    }
}

// ============================================================================
// GF4 (4-bit: 1:2 split)
// ============================================================================

pub struct GF4Quantizer;

impl FormatQuantizer for GF4Quantizer {
    fn name(&self) -> &'static str {
        "GF4"
    }

    fn bit_width(&self) -> u8 {
        4
    }

    fn quantize(&self, value: f64) -> f64 {
        GF4::from_f32(value as f32).to_f32() as f64
    }
}

// ============================================================================
// GF4a (4-bit adaptive: 1:2 split)
// ============================================================================

pub struct GF4aQuantizer;

impl FormatQuantizer for GF4aQuantizer {
    fn name(&self) -> &'static str {
        "GF4a"
    }

    fn bit_width(&self) -> u8 {
        4
    }

    fn quantize(&self, value: f64) -> f64 {
        GF4a::from_f32(value as f32).to_f32() as f64
    }
}

// ============================================================================
// GF6a (6-bit: 2:3 split)
// ============================================================================

pub struct GF6aQuantizer;

impl FormatQuantizer for GF6aQuantizer {
    fn name(&self) -> &'static str {
        "GF6a"
    }

    fn bit_width(&self) -> u8 {
        6
    }

    fn quantize(&self, value: f64) -> f64 {
        GF6a::from_f32(value as f32).to_f32() as f64
    }
}

// ============================================================================
// GF8 (8-bit: 3:4 split)
// ============================================================================

pub struct GF8Quantizer;

impl FormatQuantizer for GF8Quantizer {
    fn name(&self) -> &'static str {
        "GF8"
    }

    fn bit_width(&self) -> u8 {
        8
    }

    fn quantize(&self, value: f64) -> f64 {
        GF8::from_f32(value as f32).to_f32() as f64
    }
}

// ============================================================================
// GF12 (12-bit: 4:7 split)
// ============================================================================

pub struct GF12Quantizer;

impl FormatQuantizer for GF12Quantizer {
    fn name(&self) -> &'static str {
        "GF12"
    }

    fn bit_width(&self) -> u8 {
        12
    }

    fn quantize(&self, value: f64) -> f64 {
        use trios_trainer_igla::phi_numbers::GF12;
        GF12::from_f32(value as f32).to_f32() as f64
    }
}

// ============================================================================
// GF16 (16-bit: 6:9 split)
// ============================================================================

pub struct GF16Quantizer;

impl FormatQuantizer for GF16Quantizer {
    fn name(&self) -> &'static str {
        "GF16"
    }

    fn bit_width(&self) -> u8 {
        16
    }

    fn quantize(&self, value: f64) -> f64 {
        GF16::from_f32(value as f32).to_f32() as f64
    }
}

// ============================================================================
// GF32 (32-bit: 13:18 split)
// ============================================================================

pub struct GF32Quantizer;

impl FormatQuantizer for GF32Quantizer {
    fn name(&self) -> &'static str {
        "GF32"
    }

    fn bit_width(&self) -> u8 {
        32
    }

    fn quantize(&self, value: f64) -> f64 {
        GF32::from_f32(value as f32).to_f32() as f64
    }
}

// ============================================================================
// GF64 (64-bit: 21:42 split)
// ============================================================================

pub struct GF64Quantizer;

impl FormatQuantizer for GF64Quantizer {
    fn name(&self) -> &'static str {
        "GF64"
    }

    fn bit_width(&self) -> u8 {
        64
    }

    fn quantize(&self, value: f64) -> f64 {
        GF64::from_f64(value).to_f64()
    }
}

// ============================================================================
// GF20 (20-bit: 7:12 split)
// ============================================================================

pub struct GF20Quantizer;

impl FormatQuantizer for GF20Quantizer {
    fn name(&self) -> &'static str {
        "GF20"
    }

    fn bit_width(&self) -> u8 {
        20
    }

    fn quantize(&self, value: f64) -> f64 {
        use trios_trainer_igla::phi_numbers::GF20;
        GF20::from_f32(value as f32).to_f32() as f64
    }
}

// ============================================================================
// GF24 (24-bit: 9:14 split)
// ============================================================================

pub struct GF24Quantizer;

impl FormatQuantizer for GF24Quantizer {
    fn name(&self) -> &'static str {
        "GF24"
    }

    fn bit_width(&self) -> u8 {
        24
    }

    fn quantize(&self, value: f64) -> f64 {
        use trios_trainer_igla::phi_numbers::GF24;
        GF24::from_f32(value as f32).to_f32() as f64
    }
}

// ============================================================================
// FP16 (IEEE half precision: 5:10 split)
// ============================================================================

pub struct Fp16Quantizer;

impl FormatQuantizer for Fp16Quantizer {
    fn name(&self) -> &'static str {
        "FP16"
    }

    fn bit_width(&self) -> u8 {
        16
    }

    fn quantize(&self, value: f64) -> f64 {
        FP16::from_f32(value as f32).to_f32() as f64
    }
}

// ============================================================================
// BF16 (Brain float: 8:7 split)
// ============================================================================

pub struct Bf16Quantizer;

impl FormatQuantizer for Bf16Quantizer {
    fn name(&self) -> &'static str {
        "BF16"
    }

    fn bit_width(&self) -> u8 {
        16
    }

    fn quantize(&self, value: f64) -> f64 {
        BF16::from_f32(value as f32).to_f32() as f64
    }
}

// ============================================================================
// FP32 (IEEE single precision: baseline)
// ============================================================================

pub struct Fp32Quantizer;

impl FormatQuantizer for Fp32Quantizer {
    fn name(&self) -> &'static str {
        "FP32"
    }

    fn bit_width(&self) -> u8 {
        32
    }

    fn quantize(&self, value: f64) -> f64 {
        // Identity for baseline
        value
    }
}

// ============================================================================
// Format factory
// ============================================================================

/// Create quantizer for a given format name
pub fn create_quantizer(format_name: &str) -> Option<Box<dyn FormatQuantizer>> {
    match format_name {
        "GF4" => Some(Box::new(GF4Quantizer)),
        "GF4a" => Some(Box::new(GF4aQuantizer)),
        "GF6a" => Some(Box::new(GF6aQuantizer)),
        "GF8" => Some(Box::new(GF8Quantizer)),
        "GF12" => Some(Box::new(GF12Quantizer)),
        "GF16" => Some(Box::new(GF16Quantizer)),
        "GF20" => Some(Box::new(GF20Quantizer)),
        "GF24" => Some(Box::new(GF24Quantizer)),
        "GF32" => Some(Box::new(GF32Quantizer)),
        "GF64" => Some(Box::new(GF64Quantizer)),
        "FP16" => Some(Box::new(Fp16Quantizer)),
        "BF16" => Some(Box::new(Bf16Quantizer)),
        "FP32" => Some(Box::new(Fp32Quantizer)),
        _ => None,
    }
}

/// Create all 12 quantizers
pub fn all_quantizers() -> Vec<Box<dyn FormatQuantizer>> {
    vec![
        Box::new(GF4Quantizer),
        Box::new(GF4aQuantizer),
        Box::new(GF6aQuantizer),
        Box::new(GF8Quantizer),
        Box::new(GF12Quantizer),
        Box::new(GF16Quantizer),
        Box::new(GF20Quantizer),
        Box::new(GF24Quantizer),
        Box::new(GF32Quantizer),
        Box::new(GF64Quantizer),
        Box::new(Fp16Quantizer),
        Box::new(Bf16Quantizer),
        Box::new(Fp32Quantizer),
    ]
}
