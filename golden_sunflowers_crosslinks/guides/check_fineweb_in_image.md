# Guide: Verify fineweb.bin exists in Railway Docker image

## Why this matters

Before deploying the scarab.rs Option B patch (which passes corpus paths from config_json), we must verify that `/work/data/fineweb_*.bin` files actually exist in the Railway Docker image. If they don't exist, the trainer will fail at runtime even with correct paths.

## Method 1: Check Dockerfile (build-time)

Look at `Dockerfile` or `docker-trainer.yml` to see what gets COPYed during image build:

```bash
# Find data COPY or ADD directives
grep -n "COPY.*data\|ADD.*data" Dockerfile*

# Check for fineweb
grep -i "fineweb" Dockerfile

# Look for data directory structure
docker build --progress=plain -o - . 2>&1 | tee build.log | grep -i fineweb
```

### Expected patterns

**Pattern A (fineweb present at build time):**
```dockerfile
COPY crates/trios-trainer-igla/data/fineweb_train.bin /work/data/
COPY crates/trios-trainer-igla/data/fineweb_val.bin /work/data/
```

**Pattern B (data downloaded at container start):**
```dockerfile
# No COPY, download script runs on container start
RUN python3 /scripts/download_fineweb.py --to /work/data/
```

**Pattern C (tiny_shakespeare only):**
```dockerfile
COPY crates/trios-trainer-igla/data/tiny_shakespeare*.txt /work/data/
# OR no fineweb mentioned
```

## Method 2: Check Cargo.toml (embedded assets)

```bash
# Look for embedded data (rare but possible)
grep -i "embed\|include_bytes\|include_str" crates/trios-trainer-igla/Cargo.toml
```

If fineweb data is embedded in binary, image will have it regardless of Dockerfile.

## Method 3: Inspect built image directly (most reliable)

```bash
# Build or pull current image
IMAGE_ID=$(gh api repos/gHashTag/trios-trainer-igla/packages/trios-trainer | \
  jq -r '.[0].container_id // empty' || gh cr images get gcr.io/trinity-s3ai/trios-trainer:latest --format '{{.ID}}')

# Create temporary container to inspect filesystem
docker create --name temp-inspect $IMAGE_ID

# Copy data directory out
docker cp temp-inspect:/work/data /tmp/railway_data

# List what's inside
ls -lh /tmp/railway_data/
du -sh /tmp/railway_data/

# Cleanup
docker rm temp-inspect

# Check for fineweb files
find /tmp/railway_data -name "*fineweb*"
```

### Expected output if fineweb present:

```
/tmp/railway_data/
total 32M
-rw-r--r-- 1 playra staff 8.0M May  1 00:14 fineweb_train.bin
-rw-r--r-- 1 playra staff 24M May  1 00:14 fineweb_val.bin
```

### Expected output if ONLY tiny_shakespeare:

```
/tmp/railway_data/
total 512K
-rw-r--r-- 1 playra staff 64K May  1 00:14 tiny_shakespeare.txt
-rw-r--r-- 1 playra staff 448K May  1 00:14 tiny_shakespeare_val.txt
```

## Method 4: Check GitHub Container Registry

```bash
# Get image manifest
gh api repos/gHashTag/trios-trainer-igla/packages/trios-trainer | \
  jq '.[0].container_manifest'

# Look at layer history
gh api repos/gHashTag/trios-trainer-igla/packages/trios-trainer | \
  jq '.[0].container_layers[] | select(.created != null)'
```

Check for fineweb-related layer history.

## Method 5: Check Railway deployment environment

```bash
# Check environment variables set in Railway
railway variables --service 12448b20-6b6c-4004-915e-e9bd7a3a9d53

# Or check service configuration
railway service get 12448b20-6b6c-4004-915e-e9bd7a3a9d53

# Look for TRIOS_CORPUS or data-related env vars
```

## Decision matrix after inspection

| Finding | Action |
|----------|--------|
| fineweb_train.bin exists (8MB+) | ✅ Proceed with Option B patch |
| fineweb_train.bin exists but tiny (<1MB) | ⚠️ Partial/corrupted data - regenerate |
| Only tiny_shakespeare exists | 🛑 STOP - need data download script |
| Neither corpus exists | 🛑 STOP - need data mounting solution |

## If fineweb NOT in image: Three options

### Option 1: Add data download step to Dockerfile

```dockerfile
# Add after base image
RUN --mount=type=cache,target=/root/.cache \
    python3 -c "from huggingface_hub import snapshot_download; \
    snapshot_download('allenai/fineweb-2', \
    repo_type='dataset', \
    repo_id='shard', \
    allow_patterns=['fineweb_train.bin', 'fineweb_val.bin'], \
    cache_dir='/root/.cache'); \
    from huggingface_hub import snapshot_download; \
    snapshot_download('allenai/fineweb-2', \
    repo_type='dataset', \
    repo_id='shard', \
    allow_patterns=['fineweb_tokenizer.model'], \
    cache_dir='/root/.cache'); \
    mv /root/.cache/downloads/*/*.bin /work/data/; \
    mv /root/.cache/downloads/*/*.model /work/data/"
```

### Option 2: Mount from Railway volume (requires Railway Business plan)

```yaml
# In railway.toml or service settings
[volumes]
data = "/work/data"
```

### Option 3: Accept tiny_shakespeare as IGLA corpus, separate FineWeb pipeline

Document that IGLA Race is on tiny_shakespeare. Create separate scarab-image-fine for parameter-golf track.

---

*Prepared 2026-05-02. Execute on your machine or provide output of `docker create` inspection.*