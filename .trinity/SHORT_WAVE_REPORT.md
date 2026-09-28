# Short Wave Extended — IGLA RACE Experiment Matrix

## Сводка

**Масштабируем эксперименты:** 19 форматов × 3 алгоритма × 3 архитектуры = **171 эксперимент**

### Конфигурация

| Параметр | Значение |
|----------|----------|
| Steps | 500 (warmup) |
| Hidden | 128 |
| Priority | 85 (exploratory) |
| Seeds | {200..206} (7 seeds) |
| Accounts | acc0-acc5 (distributed load) |

### Форматы (19)

| Tier | Форматы |
|------|----------|
| A (runnable) | fp32, fp16, bf16, tf32 |
| B (near-runnable) | fp64, fp8_e4m3, fp8_e5m2, gf8, gf12, gf20, gf24, gf32, gf64, int8, int16, int32, uint8 |
| T5 (sub-byte) | gf4 |

### Алгоритмы (3)

| Алгоритм | Примечание |
|----------|----------|
| adamw | Базовый оптимизатор |
| muon | Nesterov momentum + предусловия (рекомендуется для mantissa-heavy форматов) |
| muon-cwd | Coordinate-wise descent (экспериментальный) |

### Архитектуры (3)

| Архитектура | Описание | Мин. rung |
|-------------|----------|----------|
| hybrid | TF-2L (champion) | 1000 |
| attn | 1-layer attention | 1000 |
| jepa | Joint-Embedding Predictive Architecture | 3000 |

## JEPA-T Behaviour Analysis

### Характеристики JEPA-T

**Имя:** Joint-Embedding Predictive Architecture (Ternary)
**Основано на:** LeJEPA/LeWorldModel principles

### Конфигурация по умолчанию

```rust
JepaConfig {
    d_model: 384,        // или 128 для short wave
    mask_ratio: 0.3,     // 30% tokens masked
    min_span: 3,          // минимальный span для маскирования
    max_span: 9,          // максимальный span для маскирования
    num_spans: 2,          // количество спанов
    ema_start: 0.996,    // начальный EMA decay
    ema_end: 1.0,          // финальный EMA decay
    ema_ramp_steps: 30000,  // шаги для EMA ramp
    predictor_lr_mult: 0.1,  // 10× медленнее чем encoder
}
```

### Ключевые отличия от других архитектур

| Параметр | JEPA-T | Hybrid/Attn | Влияние |
|----------|---------|--------------|----------|
| Mask ratio | 0.3 | N/A | 30% токенов маскируются |
| EMA target | Да | Нет | Target encoder обновляется через EMA |
| Rung schedule | [3000, 9000, 27000] | [1000, 3000, 9000, 27000] | Дольше старт |
| Predictor LR | 0.1× encoder | N/A | Predictor учится медленнее |
| Convergence criteria | loss < 1.0 AND variance > 0.01 | loss < 1.85 (Gate-2) | Более строгая |

### Ожидаемое поведение JEPA-T

**Преимущества:**
- ✅ EMA target улучшает стабилисть обучения
- ✅ Masked prediction учит модель предсказывать будущее
- ✅ Философия LeJEPA: предсказывать эмбеддинги вместо токенов

**Недостатки (для short wave):**
- ⚠️ Требует больше шагов для сходимости (min_rung=3000)
- ⚠️ EMA ramp занимает 30000 шагов до полной стабилизации
- ⚠️ Predictor учится 10× медленнее → медленная инициализация

**Hypothesis для IGLA RACE:**
- JEPA-T с mantissa-heavy форматами (gf16, gf20, gf24) показывает
  преимущество при достаточно долгом обучении (9000+ шагов)
- На 500 шагах JEPA-T может проигрывать Hybrid/Attn из-за
  медленного прогрева EMA

## SQL Injection

SQL файл готов: `.trinity/short_wave_extended.sql`

**Для выполнения нужно Railway DSN.**

### Шаги:

1. **Получить Railway DSN:**
   - Через Railway Console → Project Settings → Database
   - Или через Railway CLI: `railway variables`

2. **Выполнить SQL:**
   ```bash
   psql $RAILWAY_DATABASE_URL -f .trinity/short_wave_extended.sql
   ```

3. **Проверить статус очереди:**
   ```sql
   SELECT status, COUNT(*) FROM experiment_queue GROUP BY status;
   ```

## Experiment Queue Distribution

| Account | Форматы (распределение) | Примерный баланс |
|---------|--------------------------|-------------------|
| acc0 | fp32, fp64, fp16, bf16, tf32, fp8_e4m3, fp8_e5m2, gf4 | ~11 experiments |
| acc1 | gf8, gf12, gf16, gf20, gf24, gf32, gf64 | ~8 experiments |
| acc2 | int8, int16, int32, uint8 | ~8 experiments |
| acc3 | [резерв для JEPA-T расширения] | ~6 experiments |
| acc4 | [резерв для других архитектур] | ~6 experiments |

*Балансировка может быть скорректирована после первого прогона.*

## Monitoring

### Ключевые метрики для наблюдения

1. **Queue fill rate:** сколько экспериментов подбирается vs. завершается
2. **JEPA-T convergence rate:** доля JEPA-T экспериментов достигающих loss < 1.0
3. **Format×Algorithm pattern:** подтверждение гипотезы #446 (Muon выигрывает mantissa-heavy)
4. **Architecture comparison:** Hybrid vs. Attn vs. JEPA-T на разных форматах

### Ожидаемая длительность

- **Hybrid/Attn:** ~2-3 минуты на 500 шагов (h=128)
- **JEPA-T:** ~5-7 минут на 500 шагов (EMA ramp overhead)

### Триггеры для действий

| Событие | Действие |
|----------|----------|
| Queue pending > 150 | Запустить ещё воркеры |
| JEPA-T failure rate > 30% | Увеличить ema_ramp_steps или predictor_lr_mult |
| Format×Algorithm pattern нарушен | Проверить implementацию формата в trios-trainer-igla |

## Next Steps

1. ✅ SQL файл создан (`.trinity/short_wave_extended.sql`)
2. ⏳ Получить Railway DSN
3. ⏳ Выполнить SQL injection
4. ⏳ Мониторить воркеров
5. ⏳ Собрать результаты в leaderboard-snapshot формате

---

🌻 φ² + φ⁻² = 3 · TRINITY · NEVER STOP
