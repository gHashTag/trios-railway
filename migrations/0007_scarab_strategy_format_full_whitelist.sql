-- Migration 0007: Expand ssot.scarab_strategy.format CHECK to full trainer-igla FormatKind::all()
--
-- Anchor: phi^2 + phi^-2 = 3 · DOI 10.5281/zenodo.19227877
-- Resolves: trios-railway#182
-- Trainer ref: gHashTag/trios-trainer-igla:src/fake_quant.rs::FormatKind::all() @ 0affa84
--
-- Before this migration, only 12 formats were accepted, blocking Tier-1 gf-midrange
-- (gf4/gf8/gf32/gf64), Tier-2 (MX, posit8/32), and 49 other unmeasured formats.
-- This migration expands the whitelist to all 70 canonical formats from the trainer.

ALTER TABLE ssot.scarab_strategy
    DROP CONSTRAINT IF EXISTS scarab_strategy_format_check;

ALTER TABLE ssot.scarab_strategy
    ADD CONSTRAINT scarab_strategy_format_check CHECK (format = ANY (ARRAY[
        -- IEEE float (4 narrow + 4 wide)
        'fp16','bf16','tf32','fp32','fp64','fp80','binary16','binary32','binary128','binary256','f32',
        -- OCP small float (5)
        'fp8_e4m3','fp8_e5m2','fp6_e3m2','fp6_e2m3','fp4_e2m1',
        -- MX Microscaling (3)
        'mxfp4','mxfp6','mxfp8',
        -- Galois Field / Trinity canon (9)
        'gf4','gf8','gf12','gf16','gf20','gf24','gf32','gf64','gf256',
        -- Integer (5)
        'int4','int8','int16','int32','uint8',
        -- NormalFloat / QLoRA (2)
        'nf4','nf8',
        -- Posit (4)
        'posit8','posit16','posit32','posit64',
        -- Logarithmic (1)
        'lns8',
        -- Decimal (3)
        'decimal32','decimal64','decimal128',
        -- Stochastic / advanced (6)
        'stochastic_rnd','stochastic_round','tapered_fp','block_fp','shared_exp','afp',
        -- Unum exotics (6)
        'unum_i','unum_ii','unum_i8','unum_i16','unum_ii8','unum_ii16',
        -- Q-format DSP (3)
        'q_format','q15','q31',
        -- BCD (3)
        'bcd','bcd8','bcd16',
        -- IBM HFP legacy (3)
        'ibm_hfp','ibm_hfp_short','ibm_hfp_long',
        -- VAX float legacy (4)
        'vax_f','vax_d','vax_g','vax_h',
        -- Cray / minifloat (2)
        'cray_float','minifloat'
    ]::text[]));

-- Sanity: format column unchanged, but write protocol now accepts the full trainer registry.
-- Note: 15 of these (fp64, fp80, binary128/256, decimal64/128, vax_d/g/h, cray_float,
-- ibm_hfp_long, posit64, stochastic_rnd, stochastic_round, q31) are UNSUPPORTED in
-- the f32 simulator and silently identity-pass — runtime guard lives in the trainer,
-- not the DB. CHECK only enforces canonical-spelling discipline.
