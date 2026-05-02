# Railway Scarab Worker Deployment Instructions

## Инструкция по развертыванию scarab воркеров

### Неактивные аккаунты:
- **acc0**: 160 воркеров (последний heartbeat: 2026-04-30)
- **acc1**: 110 воркеров (последний heartbeat: 2026-04-30)  
- **acc6**: 3 воркера (последний heartbeat: 2026-04-30)

---

## Шаг 1: Вход в Railway (один раз)

```bash
railway login
```

---

## Шаг 2: Развернуть scarab-acc0 (trios-trainer проект)

```bash
# Перейдите в директорию проекта
cd /Users/playra/trios-trainer-igla

# Убедитесь что Railway CLI работает
railway status

# Создайте scarab-acc0 сервис
cat > /tmp/railway-acc0.json << 'EOF'
{
  "$schema": "https://railway.app/railway.schema.json",
  "build": {
    "builder": "DOCKERFILE",
    "dockerfilePath": "Dockerfile.scarab"
  },
  "deploy": {
    "startCommand": "/usr/local/bin/scarab",
    "restartPolicyType": "NEVER"
  }
}
EOF

# Свяжите проект
railway link

# Разверните сервис
railway up -f /tmp/railway-acc0.json

# Установите переменные окружения
railway variables set NEON_DATABASE_URL=postgres://npg_NHBC5hdbM0Kx:ep-curly-math-ao51pquy-pooler.c-2.ap-southeast-1.aws.neon.tech/neondb SCARAB_ACCOUNT=acc0
```

---

## Шаг 3: Развернуть scarab-acc1 (IGLA проект)

```bash
# Переключитесь на IGLA проект
railway link

# Создайте scarab-acc1 сервис
cat > /tmp/railway-acc1.json << 'EOF'
{
  "$schema": "https://railway.app/railway.schema.json",
  "build": {
    "builder": "DOCKERFILE",
    "dockerfilePath": "Dockerfile.scarab"
  },
  "deploy": {
    "startCommand": "/usr/local/bin/scarab",
    "restartPolicyType": "NEVER"
  }
}
EOF

# Разверните сервис
railway up -f /tmp/railway-acc1.json

# Установите переменные окружения
railway variables set NEON_DATABASE_URL=postgres://npg_NHBC5hdbM0Kx:ep-curly-math-ao51pquy-pooler.c-2.ap-southeast-1.aws.neon.tech/neondb SCARAB_ACCOUNT=acc1
```

---

## Шаг 4: Развернуть scarab-acc6 (thriving-eagerness проект)

```bash
# Переключитесь на thriving-eagerness проект
railway link

# Создайте scarab-acc6 сервис
cat > /tmp/railway-acc6.json << 'EOF'
{
  "$schema": "https://railway.app/railway.schema.json",
  "build": {
    "builder": "DOCKERFILE",
    "dockerfilePath": "Dockerfile.scarab"
  },
  "deploy": {
    "startCommand": "/usr/local/bin/scarab",
    "restartPolicyType": "NEVER"
  }
}
EOF

# Разверните сервис
railway up -f /tmp/railway-acc6.json

# Установите переменные окружения
railway variables set NEON_DATABASE_URL=postgres://npg_NHBC5hdbM0Kx:ep-curly-math-ao51pquy-pooler.c-2.ap-southeast-1.aws.neon.tech/neondb SCARAB_ACCOUNT=acc6
```

---

## Шаг 5: Проверка статуса

```bash
# Проверьте что все сервисы запущены
railway status

# Проверьте что воркеры активны в базе данных
PGPASSWORD="npg_NHBC5hdbM0Kx" psql -h "ep-curly-math-ao51pquy-pooler.c-2.ap-southeast-1.aws.neon.tech" -U "neondb_owner" -d "neondb" -c "
SELECT railway_acc, COUNT(*) as count,
       MAX(last_heartbeat) as latest_heartbeat
FROM scarabs
GROUP BY railway_acc
ORDER BY railway_acc;
"
```

---

## Railway Dashboard

Альтернативно можно использовать Railway Dashboard:

### acc0 (trios-trainer)
https://railway.app/project/265301ce-0bf2-4187-a36f-348b0eb9942f

### acc1 (IGLA)
https://railway.app/project/e4fe33bb-3b09-4842-9782-7d2dea1abc9b

### acc6 (thriving-eagerness)
https://railway.app/project/39d833c1-4cb6-4af9-b61b-c204b6733a98

---

## Удаление старых записей scarabs (опционально)

```bash
PGPASSWORD="npg_NHBC5hdbM0Kx" psql -h "ep-curly-math-ao51pquy-pooler.c-2.ap-southeast-1.aws.neon.tech" -U "neondb_owner" -d "neondb" << 'EOF'
-- Удалить старые записи scarabs для acc0, acc1, acc6
DELETE FROM scarabs
WHERE railway_acc IN ('acc0', 'acc1', 'acc6');
EOF
```
