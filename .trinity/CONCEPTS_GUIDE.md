# IGLA RACE — NTA, Гибриды, PHI — Полный гайд

## 📐 Введение

Документация по всем ключевым концепциям IGLA RACE:
- **NTA** — Neural Tensor Approximation
- **Гибриды** — Hybrid архитектуры
- **PHI** — Golden Float Family (φ-оптимизированные форматы)
- **Лучшие результаты** — Где найти best эксперименты

---

## 1. PHI — Golden Float Family

### Суть

φ (фи) = 1.618033988749... — золотое сечение
Форматы оптимизированы вокруг φ-отношений для лучшей точности.

### PHI Форматы (phi_numbers/)

| Формат | EXP:MANT | Total bits | φ-distance | Tier | Описание |
|---------|------------|------------|-------------|------|----------|
| **GF4** | 1:2 | 4 | 0.146 (φ⁻⁴) | T5 | Sub-byte, 2-bit exp + 1-bit mantissa |
| **GF8** | 3:4 | 8 | 0.382 (φ⁻²) | T2 | 8-bit, balanced φ-split |
| **GF12** | 4:7 | 12 | 0.090 (φ⁻⁵) | T2 | 12-bit, φ-heavy |
| **GF16** ⭐ | 5:9 | 16 | 0.0 (reference) | **T1** | Champion format! |
| GF20 | 6:13 | 20 | 0.056 (φ⁻⁶) | T2 | 20-bit, wide mantissa |
| GF24 | 8:15 | 24 | 0.034 (φ⁻⁷) | T2 | 24-bit, near-fp32 |
| GF32 | 11:19 | 32 | 0.236 (φ⁻³) | T2 | 32-bit, φ-cube |
| GF64 | 23:39 | 64 | 2.618 (φ²) | T2 | 64-bit, φ-square |

**φ-distance:** чем ниже, тем ближе к GF16 (reference, distance=0).

### PHI Свойства

```rust
// Основные константы (phi_constants.rs)
pub const PHI: f64 = 1.6180339887498948;
pub const PHI_SQUARED: f64 = 2.6180339887498948;  // φ²
pub const PHI_CUBED: f64 = 4.2360679774997897;    // φ³

// GF16 — фи-анкор, 6:9 split (≈2/3)
pub const GF16_EXP_BITS: u8 = 6;
pub const GF16_MANT_BITS: u8 = 9;
pub const GF16_SPLIT_RATIO: f64 = 6.0 / 9.0;
```

### Trinity Identity

```
φ² + 1/φ² = 3
```

Это фундаментальное уравнение всей IGLA архитектуры.

---

## 2. NTA — Neural Tensor Approximation

**Статус:** ⚠️ В разработке, не активно в IGLA RACE

### Предназначение

NTA — это архитектура для аппроксимации тензоров с:
- Сжатием параметров модели (compression-aware training)
- Низкой точностью хранения (quantization)
- Эффективной реконструкцией (decoding-aware training)

### Потенциальные преимущества

- 🟢 Компактное хранение (меньше память, быстрее inference)
- 🟢 Низкая энергоэффективность (меньше FLOPs)
- 🟡 Потенциальная потеря точности

### Почему не в IGLA RACE

1. IGLA RACE фокусируется на численные форматы точности (FP/INT/GF), а не на архитектурную оптимизацию
2. NTA требует дополнительной инфраструктуры (compression/decompression pipeline)
3. Gate-2 цель: BPB ≤ 1.85 — NTA может помочь или мешать

### Будущее направление

Если внедрить: может быть отдельной ланой "NTA-RACE" для тестирования:
- NTA + GF16 (фиксированное поле: 2.4–6.4× против конусных форматов на троичной сети)
- NTA + GF8 (баланс точность/размер)
- НTA + различные архитектуры

---

## 3. Гибриды — Hybrid Architecture

### Что это

Hybrid Attention — комбинация ngram + attention в одной архитектуре.

### Структура (model_hybrid_attn.rs)

```
Embedding → N-gram (ctx-weighted) → LayerNorm → [Projection + HybridAttn] → Hidden → LM Head
```

### Ключевые компоненты

| Компонент | Описание | Параметры |
|------------|----------|-----------|
| Embedding | Токен эмбеддинги | VOCAB × DIM = 128×64 |
| N-gram ctx | Контекстное окно | NUM_CTX=6, CTX_WEIGHTS=[0.70, 0.45, ...] |
| LayerNorm | Нормализация | eps=1e-5 |
| Projection | Линейная проекция | hidden × DIM |
| **HybridAttn** | Causal self-attention | d_model=384, attn_layers={1,2} |
| LM Head | Логит-регрессия | VOCAB × hidden |

### φ-Safe Constraints

