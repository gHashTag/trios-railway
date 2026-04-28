# trios-mcp-gateway — Единый MCP для управления флотом

Один Streamable-HTTP MCP сервер управляет всеми 4 Railway-аккаунтами.
После подключения в Perplexity → полный контроль флота из чата.

## Архитектура

```
Perplexity Chat
    │  Streamable HTTP MCP
    ▼
https://<service>.up.railway.app/mcp
    │
    ├── RAILWAY_TOKEN_ACC0  → kaglerslomaansc@hotmail.com
    ├── RAILWAY_TOKEN_ACC1  → rumbodzalaclhdv0@hotmail.com
    ├── RAILWAY_TOKEN_ACC2  → brabbtjubindt5cug@hotmail.com
    └── RAILWAY_TOKEN_ACC3  → gondiigamzevup@hotmail.com
```

## Шаг 1 — Получить токены для всех 4 аккаунтов

Для каждого аккаунта:
1. Войди в https://railway.com под нужным email
2. Открой https://railway.com/account/tokens
3. Нажми **New Token** → Name: `mcp-gateway` → Scope: **All workspaces**
4. Скопируй токен — он показывается только один раз!

| Аккаунт | Email | Переменная |
|---------|-------|------------|
| Acc0 | kaglerslomaansc@hotmail.com | `RAILWAY_TOKEN_ACC0` |
| Acc1 | rumbodzalaclhdv0@hotmail.com | `RAILWAY_TOKEN_ACC1` |
| Acc2 | brabbtjubindt5cug@hotmail.com | `RAILWAY_TOKEN_ACC2` |
| Acc3 | gondiigamzevup@hotmail.com | `RAILWAY_TOKEN_ACC3` |

## Шаг 2 — Задеплоить gateway в проект da1fb0c7

### Вариант A: Railway Dashboard (рекомендуется)

1. Открой https://railway.com/project/da1fb0c7-199f-42b0-9f08-a84d122feb5b
2. **+ New Service** → **Docker Image**
3. Image: `ghcr.io/ghashtag/trios-mcp-gateway:latest`
   _(или `ghcr.io/ghashtag/trios-railway-mcp:latest` если gateway ещё не собран)_
4. После создания сервиса → **Variables** → добавь:

```
RAILWAY_TOKEN=<acc0_personal_token>          # default (любой рабочий)
RAILWAY_TOKEN_ACC0=<acc0_personal_token>
RAILWAY_TOKEN_ACC1=<acc1_personal_token>
RAILWAY_TOKEN_ACC2=<acc2_personal_token>
RAILWAY_TOKEN_ACC3=<acc3_personal_token>
RAILWAY_TOKEN_AUTH=team
NEON_DATABASE_URL=<твой_neon_pool_url>
PORT=8080
RUST_LOG=info
```

5. **Settings** → **Networking** → **Generate Domain** → получи публичный URL
6. Подожди `ACTIVE` статус (обычно 2-3 мин)

### Вариант B: CLI

```bash
export RAILWAY_TOKEN=<любой_рабочий_токен>
railway link da1fb0c7-199f-42b0-9f08-a84d122feb5b
railway service create --name trios-mcp-gateway
railway variables set \
  RAILWAY_TOKEN=$RAILWAY_TOKEN \
  RAILWAY_TOKEN_ACC0=$ACC0 \
  RAILWAY_TOKEN_ACC1=$ACC1 \
  RAILWAY_TOKEN_ACC2=$ACC2 \
  RAILWAY_TOKEN_ACC3=$ACC3 \
  RAILWAY_TOKEN_AUTH=team \
  NEON_DATABASE_URL=$NEON_DATABASE_URL \
  PORT=8080
railway up --image ghcr.io/ghashtag/trios-mcp-gateway:latest
```

## Шаг 3 — Проверить что сервис живой

```bash
# Подставь свой URL:
SVC_URL=https://trios-mcp-gateway.up.railway.app

# Healthcheck:
curl -s $SVC_URL/healthz
# Ожидаем: 200 OK  {"status":"ok","accounts":["acc0","acc1","acc2","acc3"]}

# Список доступных инструментов:
curl -s -X POST $SVC_URL/mcp \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":1}' | jq .result.tools[].name
# Ожидаем:
# "railway_service_list"
# "railway_service_deploy"
# "railway_service_redeploy"
# "railway_service_delete"
# "railway_experience_append"
# "railway_audit_migrate_sql"
```

## Шаг 4 — Подключить в Perplexity

1. Открой **Perplexity** → **Settings** → **Connectors**
2. Нажми **+ Add connector** (или **+ Custom connector**)
3. Выбери тип: **Streamable HTTP MCP**
4. Заполни:
   - **Name**: `trios-mcp-gateway`
   - **MCP Server URL**: `https://<твой-сервис>.up.railway.app/mcp`
   - **Authentication**: пусто (сервер валидирует через env vars)
5. Нажми **Add** / **Save**
6. Появится новая карточка коннектора

## Шаг 5 — Использовать в чате

В новом чате Perplexity:
1. Нажми **Sources** (или значок плагинов)
2. Включи **trios-mcp-gateway**
3. Готово! Теперь пиши:

```
Покажи все сервисы acc1
→ (вызывает railway_service_list с account=acc1)

Удали сервис c9c5324d
→ (вызывает railway_service_delete confirm=true)

Задеплой IGLA-HYBRID-FP32-seed43 на acc2
→ (вызывает railway_service_deploy)
```

## Логи подключения (что увидишь в Railway logs)

```
[INFO] trios-mcp-gateway starting on 0.0.0.0:8080
[INFO] accounts loaded: acc0=✓ acc1=✓ acc2=✓ acc3=✓
[INFO] NEON audit ledger connected
[INFO] MCP endpoint ready: POST /mcp
[INFO] Health endpoint ready: GET /healthz
[INFO] tools registered: railway_service_list, railway_service_deploy, railway_service_redeploy, railway_service_delete, railway_experience_append, railway_audit_migrate_sql
```

## Удаление мёртвых сервисов (после подключения MCP)

Когда gateway подключён, скажи мне в чате:

> "Удали все 24 мёртвых сервиса из проектов e4fe33bb, 39d833c1 и 265301ce"

Я выполню все `railway_service_delete` через MCP автоматически.

## Canonical IGLA names (для деплоя новых семян)

Формат: `IGLA-<MODEL_TYPE>-<NUM_FORMAT>-seed<N>`

Примеры:
- `IGLA-HYBRID-FP32-seed43`
- `IGLA-TRAIN_V2-FP32-seed42`
- `IGLA-JEPA-T-FP32-seed220`

---

`φ²+φ⁻²=3 · TRINITY · ONE GATEWAY · MCP-ONLY`
