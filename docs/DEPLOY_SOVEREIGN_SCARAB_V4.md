# DEPLOY — Sovereign Scarab v4 (L-SS2)

> **Anchor:** φ² + φ⁻² = 3  
> **Статус:** Операционная инструкция  
> **Epic:** [gHashTag/trios#940](https://github.com/gHashTag/trios/issues/940)  
> **Issue:** [gHashTag/trios-railway#210](https://github.com/gHashTag/trios-railway/issues/210)  
> **Образ:** `ghcr.io/ghashtag/sovereign-scarab:v4`  
> **Публикация образа:** `.github/workflows/sovereign-scarab.yml` (L-SS1, PR #159 — не трогать)  
> **Post-deploy управление:** queen-hive-mcp (L-SS3, PR #161)

---

## 1. Цель и предусловия

### 1.1 Цель

Задеплоить **27 идентичных сервисов** `sovereign-scarab:v4` на Railway.  
Каждый сервис — автономный heartbeat-агент, который:

- подключается к Trinity SSOT через `DATABASE_URL`,
- записывает пульс в `ssot.scarab_heartbeat` каждые `HEARTBEAT_INTERVAL_S` секунд,
- опрашивает очередь задач с интервалом `POLL_INTERVAL_MS` мс,
- пишет structured-логи (`RUST_LOG=info,sovereign_scarab=debug`).

### 1.2 Предусловия

| № | Предусловие | Ответственный |
|---|------------|---------------|
| P-01 | Образ `ghcr.io/ghashtag/sovereign-scarab:v4` опубликован (workflow L-SS1 прошёл) | CI/CD |
| P-02 | Railway-аккаунты ACC1 / ACC2 / ACC3 созданы и верифицированы | Оператор |
| P-03 | Каждый аккаунт имеет активный Project с достаточным лимитом сервисов (≥ 9) | Оператор |
| P-04 | `DATABASE_URL` для каждого сервиса указывает на **Trinity SSOT** (единый источник истины) | Оператор |
| P-05 | Таблица `ssot.scarab_heartbeat` существует и доступна для INSERT | DBA |
| P-06 | Таблица `ssot.scarab_audit_log` существует (для G-SS-05) | DBA |
| P-07 | GHCR-пакет публичный **или** Railway имеет pull-секрет для `ghcr.io/ghashtag` | DevOps |

> ⚠️ **ВАЖНО:** `DATABASE_URL` каждого из 27 сервисов **должен** указывать на Trinity SSOT.  
> Использование отдельных БД нарушит консистентность heartbeat-счётчика.

> ℹ️ **Mass-deploy workflows устарели:** PR #214 (замерджен) задепрекейтил автоматические  
> mass-deploy workflow. Деплой 27 сервисов выполняется **вручную** по настоящей инструкции  
> или через queen-hive-mcp (L-SS3, PR #161).

---

## 2. Переменные окружения (per service)

Каждый из 27 сервисов должен получить следующий набор env-переменных:

| Переменная | Значение | Описание |
|-----------|---------|---------|
| `DATABASE_URL` | `postgresql://<user>:<pass>@<host>:<port>/<db>` | Строка подключения к Trinity SSOT — **уникальна для каждого аккаунта** |
| `RAILWAY_SERVICE_ID` | `<uuid>` | Автоматически задаётся Railway; при ручном деплое — UUID сервиса |
| `TRAIN_DATA` | `fineweb` | Идентификатор датасета |
| `POLL_INTERVAL_MS` | `2000` | Интервал опроса очереди, миллисекунды |
| `HEARTBEAT_INTERVAL_S` | `30` | Интервал heartbeat в БД, секунды |
| `RUST_LOG` | `info,sovereign_scarab=debug` | Уровень логирования |

> `RAILWAY_SERVICE_ID` Railway проставляет автоматически в runtime. Явно задавать его  
> нужно только при локальном тестировании или нестандартных деплоях.

---

## 3. Шаги деплоя через Railway UI (один сервис)

> Повторить для каждого из 27 сервисов.

```
Railway Dashboard
  └─ [Выбрать Project]
       └─ "+ New Service"
            └─ "Docker Image"
                 └─ Ввести: ghcr.io/ghashtag/sovereign-scarab:v4
                      └─ "Deploy"
                           └─ [Открыть сервис] → "Variables"
                                └─ "+ Add Variable" (повторить для каждой переменной):
                                     DATABASE_URL        = <Trinity SSOT URL>
                                     TRAIN_DATA          = fineweb
                                     POLL_INTERVAL_MS    = 2000
                                     HEARTBEAT_INTERVAL_S = 30
                                     RUST_LOG            = info,sovereign_scarab=debug
                                └─ "Redeploy" (применить переменные)
                                     └─ [Logs] → убедиться, что нет FATAL / panics
```

### 3.1 Именование сервисов

Рекомендуемый шаблон: `scarab-{account}-{n:02}`, например:

- `scarab-acc1-01` … `scarab-acc1-09`
- `scarab-acc2-01` … `scarab-acc2-09`
- `scarab-acc3-01` … `scarab-acc3-09`

### 3.2 Healthcheck

Healthcheck Railway (`HEALTHCHECK`) **не используется** — мониторинг состояния ведётся  
через запись heartbeat-строк в `ssot.scarab_heartbeat`. Smoke-test описан в разделе 5.

---

## 4. Шаги деплоя через Railway CLI

> Railway CLI (`railway`) должен быть установлен: `npm i -g @railway/cli` или через homebrew.

```bash
# 1. Авторизация
railway login

# 2. Привязка к проекту (выполнить один раз per проект)
railway link <PROJECT_ID>

# 3. Создать новый сервис из Docker-образа
railway add --service scarab-acc1-01

# 4. Задать переменные окружения
railway variables set \
  DATABASE_URL="postgresql://..." \
  TRAIN_DATA="fineweb" \
  POLL_INTERVAL_MS="2000" \
  HEARTBEAT_INTERVAL_S="30" \
  RUST_LOG="info,sovereign_scarab=debug" \
  --service scarab-acc1-01

# 5. Задеплоить конкретный образ
railway up --image ghcr.io/ghashtag/sovereign-scarab:v4 --service scarab-acc1-01

# 6. Проверить логи
railway logs --service scarab-acc1-01
```

> Повторить шаги 3–6 для каждого из 27 сервисов.  
> Для массового деплоя можно обернуть в `for`-цикл, но убедитесь, что Railway API не  
> rate-limit'ит запросы (рекомендуется пауза `sleep 2` между итерациями).

---

## 5. Распределение 27 сервисов

### Вариант A — Три аккаунта (рекомендуется)

| Аккаунт | Проект | Сервисы | Количество |
|---------|--------|---------|-----------|
| ACC1 | trios-scarab-1 | scarab-acc1-01 … scarab-acc1-09 | 9 |
| ACC2 | trios-scarab-2 | scarab-acc2-01 … scarab-acc2-09 | 9 |
| ACC3 | trios-scarab-3 | scarab-acc3-01 … scarab-acc3-09 | 9 |

**Преимущества:** изоляция по аккаунтам снижает риск исчерпания квоты Railway; упрощает  
ротацию `DATABASE_URL` при смене пароля SSOT.

### Вариант B — Один аккаунт

Все 27 сервисов в одном проекте одного аккаунта.  
**Применимо:** если Railway-план позволяет ≥ 27 активных сервисов.

---

## 6. Smoke-test после деплоя

После деплоя всех 27 сервисов выполнить запрос к Trinity SSOT:

```sql
-- Ожидаемый результат: 27
SELECT COUNT(DISTINCT canon_name)
FROM ssot.scarab_heartbeat
WHERE ts > now() - INTERVAL '60s';
```

> Если счётчик < 27 — проверить логи отсутствующих сервисов через Railway UI/CLI.  
> Типичные причины: неверный `DATABASE_URL`, образ не скачался (см. P-07).

Дополнительная проверка audit-лога (G-SS-05):

```sql
-- Ожидаем: нет строк с exit_code != 1 за последние 5 минут
SELECT canon_name, exit_code, created_at
FROM ssot.scarab_audit_log
WHERE created_at > now() - INTERVAL '5 minutes'
  AND exit_code != 1
ORDER BY created_at DESC;
```

---

## 7. Acceptance Criteria

| ID | Критерий | Проверка |
|----|---------|---------|
| **G-SS-04** | Heartbeat coverage: все 27 сервисов пишут heartbeat не реже 1 раза в 60 секунд | `COUNT(DISTINCT canon_name) = 27` в `ssot.scarab_heartbeat` за последние 60 с |
| **G-SS-05** | Audit watchdog: ни один сервис не завершил watchdog-цикл с `exit_code != 1` | `SELECT COUNT(*) = 0` в `ssot.scarab_audit_log WHERE exit_code != 1` за последние 5 минут |

Деплой считается **успешным** при выполнении обоих критериев одновременно.

---

## 8. Rollback

Для паузы отдельного сервиса без удаления используйте queen-hive-mcp (L-SS3, PR #161):

```sql
-- Через Queen-Hive MCP
SELECT ssot.pause_scarab('<canon_name>');

-- Пример: остановить scarab-acc2-05
SELECT ssot.pause_scarab('scarab-acc2-05');
```

Для полной остановки всего флота:

```sql
-- Пауза всех 27 сервисов
SELECT ssot.pause_scarab(canon_name)
FROM ssot.scarab_heartbeat
GROUP BY canon_name;
```

Для возобновления — через Railway UI/CLI: **Redeploy** соответствующего сервиса.

> ℹ️ queen-hive-mcp (L-SS3, PR #161) предоставляет полный набор MCP-инструментов  
> для управления флотом после деплоя: pause, resume, drain, status.

---

## 9. Ссылки

| Ресурс | URL |
|--------|-----|
| Epic L-SS | https://github.com/gHashTag/trios/issues/940 |
| Issue L-SS2 | https://github.com/gHashTag/trios-railway/issues/210 |
| L-SS1 (образ) | https://github.com/gHashTag/trios-railway/pull/159 |
| L-SS3 (queen-hive-mcp) | https://github.com/gHashTag/trios-railway/pull/161 |
| PR #214 (deprecated mass-deploy) | https://github.com/gHashTag/trios-railway/pull/214 |
| Railway Docs | https://docs.railway.com |
| GHCR образ | https://ghcr.io/ghashtag/sovereign-scarab |

---

*Документ подготовлен Queen Hive Bot · φ² + φ⁻² = 3*