```rust
// INV-1: LR должен быть в безопасном диапазоне
pub const LR_SAFE_MIN: f64 = 0.002;
pub const LR_SAFE_MAX: f64 = 0.007;

// INV-3: qk_gain должен быть {φ², φ³}
pub const ALLOWED_QK_GAINS: [f64; 2] = [PHI_SQUARED, PHI_CUBED];  // {2.618, 4.236}
```

**LR band:** [0.002, 0.007] — φ-safe learning rate
**qk_gain:** {φ², φ³} = {2.618, 4.236} — только эти значения

### Валидации (R7)

1. ✅ `lr ∉ [LR_SAFE_MIN, LR_SAFE_MAX]` — INV-1 violation
2. ✅ `qk_gain ∉ {φ², φ³}` — INV-13 violation
3. ✅ `d_model > 0 && num_heads > 0 && divisible` — Shape валидация
4. ✅ All finite tensors — NonFinite проверка

---

## 4. Где найти ЛУЧШИЕ результаты

### SQL Запросы

Файл: `.trinity/best_results_all.sql`

**Основные секции:**
1. **Gate-2 candidates** — все эксперименты с final_bpb < 1.85
2. **Best by format** — лучший результат для каждого формата
3. **Best by architecture** — лучший результат для каждой архитектуры
4. **Best by optimizer** — сравнение AdamW vs Muon
5. **PHI formats matrix** — все PHI форматы с φ-distance и Gate-2 статусом
6. **JEPA-T performance** — отдельный анализ JEPA-T

### Выполнение

```bash
# Получить Railway DSN (через Railway Console или CLI)
export RAILWAY_DATABASE_URL="postgresql://..."

# Выполнить запрос
psql "$RAILWAY_DATABASE_URL" -f .trinity/best_results_all.sql
```

### Что показывает каждый запрос

| Запрос | Что показывает |
|----------|----------------|
| 1. Gate-2 candidates | TOP-20 лучших экспериментов, проходящих Gate-2 |
| 2. Best by format | Для каждого формата: best_bpb, top_3_canons |
| 3. Best by architecture | Для hybrid/attn/jepa: best_bpb, top_3_canons |
| 4. Best by optimizer | Сравнение AdamW vs Muon по всем экспериментам |
| 5. PHI formats | GF4-GF64: φ-distance, Gate-2 статус, best_bpb |
| 6. JEPA-T | Средний d_model, Gate-2/warmup pass rate |

---

## 5. Ключевые метрики

### BPB (Bits Per Byte / Byte Per Token)

| BPB | Смысл | Gate-2 |
|------|--------|---------|
| < 1.0 | Идеально (маловозможно) | ✅ PASS |
| 1.0 - 1.5 | Отлично | ✅ PASS |
| 1.5 - 2.0 | Хорошо | ✅ PASS |
| 2.0 - 2.5 | Нормально | ⚠️ CLOSE |
| > 2.5 | Проблематично | ❌ FAIL |

### Steps vs Quality

| Steps | Тип | BPB ожидания |
|--------|------|----------------|
| 500 | Warmup | ~6.5 (мало информации) |
| 1K | Early | ~5.0 |
| 3K | Rung 1 | ~3.5 |
| 9K | Rung 2 | ~2.5 |
| 27K | Rung 3 | ~2.0 |
| 54K+ | Long | ~1.85 (champion target) |

---

## 6. Рекомендуемые конфигурации

### Short Wave (h=128, steps=500)

**Muon рекомендуется для mantissa-heavy форматов:**
- GF16, GF12, GF20, GF24, GF32, GF64
- FP32, FP64, FP16, TF32

**AdamW рекомендуется для:**
- INT8, UINT8 (low-bit advantage)
- FP8 variants (narrow exponent formats)

### Long Runs (Gate-2 target)

**GF16 + Hybrid:**
```
format: gf16
arch: hybrid
hidden: 828
steps: 54000
optimizer: muon
```

**GF16 + JEPA-T:**
```
format: gf16
arch: jepa
d_model: 512
steps: 54000
optimizer: muon
```

---

## 7. Файлы и скрипты

| Файл | Назание | Назначение |
|--------|----------|-----------|
| `.trinity/short_wave_extended.sql` | SQL для 171 эксперимента |
| `.trinity/best_results_all.sql` | SQL для анализа лучших результатов |
| `.trinity/run_short_wave.sh` | Скрипт для запуска (нужен DSN) |
| `leaderboard-snapshot.md` | Skill для вывода leaderboard |

---

## 8. Связи с EPIC

- EPIC #446 — Format×Algorithm matrix
- EPIC #143 — JEPA-T и Hybrid архитектуры
- Issue #143 — IGLA race tracking

---

🌻 φ² + φ⁻² = 3 · TRINITY · NEVER STOP
