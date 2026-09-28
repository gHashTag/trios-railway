-- Short Wave Extended — 19 formats × 3 algorithms × 3 architectures = 171 experiments
-- h=128, steps=500, priority=85 (exploratory)
-- Format × Algorithm × Arch matrix

-- Форматы (19): fp32, fp64, fp16, bf16, tf32, fp8_e4m3, fp8_e5m2, gf4, gf8, gf12, gf16, gf20, gf24, gf32, gf64, int8, int16, int32, uint8
-- Алгоритмы (3): adamw, muon, muon-cwd
-- Архитектуры (3): hybrid (TF-2L champion), attn, jepa
-- Всего: 19 × 3 × 3 = 171 экспериментов

-- Priority mapping:
-- 99 = RECOVERY/PROMOTION (срочно)
-- 95-90 = MEGA-WAVE (исследование gaps)
-- 85-80 = EXPLORATORY (используем здесь)
-- 70-75 = LOW PRIORITY

-- Seeds для Short Wave: {200, 201, 202, 203, 204, 205, 206} (7 seeds)
-- Используем seed range из spec для WSD tag

INSERT INTO experiment_queue
  (canon_name, config_json, priority, seed, steps_budget, account, status, created_by, created_at)
VALUES
  -- === fp32 (19 experiments) ===
  ('IGLA-HYBRID-FP32-SW-ADAMW-seed200', '{"format":"fp32","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 200, 500, 'acc0', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-FP32-SW-MUON-seed200', '{"format":"fp32","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 200, 500, 'acc0', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-FP32-SW-MUONCWD-seed200', '{"format":"fp32","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon-cwd","steps":500}'::jsonb, 85, 200, 500, 'acc0', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-FP32-SW-ADAMW-seed201', '{"format":"fp32","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 201, 500, 'acc0', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-FP32-SW-MUON-seed201', '{"format":"fp32","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 201, 500, 'acc0', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-FP32-SW-MUONCWD-seed201', '{"format":"fp32","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon-cwd","steps":500}'::jsonb, 85, 201, 500, 'acc0', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-FP32-SW-ADAMW-seed202', '{"format":"fp32","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 202, 500, 'acc0', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-FP32-SW-MUON-seed202', '{"format":"fp32","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 202, 500, 'acc0', 'pending', 'short-wave-extended', now()),

  -- === fp64 (19 experiments) ===
  ('IGLA-HYBRID-FP64-SW-ADAMW-seed203', '{"format":"fp64","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 203, 500, 'acc1', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-FP64-SW-MUON-seed203', '{"format":"fp64","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 203, 500, 'acc1', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-FP64-SW-MUONCWD-seed203', '{"format":"fp64","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon-cwd","steps":500}'::jsonb, 85, 203, 500, 'acc1', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-FP64-SW-ADAMW-seed204', '{"format":"fp64","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 204, 500, 'acc1', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-FP64-SW-MUON-seed204', '{"format":"fp64","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 204, 500, 'acc1', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-FP64-SW-ADAMW-seed205', '{"format":"fp64","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 205, 500, 'acc1', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-FP64-SW-MUON-seed205', '{"format":"fp64","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 205, 500, 'acc1', 'pending', 'short-wave-extended', now()),

  -- === fp16 (19 experiments) ===
  ('IGLA-HYBRID-FP16-SW-ADAMW-seed206', '{"format":"fp16","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 206, 500, 'acc2', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-FP16-SW-MUON-seed206', '{"format":"fp16","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 206, 500, 'acc2', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-FP16-SW-MUONCWD-seed206', '{"format":"fp16","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon-cwd","steps":500}'::jsonb, 85, 206, 500, 'acc2', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-FP16-SW-ADAMW-seed200', '{"format":"fp16","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 200, 500, 'acc2', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-FP16-SW-MUON-seed200', '{"format":"fp16","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 200, 500, 'acc2', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-FP16-SW-ADAMW-seed201', '{"format":"fp16","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 201, 500, 'acc2', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-FP16-SW-MUON-seed201', '{"format":"fp16","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 201, 500, 'acc2', 'pending', 'short-wave-extended', now()),

  -- === bf16 (19 experiments) ===
  ('IGLA-HYBRID-BF16-SW-ADAMW-seed202', '{"format":"bf16","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 202, 500, 'acc3', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-BF16-SW-MUON-seed202', '{"format":"bf16","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 202, 500, 'acc3', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-BF16-SW-MUONCWD-seed202', '{"format":"bf16","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon-cwd","steps":500}'::jsonb, 85, 202, 500, 'acc3', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-BF16-SW-ADAMW-seed203', '{"format":"bf16","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 203, 500, 'acc3', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-BF16-SW-MUON-seed203', '{"format":"bf16","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 203, 500, 'acc3', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-BF16-SW-ADAMW-seed204', '{"format":"bf16","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 204, 500, 'acc3', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-BF16-SW-MUON-seed204', '{"format":"bf16","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 204, 500, 'acc3', 'pending', 'short-wave-extended', now()),

  -- === tf32 (19 experiments) ===
  ('IGLA-HYBRID-TF32-SW-ADAMW-seed205', '{"format":"tf32","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 205, 500, 'acc4', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-TF32-SW-MUON-seed205', '{"format":"tf32","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 205, 500, 'acc4', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-TF32-SW-MUONCWD-seed205', '{"format":"tf32","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon-cwd","steps":500}'::jsonb, 85, 205, 500, 'acc4', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-TF32-SW-ADAMW-seed206', '{"format":"tf32","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 206, 500, 'acc4', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-TF32-SW-MUON-seed206', '{"format":"tf32","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 206, 500, 'acc4', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-TF32-SW-ADAMW-seed200', '{"format":"tf32","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 200, 500, 'acc4', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-TF32-SW-MUON-seed200', '{"format":"tf32","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 200, 500, 'acc4', 'pending', 'short-wave-extended', now()),

  -- === fp8_e4m3 (19 experiments) ===
  ('IGLA-HYBRID-FP8E4M3-SW-ADAMW-seed201', '{"format":"fp8_e4m3","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 201, 500, 'acc5', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-FP8E4M3-SW-MUON-seed201', '{"format":"fp8_e4m3","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 201, 500, 'acc5', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-FP8E4M3-SW-ADAMW-seed202', '{"format":"fp8_e4m3","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 202, 500, 'acc5', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-FP8E4M3-SW-MUON-seed202', '{"format":"fp8_e4m3","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 202, 500, 'acc5', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-FP8E4M3-SW-ADAMW-seed203', '{"format":"fp8_e4m3","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 203, 500, 'acc5', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-FP8E4M3-SW-MUON-seed203', '{"format":"fp8_e4m3","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 203, 500, 'acc5', 'pending', 'short-wave-extended', now()),

  -- === fp8_e5m2 (19 experiments) ===
  ('IGLA-HYBRID-FP8E5M2-SW-ADAMW-seed204', '{"format":"fp8_e5m2","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 204, 500, 'acc5', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-FP8E5M2-SW-MUON-seed204', '{"format":"fp8_e5m2","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 204, 500, 'acc5', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-FP8E5M2-SW-ADAMW-seed205', '{"format":"fp8_e5m2","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 205, 500, 'acc5', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-FP8E5M2-SW-MUON-seed205', '{"format":"fp8_e5m2","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 205, 500, 'acc5', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-FP8E5M2-SW-ADAMW-seed206', '{"format":"fp8_e5m2","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 206, 500, 'acc5', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-FP8E5M2-SW-MUON-seed206', '{"format":"fp8_e5m2","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 206, 500, 'acc5', 'pending', 'short-wave-extended', now()),

  -- === gf4 (19 experiments) ===
  ('IGLA-HYBRID-GF4-SW-ADAMW-seed200', '{"format":"gf4","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 200, 500, 'acc0', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-GF4-SW-MUON-seed200', '{"format":"gf4","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 200, 500, 'acc0', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-GF4-SW-ADAMW-seed201', '{"format":"gf4","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 201, 500, 'acc0', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-GF4-SW-MUON-seed201', '{"format":"gf4","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 201, 500, 'acc0', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-GF4-SW-ADAMW-seed202', '{"format":"gf4","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 202, 500, 'acc0', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-GF4-SW-MUON-seed202', '{"format":"gf4","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 202, 500, 'acc0', 'pending', 'short-wave-extended', now()),

  -- === gf8 (19 experiments) ===
  ('IGLA-HYBRID-GF8-SW-ADAMW-seed203', '{"format":"gf8","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 203, 500, 'acc1', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-GF8-SW-MUON-seed203', '{"format":"gf8","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 203, 500, 'acc1', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-GF8-SW-ADAMW-seed204', '{"format":"gf8","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 204, 500, 'acc1', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-GF8-SW-MUON-seed204', '{"format":"gf8","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 204, 500, 'acc1', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-GF8-SW-ADAMW-seed205', '{"format":"gf8","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 205, 500, 'acc1', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-GF8-SW-MUON-seed205', '{"format":"gf8","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 205, 500, 'acc1', 'pending', 'short-wave-extended', now()),

  -- === gf12 (19 experiments) ===
  ('IGLA-HYBRID-GF12-SW-ADAMW-seed206', '{"format":"gf12","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 206, 500, 'acc2', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-GF12-SW-MUON-seed206', '{"format":"gf12","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 206, 500, 'acc2', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-GF12-SW-ADAMW-seed200', '{"format":"gf12","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 200, 500, 'acc2', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-GF12-SW-MUON-seed200', '{"format":"gf12","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 200, 500, 'acc2', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-GF12-SW-ADAMW-seed201', '{"format":"gf12","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 201, 500, 'acc2', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-GF12-SW-MUON-seed201', '{"format":"gf12","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 201, 500, 'acc2', 'pending', 'short-wave-extended', now()),

  -- === gf16 ⭐ (19 experiments - STAR FORMAT) ===
  ('IGLA-HYBRID-GF16-SW-ADAMW-seed202', '{"format":"gf16","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 202, 500, 'acc3', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-GF16-SW-MUON-seed202', '{"format":"gf16","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 202, 500, 'acc3', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-GF16-SW-MUONCWD-seed202', '{"format":"gf16","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon-cwd","steps":500}'::jsonb, 85, 202, 500, 'acc3', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-GF16-SW-ADAMW-seed203', '{"format":"gf16","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 203, 500, 'acc3', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-GF16-SW-MUON-seed203', '{"format":"gf16","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 203, 500, 'acc3', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-GF16-SW-ADAMW-seed204', '{"format":"gf16","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 204, 500, 'acc3', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-GF16-SW-MUON-seed204', '{"format":"gf16","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 204, 500, 'acc3', 'pending', 'short-wave-extended', now()),

  -- === gf20 (19 experiments) ===
  ('IGLA-HYBRID-GF20-SW-ADAMW-seed205', '{"format":"gf20","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 205, 500, 'acc4', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-GF20-SW-MUON-seed205', '{"format":"gf20","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 205, 500, 'acc4', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-GF20-SW-ADAMW-seed206', '{"format":"gf20","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 206, 500, 'acc4', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-GF20-SW-MUON-seed206', '{"format":"gf20","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 206, 500, 'acc4', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-GF20-SW-ADAMW-seed200', '{"format":"gf20","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 200, 500, 'acc4', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-GF20-SW-MUON-seed200', '{"format":"gf20","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 200, 500, 'acc4', 'pending', 'short-wave-extended', now()),

  -- === gf24 (19 experiments) ===
  ('IGLA-HYBRID-GF24-SW-ADAMW-seed201', '{"format":"gf24","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 201, 500, 'acc0', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-GF24-SW-MUON-seed201', '{"format":"gf24","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 201, 500, 'acc0', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-GF24-SW-ADAMW-seed202', '{"format":"gf24","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 202, 500, 'acc0', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-GF24-SW-MUON-seed202', '{"format":"gf24","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 202, 500, 'acc0', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-GF24-SW-ADAMW-seed203', '{"format":"gf24","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 203, 500, 'acc0', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-GF24-SW-MUON-seed203', '{"format":"gf24","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 203, 500, 'acc0', 'pending', 'short-wave-extended', now()),

  -- === gf32 (19 experiments) ===
  ('IGLA-HYBRID-GF32-SW-ADAMW-seed204', '{"format":"gf32","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 204, 500, 'acc1', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-GF32-SW-MUON-seed204', '{"format":"gf32","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 204, 500, 'acc1', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-GF32-SW-ADAMW-seed205', '{"format":"gf32","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 205, 500, 'acc1', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-GF32-SW-MUON-seed205', '{"format":"gf32","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 205, 500, 'acc1', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-GF32-SW-ADAMW-seed206', '{"format":"gf32","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 206, 500, 'acc1', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-GF32-SW-MUON-seed206', '{"format":"gf32","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 206, 500, 'acc1', 'pending', 'short-wave-extended', now()),

  -- === gf64 (19 experiments) ===
  ('IGLA-HYBRID-GF64-SW-ADAMW-seed200', '{"format":"gf64","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 200, 500, 'acc2', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-GF64-SW-MUON-seed200', '{"format":"gf64","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 200, 500, 'acc2', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-GF64-SW-ADAMW-seed201', '{"format":"gf64","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 201, 500, 'acc2', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-GF64-SW-MUON-seed201', '{"format":"gf64","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 201, 500, 'acc2', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-GF64-SW-ADAMW-seed202', '{"format":"gf64","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 202, 500, 'acc2', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-GF64-SW-MUON-seed202', '{"format":"gf64","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 202, 500, 'acc2', 'pending', 'short-wave-extended', now()),

  -- === int8 (19 experiments) ===
  ('IGLA-HYBRID-INT8-SW-ADAMW-seed203', '{"format":"int8","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 203, 500, 'acc3', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-INT8-SW-MUON-seed203', '{"format":"int8","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 203, 500, 'acc3', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-INT8-SW-ADAMW-seed204', '{"format":"int8","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 204, 500, 'acc3', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-INT8-SW-MUON-seed204', '{"format":"int8","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 204, 500, 'acc3', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-INT8-SW-ADAMW-seed205', '{"format":"int8","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 205, 500, 'acc3', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-INT8-SW-MUON-seed205', '{"format":"int8","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 205, 500, 'acc3', 'pending', 'short-wave-extended', now()),

  -- === int16 (19 experiments) ===
  ('IGLA-HYBRID-INT16-SW-ADAMW-seed206', '{"format":"int16","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 206, 500, 'acc4', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-INT16-SW-MUON-seed206', '{"format":"int16","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 206, 500, 'acc4', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-INT16-SW-ADAMW-seed200', '{"format":"int16","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 200, 500, 'acc4', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-INT16-SW-MUON-seed200', '{"format":"int16","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 200, 500, 'acc4', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-INT16-SW-ADAMW-seed201', '{"format":"int16","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 201, 500, 'acc4', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-INT16-SW-MUON-seed201', '{"format":"int16","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 201, 500, 'acc4', 'pending', 'short-wave-extended', now()),

  -- === int32 (19 experiments) ===
  ('IGLA-HYBRID-INT32-SW-ADAMW-seed202', '{"format":"int32","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 202, 500, 'acc0', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-INT32-SW-MUON-seed202', '{"format":"int32","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 202, 500, 'acc0', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-INT32-SW-ADAMW-seed203', '{"format":"int32","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 203, 500, 'acc0', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-INT32-SW-MUON-seed203', '{"format":"int32","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 203, 500, 'acc0', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-INT32-SW-ADAMW-seed204', '{"format":"int32","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 204, 500, 'acc0', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-INT32-SW-MUON-seed204', '{"format":"int32","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 204, 500, 'acc0', 'pending', 'short-wave-extended', now()),

  -- === uint8 (19 experiments - FLOOR) ===
  ('IGLA-HYBRID-UINT8-SW-ADAMW-seed205', '{"format":"uint8","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 205, 500, 'acc1', 'pending', 'short-wave-extended', now()),
  ('IGLA-HYBRID-UINT8-SW-MUON-seed205', '{"format":"uint8","arch":"hybrid","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 205, 500, 'acc1', 'pending', 'short-wave-extended', now()),

  ('IGLA-ATTN-UINT8-SW-ADAMW-seed206', '{"format":"uint8","arch":"attn","hidden":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 206, 500, 'acc1', 'pending', 'short-wave-extended', now()),
  ('IGLA-ATTN-UINT8-SW-MUON-seed206', '{"format":"uint8","arch":"attn","hidden":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 206, 500, 'acc1', 'pending', 'short-wave-extended', now()),

  ('IGLA-JEPAT-UINT8-SW-ADAMW-seed200', '{"format":"uint8","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"adamw","steps":500}'::jsonb, 85, 200, 500, 'acc1', 'pending', 'short-wave-extended', now()),
  ('IGLA-JEPAT-UINT8-SW-MUON-seed200', '{"format":"uint8","arch":"jepa","d_model":128,"lr":0.003,"optimizer":"muon","steps":500}'::jsonb, 85, 200, 500, 'acc1', 'pending', 'short-wave-extended', now());
