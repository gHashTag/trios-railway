-- 0006_numeric_format_registry.sql
-- SSOT table for 63 numeric formats (trios-numeric-catalog).
-- One row per format token; tier/runnable/bits/metadata from the Rust enum.

CREATE TABLE IF NOT EXISTS numeric_format_registry (
    token       TEXT PRIMARY KEY,           -- e.g. "gf16", "fp8_e4m3"
    tier        TEXT NOT NULL,              -- T1..T9
    runnable    BOOLEAN NOT NULL DEFAULT FALSE,
    bits        INT,
    exp_bits    INT,
    mant_bits   INT,
    storage     TEXT,                       -- U8, U16, U32, U64, U128, Variable
    phi_distance DOUBLE PRECISION,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- T1: runnable (4 formats)
INSERT INTO numeric_format_registry (token, tier, runnable, bits, exp_bits, mant_bits, storage, phi_distance, description) VALUES
    ('binary32',  'T1', TRUE, 32, 8,  23, 'U32', 1.618033988749895, 'IEEE 754 single precision (32-bit)'),
    ('binary16',  'T1', TRUE, 16, 5,  10, 'U16', 0.618033988749895, 'IEEE 754 half precision (16-bit)'),
    ('bfloat16',  'T1', TRUE, 16, 8,   7, 'U16', 1.0,               'Google Brain float16 (16-bit, 8-exp)'),
    ('gf16',      'T1', TRUE, 16, 5,  10, 'U16', 0.0,               'Golden Float 16 (φ-anchored, 16-bit)')
ON CONFLICT (token) DO UPDATE SET
    runnable = EXCLUDED.runnable,
    tier = EXCLUDED.tier,
    bits = EXCLUDED.bits,
    exp_bits = EXCLUDED.exp_bits,
    mant_bits = EXCLUDED.mant_bits,
    storage = EXCLUDED.storage,
    phi_distance = EXCLUDED.phi_distance,
    description = EXCLUDED.description;

-- T2: near-runnable (10 formats)
INSERT INTO numeric_format_registry (token, tier, runnable, bits, exp_bits, mant_bits, storage, phi_distance, description) VALUES
    ('binary64',   'T2', FALSE, 64, 11, 52, 'U64', 2.618033988749895, 'IEEE 754 double precision (64-bit)'),
    ('tf32',       'T2', FALSE, 32, 8,  10, 'U32', 1.0,               'NVIDIA TensorFloat-32 (19-bit: 8-exp, 10-mant)'),
    ('fp8_e4m3',   'T2', FALSE,  8, 4,   3, 'U8',  1.23606797749979, 'FP8 E4M3 (NVIDIA/AMD, 8-bit)'),
    ('fp8_e5m2',   'T2', FALSE,  8, 5,   2, 'U8',  1.38196601125011, 'FP8 E5M2 (NVIDIA/AMD, 8-bit)'),
    ('gf8',        'T2', FALSE,  8, 4,   3, 'U8',  0.38196601125011, 'Golden Float 8 (φ-anchored, 8-bit)'),
    ('gf32',       'T2', FALSE, 32, 8,  23, 'U32', 0.23606797749979, 'Golden Float 32 (φ-anchored, 32-bit)'),
    ('int8',       'T2', FALSE,  8, NULL, NULL, 'U8', NULL, 'Signed integer 8-bit'),
    ('int16',      'T2', FALSE, 16, NULL, NULL, 'U16', NULL, 'Signed integer 16-bit'),
    ('int32',      'T2', FALSE, 32, NULL, NULL, 'U32', NULL, 'Signed integer 32-bit'),
    ('uint8',      'T2', FALSE,  8, NULL, NULL, 'U8', NULL, 'Unsigned integer 8-bit')
ON CONFLICT (token) DO UPDATE SET
    tier = EXCLUDED.tier,
    bits = EXCLUDED.bits,
    exp_bits = EXCLUDED.exp_bits,
    mant_bits = EXCLUDED.mant_bits,
    storage = EXCLUDED.storage,
    phi_distance = EXCLUDED.phi_distance,
    description = EXCLUDED.description;

-- T3: research — Posit/LNS/block-FP (14 formats)
INSERT INTO numeric_format_registry (token, tier, runnable, bits, storage, description) VALUES
    ('posit8',             'T3', FALSE,  8, 'U8',  'Posit<8,0> (Type III unum)'),
    ('posit16',            'T3', FALSE, 16, 'U16', 'Posit<16,1> (Type III unum)'),
    ('posit32',            'T3', FALSE, 32, 'U32', 'Posit<32,2> (Type III unum)'),
    ('posit64',            'T3', FALSE, 64, 'U64', 'Posit<64,3> (Type III unum)'),
    ('lns8',               'T3', FALSE,  8, 'U8',  'Logarithmic Number System (8-bit)'),
    ('mxfp8',              'T3', FALSE,  8, 'U8',  'OCP Microscaling FP8'),
    ('mxfp6',              'T3', FALSE,  6, 'U8',  'OCP Microscaling FP6'),
    ('mxfp4',              'T3', FALSE,  4, 'U8',  'OCP Microscaling FP4'),
    ('nf4',                'T3', FALSE,  4, 'U8',  'NormalFloat-4 (QLoRA quantization)'),
    ('afp',                'T3', FALSE,  8, 'U8',  'Alternating Float Point'),
    ('block_fp',           'T3', FALSE,  8, 'U8',  'Block Floating Point'),
    ('shared_exponent',    'T3', FALSE,  8, 'U8',  'Shared Exponent (bfloat-like)'),
    ('stochastic_rounding','T3', FALSE,  8, 'U8',  'Stochastic Rounding wrapper'),
    ('tapered_fp',         'T3', FALSE,  8, 'U8',  'Tapered Floating Point')
ON CONFLICT (token) DO NOTHING;

-- T4: exotic wide (6 formats)
INSERT INTO numeric_format_registry (token, tier, runnable, bits, storage, description) VALUES
    ('binary128',    'T4', FALSE, 128, 'U128', 'IEEE 754 quad precision (128-bit)'),
    ('binary256',    'T4', FALSE, 256, 'U128', 'IEEE 754 oct precision (256-bit)'),
    ('double_double','T4', FALSE, 128, 'U128', 'Double-Double arithmetic (128-bit effective)'),
    ('quad_double',  'T4', FALSE, 256, 'U128', 'Quad-Double arithmetic (256-bit effective)'),
    ('fp80',         'T4', FALSE,  80, 'U128', 'Intel extended precision (80-bit)'),
    ('int128',       'T4', FALSE, 128, 'U128', 'Signed integer 128-bit')
ON CONFLICT (token) DO NOTHING;

-- T5: sub-byte / micro (9 formats)
INSERT INTO numeric_format_registry (token, tier, runnable, bits, exp_bits, mant_bits, storage, phi_distance, description) VALUES
    ('fp6_e3m2', 'T5', FALSE, 6, 3, 2, 'U8', NULL, 'FP6 E3M2 (6-bit float)'),
    ('fp6_e2m3', 'T5', FALSE, 6, 2, 3, 'U8', NULL, 'FP6 E2M3 (6-bit float)'),
    ('fp4_e2m1', 'T5', FALSE, 4, 2, 1, 'U8', NULL, 'FP4 E2M1 (4-bit float)'),
    ('gf4',      'T5', FALSE, 4, 2, 1, 'U8', 0.14589803375031, 'Golden Float 4 (φ-anchored, 4-bit)'),
    ('gf12',     'T5', FALSE, 12, 4, 7, 'U16', 0.09016994374947, 'Golden Float 12 (φ-anchored, 12-bit)'),
    ('gf20',     'T5', FALSE, 20, 5, 14, 'U32', 0.05572809000084, 'Golden Float 20 (φ-anchored, 20-bit)'),
    ('gf24',     'T5', FALSE, 24, 6, 17, 'U32', 0.03444185374863, 'Golden Float 24 (φ-anchored, 24-bit)'),
    ('int4',     'T5', FALSE,  4, NULL, NULL, 'U8', NULL, 'Signed integer 4-bit'),
    ('uint4',    'T5', FALSE,  4, NULL, NULL, 'U8', NULL, 'Unsigned integer 4-bit')
ON CONFLICT (token) DO NOTHING;

-- T6: decimal (3 formats)
INSERT INTO numeric_format_registry (token, tier, runnable, bits, exp_bits, storage, description) VALUES
    ('decimal32',  'T6', FALSE, 32,  8, 'U32', 'IEEE 754-2008 decimal32'),
    ('decimal64',  'T6', FALSE, 64, 10, 'U64', 'IEEE 754-2008 decimal64'),
    ('decimal128', 'T6', FALSE,128, 14, 'U128','IEEE 754-2008 decimal128')
ON CONFLICT (token) DO NOTHING;

-- T7: historical / archival (7 formats)
INSERT INTO numeric_format_registry (token, tier, runnable, bits, storage, description) VALUES
    ('ibm_hfp',   'T7', FALSE, 32, 'U32', 'IBM Hexadecimal Floating Point (32-bit)'),
    ('mbf',       'T7', FALSE, 32, 'U32', 'Microsoft Binary Format (32-bit)'),
    ('vax_f',     'T7', FALSE, 32, 'U32', 'VAX F_floating (32-bit)'),
    ('vax_d',     'T7', FALSE, 64, 'U64', 'VAX D_floating (64-bit)'),
    ('vax_g',     'T7', FALSE, 64, 'U64', 'VAX G_floating (64-bit)'),
    ('vax_h',     'T7', FALSE,128, 'U128','VAX H_floating (128-bit)'),
    ('cray_float','T7', FALSE, 64, 'U64', 'Cray floating point (64-bit)')
ON CONFLICT (token) DO NOTHING;

-- T8: unum family (2 formats)
INSERT INTO numeric_format_registry (token, tier, runnable, bits, storage, description) VALUES
    ('unum1', 'T8', FALSE, 0, 'Variable', 'Unum Type I (variable width)'),
    ('unum2', 'T8', FALSE, 0, 'Variable', 'Unum Type II (Posit + valids)')
ON CONFLICT (token) DO NOTHING;

-- T9: fixed / encoded (8 formats)
INSERT INTO numeric_format_registry (token, tier, runnable, bits, storage, description) VALUES
    ('q_format',     'T9', FALSE, 16, 'U16', 'Q-format fixed point (16-bit)'),
    ('bcd',          'T9', FALSE,  8, 'U8',  'Binary Coded Decimal (8-bit)'),
    ('minifloat',    'T9', FALSE,  8, 'U8',  'Minifloat (custom 8-bit)'),
    ('int64',        'T9', FALSE, 64, 'U64', 'Signed integer 64-bit'),
    ('uint64',       'T9', FALSE, 64, 'U64', 'Unsigned integer 64-bit'),
    ('uint128',      'T9', FALSE,128, 'U128','Unsigned integer 128-bit'),
    ('saturated_fp', 'T9', FALSE,  8, 'U8',  'Saturated floating point (8-bit)'),
    ('fixed_accum',  'T9', FALSE, 32, 'U32', 'Fixed-point accumulator (32-bit)')
ON CONFLICT (token) DO NOTHING;

-- bpb_samples honesty flag
ALTER TABLE bpb_samples ADD COLUMN IF NOT EXISTS format_honest BOOLEAN;

-- Index for fast tier/runnable lookups
CREATE INDEX IF NOT EXISTS idx_format_registry_tier ON numeric_format_registry (tier);
CREATE INDEX IF NOT EXISTS idx_format_registry_runnable ON numeric_format_registry (runnable) WHERE runnable = TRUE;
