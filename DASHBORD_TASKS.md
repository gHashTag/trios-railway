# Claude Code / IGLA Leaderboard Tasks

## 📊 Current Performance Analysis

### BPB Samples Write Rate (from database query)
| Time Range | Records | Rate |
|------------|---------|------|
| Last 1 min | 2 | ~2/min |
| Last 5 min | 7 | ~1.4/min |
| Last 10 min | 14 | ~1.4/min |

**Current Capacity**: ~1-4 records/minute (84 records/hour)

---

## 🎯 High-Priority Tasks for IGLA Leaderboard Improvement

### 1. Performance Optimization (Data Pipeline)

**Task ID**: perf-001
**Title**: Optimize Neon Database Write Performance
**Description**: 
- Current bpb_samples write throughput: ~1.4 records/minute
- Target: Achieve 5-10 records/minute (300-600/hour)
- Potential bottlenecks: 
  * Neon connection pooling (too many individual writes)
  * Batch insert optimization (INSERT ... VALUES(...))
  * Asynchronous writes via NOTIFY
- Implementation priority: HIGH (immediate impact on active workers)

**Acceptance Criteria**: 
- Average bpb_samples write rate increases to >5/min
- P99 write latency decreases to <2 seconds
- Database write error rate < 1%

**Estimated Effort**: 4-8 hours

---

### 2. Quality Enhancement (Model Development)

**Task ID**: qual-001
**Title**: Improve Model Quality and Add Validation
**Description**: 
- Analyze top-performing experiments (bpb < 1.0)
- Identify common hyperparameters in successful runs
- Add statistical significance testing for model improvements
- Implement early stopping based on confidence intervals
- Add model versioning and tracking to prevent regressions

**Acceptance Criteria**: 
- Baseline bpb < 1.1 established with >95% confidence
- New model variant shows statistically significant improvement (p < 0.05)
- Validation set prevents model quality degradation

**Estimated Effort**: 8-16 hours

---

### 3. Scalability Enhancement (Infrastructure)

**Task ID**: scale-001
**Title**: Scalable Worker Pool Architecture
**Description**: 
- Design auto-scaling scarab worker pool based on queue depth
- Implement worker health monitoring and auto-recovery
- Optimize resource allocation per account
- Add worker scheduling fairness (round-robin or priority-based)
- Target: Scale to 500+ workers across all accounts
- Implement graceful deployment without task interruption

**Acceptance Criteria**: 
- Can deploy additional workers via Railway CLI automation
- Worker recovery time < 30 seconds from failure
- Queue processing fairness: tasks picked up within 30 seconds of being pending
- System handles 500+ concurrent experiments without performance degradation

**Estimated Effort**: 2-3 days

---

## 📋 Task Execution Plan

### Phase 1: Performance Optimization (Week 1)
- [ ] Batch insert optimization for bpb_samples
- [ ] Neon connection pooling implementation
- [ ] Asynchronous write queue with NOTIFY
- [ ] Benchmark current write performance
- [ ] Target: 5-10 records/minute

### Phase 2: Quality Enhancement (Week 2-3)
- [ ] Analyze successful experiment patterns
- [ ] Implement statistical significance testing
- [ ] Add confidence interval tracking
- [ ] Early stopping optimization
- [ ] Model versioning system
- [ ] Baseline bpb < 1.1 with validation

### Phase 3: Scalability Enhancement (Week 3-4)
- [ ] Design auto-scaling architecture
- [ ] Implement worker health monitoring
- [ ] Auto-recovery mechanisms
- [ ] Resource allocation optimization
- [ ] Fair scheduling implementation
- [ ] Target: 500+ workers across 7 accounts
- [ ] Graceful deployment system

---

## 🎯 Success Metrics

### Performance (Target vs Current)
| Metric | Current | Target | Gap |
|---------|---------|--------|-----|
| Records/min | ~1.4 | 5-10 | 3.6-8x |
| Throughput | ~84/hour | 300-600/hour | 3.6x |

### Quality
| Metric | Current Status | Target |
|---------|---------------|--------|
| Baseline bpb | < 1.1 (established) | < 1.1 with validation |
| Top performers | BPB < 1.0 | BPR < 0.05 with p<0.05 |

### Scalability
| Metric | Current | Target |
|---------|---------|--------|
| Active workers | ~150 | 500 | 3.3x |
| Recovery time | Manual | < 30 sec | |

---

## 📊 Dashboard Requirements

### Required Visualizations
1. **BPB Write Rate Chart**: Real-time records/minute over time
2. **Queue Depth Chart**: Pending tasks vs active workers
3. **Worker Health Map**: Account status (active/inactive) over time
4. **Model Performance Comparison**: Side-by-side BPB comparison
5. **Learning Curve**: Cumulative experiments vs performance over time

### Data Sources
- `bpb_samples`: Write latency, throughput, error rate
- `strategy_queue`: Queue depth, completion rate, pending time
- `scarabs`: Worker health, account distribution
- Railway services: Deployment status, restarts

### Technical Notes
- Current write rate ~1.4/min = ~84/hour is already decent for training
- Main bottleneck: worker availability (many accounts inactive)
- Railway CLI automation needed for true auto-scaling
- Consider using connection pooling to reduce Neon connection overhead

---

## 🚀 Priority Order

1. **HIGH**: Deploy remaining scarab workers (acc0, acc1, acc6) via Railway Dashboard
   - Immediate impact: 243 workers will become active
   - Estimated effort: 10-15 minutes
   
2. **MEDIUM**: Performance optimization - Batch inserts
   - Can be implemented without worker changes
   - Estimated effort: 4-8 hours

3. **MEDIUM**: Quality enhancement - Validation and tracking
   - Important for preventing regressions
   - Estimated effort: 8-16 hours

4. **LOW**: Scalability architecture design
   - Long-term, can start after immediate tasks
   - Estimated effort: 2-3 days

---

## 📝 Next Actions

1. **Deploy scarab workers** (HIGH PRIORITY)
   - Use Railway Dashboard: https://railway.app
   - For each account (acc0, acc1, acc6):
     * Create new service: scarab-{account}
     * Dockerfile: Dockerfile.scarab
     * Start command: /usr/local/bin/scarab
     * Environment: NEON_DATABASE_URL, SCARAB_ACCOUNT={account}
   
2. **Monitor worker activation**
   - Run: `SELECT railway_acc, COUNT(*), MAX(last_heartbeat) FROM scarabs GROUP BY railway_acc`
   - Expect all accounts to show 0 in "last 5 min" after deployment

3. **Verify bpb_samples write rate**
   - Monitor: New records/minute should increase to >5
   - Target: 300-600/hour

---

## 📊 Estimated Impact

| Task | Timeframe | Records/Hour Impact |
|-------|-----------|---------------------|
| Deploy 243 workers | Immediate | +150-240 throughput/hour |
| Performance opt | Week 1 | +100-200 throughput/hour |
| Quality tracking | Weeks 2-3 | - |
| Scalability design | Weeks 3-4 | - |
| **Total additional throughput** | ~250-340 records/hour |

**Projected Daily Capacity** with deployed workers: ~8,000-10,000 experiments/day (current: ~4,000)

---

**READY FOR EXECUTION** 🎯
