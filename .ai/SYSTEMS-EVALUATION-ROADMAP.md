# .ai/SYSTEMS-EVALUATION-ROADMAP.md
**Canonical Project Improvement Guidance**
**Established:** 2026-02-28
**Status:** APPROVED & ACTIVE
**Responsibility:** Architecture team + CI/build engineering
**Review Cycle:** Quarterly

---

## PURPOSE

This document serves as the **single source of truth** for the project's systems-level improvement roadmap. It provides:

1. Comprehensive baseline metrics (measured Feb 28, 2026)
2. Prioritized remediation actions (P0 → P4 tiers)
3. Phase-gated implementation sequencing (8-12 weeks)
4. Success criteria and go/no-go gates
5. Quarterly evaluation ritual for course correction

**For Agents:** This document defines decision criteria, artifact locations, and success metrics for any improvement work.
**For Humans:** This document provides the architecture-level context for feature prioritization and refactoring decisions.

---

## CRITICAL BASELINES (As of 2026-02-28)

### Architecture Health
- ✓ **Circular Dependencies:** 0 (verified via dependency tree)
- ✓ **App Isolation:** 6/6 apps with zero inter-dependencies
- ✓ **Layering:** Perfect contract → runtime → apps separation
- ⚠️ **desktop_runtime Coupling:** Depends on all 6 apps directly (blocking point)

### Testing Maturity
- ✗ **Unit Tests:** 108 (solid foundation)
- ✗ **Integration Tests:** 6 (CRITICAL GAP: need 100+)
- ✓ **E2E Tests:** 18 scenarios, 20 slices (mature)
- **Gap Impact:** Reducer logic untested; app lifecycle untested; contract validation untested

### Build & Performance
- ✓ **sccache:** Integrated, 4.5% utilized (healthy headroom)
- ✗ **Compile-time SLOs:** NOT DEFINED (regressions undetected)
- ✗ **LTO in Release:** NOT ENABLED (5-10% binary improvement on table)
- **Baseline Times:** check 48.9s, test 45.9s, verify-fast 32.9s

### Automation Maintainability
- ✓ **Command Families:** 13 modular, clear responsibilities
- ✗ **e2e.rs Size:** 2,338 lines (35% of all command code, monolithic)
- ✗ **Code Duplication:** 45% in error handling, option parsing, subprocess lifecycle
- **Scaling Risk:** Each feature adds 100-200 LOC; unmaintainable in 6-12 months

### Documentation Governance
- ✓ **Frontmatter Compliance:** 100% (34/34 docs)
- ✓ **Review Freshness:** 85.3% within 30 days (zero stale docs)
- ✓ **Crate Coverage:** 93.8% (15/16 crates documented)
- **Minor Gaps:** Wiki sync formalization, coverage metrics publication

### Developer Experience
- ✓ **Unified Interface:** xtask with 13 commands, Cargo aliases
- ⚠️ **Artifact Cleanup:** Policy undefined (unbounded growth risk)
- ⚠️ **Onboarding:** > 30 min (context scattered across 5+ files)
- ⚠️ **Error Context:** Generic error propagation (hard to diagnose)

---

## PRIORITIZED REMEDIATION ROADMAP

### Phase 1: Foundational Stability (Weeks 1-2, 35-42 hours)
**Objective:** Establish CI gates; fix critical gaps; unblock scaling work

#### P0 Actions (BLOCKING — Must Complete Before Phase 2)
- [ ] **Circular Dependency Detection** (2h) - Add `cargo deny` CI gate
- [ ] **Reducer Unit Tests** (12h) - Target > 60% coverage (50+ tests)
- [ ] **Integration Test Scaffolding** (8h) - Create `AppTestHarness` abstraction
- [ ] **Profile Optimization** (1h) - Add [profile.release] LTO + strip
- [ ] **Compile-time SLOs** (2h) - Define & enforce thresholds (check < 55s)
- [ ] **e2e.rs Phase 1** (6h) - Extract types & config modules
- [ ] **Doc Freshness Gate** (2h) - Enforce 90-day CI block
- [ ] **Artifact Cleanup** (2h) - Implement `cargo xtask cleanup`

**Phase 1 Go/No-Go Gate (End of Week 2):**
- All P0 actions complete ✓
- CI gates blocking regressions ✓
- No new failures introduced ✓

#### P1 Actions (HIGH PRIORITY — Start After P0)
- [ ] **File I/O Helpers** (2h) - Extract to `runtime/helpers.rs`
- [ ] **sccache Metrics** (3h) - Log hit/miss rates to artifacts
- [ ] **E2E Determinism Baseline** (3h) - 10-run test, < 2% failure rate
- [ ] **Contract Validation Tests** (4h) - 112-test matrix for all types
- [ ] **Feature Flag Matrix** (2h) - Document all valid combinations
- [ ] **Contract Evolution Policy** (3h) - Create ADR with v2→v3 criteria
- [ ] **Wiki Sync Formalization** (1h) - Add PR template section

---

### Phase 2: Structural Clarity (Weeks 3-4, 63 hours)
**Objective:** Modularize xtask; build test infrastructure; document boundaries

#### P1 Continued Work
- [ ] **e2e.rs Modularization** (20h)
  - Phase 2: harness module (8h)
  - Phase 3: manifest module (3h)
  - Phase 4: promotion module (3h)
  - Phase 5: consolidation (6h)
- [ ] **App Integration Test Suite** (18h) - 3-5 tests per app × 6 apps
- [ ] **Browser Fallback Tests** (6h) - Desktop/browser capability parity
- [ ] **Error Context Enhancement** (4h) - Rich diagnostics with operation chain
- [ ] **Subprocess Lifecycle RAII** (8h) - `BackgroundHttpServerGuard` implementation
- [ ] **Fan-in/Fan-out Metrics** (2h) - Measure and document crate coupling
- [ ] **Transitive Dependency Audit** (2h) - Report: top 10 largest deps
- [ ] **MSRV Policy** (1h) - Define Rust 1.70 baseline

**Phase 2 Go/No-Go Gate (End of Week 4):**
- e2e.rs > 50% modularized ✓
- Integration test harness working ✓
- Test suite passing (no regressions) ✓

---

### Phase 3: Performance Instrumentation (Weeks 5-6, 23 hours)
**Objective:** Enable data-driven decisions; optimize compile paths

#### P0-P1 Performance Actions
- [ ] **Regression Detection CI** (4h) - Block builds > 15% slower
- [ ] **Compiler Perf Trends** (3h) - Monthly chart generation
- [ ] **Runtime Benchmarks** (6h) - 5+ hot-path Criterion suites
- [ ] **E2E Compile Optimization** (3h) - Design feature gates (ci-headless-minimal)
- [ ] **Token Registry** (3h) - Single source of truth for design tokens
- [ ] **Responsive Viewport Coverage** (4h) - 100% breakpoint testing

**Phase 3 Go/No-Go Gate (End of Week 6):**
- Regression detection working ✓
- Performance SLOs > 90% compliant ✓
- Dashboard metrics visible ✓

---

### Phase 4: Full Coverage & Optimization (Weeks 7-8, 32 hours)
**Objective:** Comprehensive coverage; optimized feedback loop

#### P2-P4 Polish Actions
- [ ] **Option Parsing Macro** (3h) - 8 commands using unified macro
- [ ] **Skin A/B Testing** (3h) - All skins × all viewports validated
- [ ] **WCAG Validation** (5h) - Zero accessibility violations detected
- [ ] **Automated Baseline Promotion** (4h) - Batch promotion workflow
- [ ] **Performance Dashboard** (4h) - Monthly trends published
- [ ] **Onboarding Optimization** (4h) - Clone → first test < 30 min
- [ ] **Wiki Integration** (3h) - Auto-suggest SHA in PR templates
- [ ] **Browser App Tests** (6h) - All apps tested in browser + desktop modes

**Phase 4 Go/No-Go Gate (End of Week 8):**
- Coverage > 60% (unit + integration) ✓
- Onboarding < 30 min documented ✓
- All metrics historical baseline established ✓

---

## SUCCESS METRICS & CONTINUOUS TRACKING

### Architecture Domain
| Metric | Baseline | Target | Frequency | Owner |
|--------|----------|--------|-----------|-------|
| Circular dependencies | 0 | 0 | Every commit | Arch team |
| Feature combos tested | 90% | 100% | Monthly | QA team |
| Contract evolution policy | No | ADR-approved | Post-Phase 1 | Arch team |
| App registry decoupling | Inline | Plugin-ready | Post-Phase 2 | Arch team |

### Testing Domain
| Metric | Baseline | Target | Frequency | Owner |
|--------|----------|--------|-----------|-------|
| Unit test count | 108 | 200+ | Per sprint | QA team |
| Integration tests | 6 | 100+ | Per sprint | QA team |
| Reducer coverage | 49% | 60%+ | Every commit | QA team |
| E2E flakiness | Unknown | < 2% | Weekly | QA team |
| Contract validation tests | 3 | 112+ | Per sprint | QA team |

### Build & Performance Domain
| Metric | Baseline | Target | Frequency | Owner |
|--------|----------|--------|-----------|-------|
| Compile SLO compliance | N/A | 95%+ | Every PR | Build team |
| sccache hit rate | Unknown | 75-85% | Weekly | Build team |
| Release binary size | Unknown | -10% (LTO) | Monthly | Build team |
| Regression detection latency | Manual | < 1 hour | Every PR | Build team |

### Automation Domain
| Metric | Baseline | Target | Frequency | Owner |
|--------|----------|--------|-----------|-------|
| e2e.rs size | 2,338 LOC | 150 LOC | Post-Phase 2 | Eng team |
| Code duplication | 45% | < 10% | Monthly | Eng team |
| Max command module | 2,337 LOC | 500 LOC | Monthly | Eng team |
| Subprocess orphan processes | Unknown | 0 | Weekly | Eng team |
| Workflow recording coverage | 70% | 100% | Per sprint | Eng team |

### Documentation Domain
| Metric | Baseline | Target | Frequency | Owner |
|--------|----------|--------|-----------|-------|
| Frontmatter compliance | 100% | 100% | Every commit | Docs team |
| Doc staleness (> 90d) | 0 | 0 | Weekly | Docs team |
| Crate coverage | 93.8% | 100% | Monthly | Docs team |
| Wiki sync compliance | N/A | 100% of PRs | Per PR | Docs team |

### Developer Experience Domain
| Metric | Baseline | Target | Frequency | Owner |
|--------|----------|--------|-----------|-------|
| Onboarding time | > 30 min | < 30 min | Quarterly | Eng team |
| Error message clarity | 3/5 avg | 4.5/5 avg | Monthly | Eng team |
| Build artifact size | 960 MB | < 500 MB | Weekly | Build team |
| Artifact cleanup automation | No | Yes | Phase 1 | Eng team |

---

## DECISION POINTS & GO/NO-GO GATES

### Week 2 Gate: Foundation Established
**Required for Phase 2 Approval:**
- [ ] `cargo deny check` passing in CI
- [ ] Compile-time SLO framework implemented (not all SLOs met yet)
- [ ] Reducer test suite at 50%+ coverage
- [ ] Integration test harness passing first tests
- [ ] Doc freshness CI gate blocking stale docs
- [ ] No new regressions introduced

**If Gate Fails:** Hold Phase 2 start; resolve blockers in Week 3

### Week 4 Gate: Modularization Path Clear
**Required for Phase 3 Approval:**
- [ ] e2e.rs > 50% extracted into submodules
- [ ] 20+ integration tests passing
- [ ] Error context propagation working in harness module
- [ ] RAII subprocess cleanup tested and working
- [ ] All Phase 1 P0 actions complete
- [ ] CI test suite passes consistently

**If Gate Fails:** Continue Phase 2 work; reassess in Week 5

### Week 6 Gate: Performance Instrumentation Working
**Required for Phase 4 Approval:**
- [ ] Regression detection blocks non-compliant PRs
- [ ] sccache hit rate tracked and visible
- [ ] Performance SLOs > 90% compliant
- [ ] Monthly trend reports generated
- [ ] Baseline metrics established for all domains

**If Gate Fails:** Focus on completing performance instrumentation; delay Phase 4 polish

### Week 8 Gate: Full Roadmap Complete
**Final Success Criteria:**
- [ ] Test coverage > 60% (unit + integration)
- [ ] Onboarding documented < 30 min
- [ ] All metrics baseline established
- [ ] Quarterly evaluation ritual scheduled
- [ ] Zero regressions in main branch
- [ ] Team trained on new processes

---

## QUARTERLY EVALUATION RITUAL

**Established:** End of Phase 4 (Week 8)
**Owner:** Architecture team + QA lead

### Monthly Automation (First Friday of month)
```bash
cargo cache stats  # Compiler cache health
cargo perf dev-loop-baseline  # Dev iteration performance
cargo test --workspace  # Test suite status
cargo xtask docs audit-report  # Documentation status
```
**Output:** Metrics published to performance dashboard; alerts if thresholds breached

### Quarterly Manual Review (End of each quarter)
**Participants:** Architecture team, QA lead, build engineering, documentation owner

**Agenda:**
1. Review all success metrics against targets (15 min)
2. Analyze trends (compile time, test count, coverage) (15 min)
3. Identify new blockers or scaling pressures (15 min)
4. Recommend P0/P1 actions for next quarter (15 min)
5. Publish evaluation report (internal wiki) (10 min)

**Deliverable:** Updated roadmap with next 12-week priorities

### Annual Strategic Review (End of year)
**Participants:** Engineering leadership, product, architecture

**Topics:**
- Feature flag complexity assessment (consider v3 contract?)
- Third-party app extensibility readiness (plugin system ready?)
- Automation framework scalability (can xtask handle 50+ commands?)
- Long-term performance targets (post-Phase 4 vision)

---

## AGENT DECISION GUIDANCE

When agents encounter decisions related to this project, use these criteria:

### Testing Decisions
- **If adding a feature:** Require corresponding integration test (Phase 2+ requirement)
- **If modifying reducer:** Require new reducer unit tests (Phase 1+ requirement)
- **If E2E scenario fails:** Investigate root cause; only accept retries as temporary (Phase 3+ requirement)

### Automation Decisions
- **If adding to e2e.rs:** Extract to submodule (Phase 2+ requirement)
- **If creating new command > 200 LOC:** Split into multiple modules (Phase 2+ requirement)
- **If subprocess management needed:** Use RAII pattern (Phase 2+ requirement)

### Documentation Decisions
- **If modifying functionality:** Update corresponding doc + Wiki (AGENTS.md requirement)
- **If doc review > 90 days old:** Flag for immediate update (Phase 1+ requirement)
- **If adding new crate:** Create reference guide + update coverage metrics (Phase 4+ requirement)

### Performance Decisions
- **If build time increases > 5%:** Investigate regression (Phase 3+ requirement)
- **If adding dependency:** Evaluate transitive cost (Phase 2+ requirement)
- **If E2E compile > 5 min:** Profile and optimize (Phase 3+ requirement)

---

## ARTIFACT LOCATIONS

All evaluation artifacts and ongoing metrics are stored in:
```
/Users/justinshort/os/.artifacts/evaluation/
├── PHASE-1-BASELINE-REPORT.md              (baseline metrics)
├── COMPREHENSIVE-EVALUATION-FINAL-REPORT.md (full roadmap)
├── perf/reports/                           (monthly performance trends)
├── automation/runs/                        (xtask workflow records)
└── monthly-metrics.json                    (dashboard data)
```

**Integration Points:**
- `.github/workflows/` - CI gates reference this roadmap
- `docs/QUARTERLY-REVIEW.md` - Evaluation ritual checklist
- `AGENTS.md` - Agent decision criteria (updated quarterly)

---

## MAINTENANCE & UPDATES

**Quarterly Review Schedule:**
- Q1: End of March (roadmap update, metrics reset)
- Q2: End of June (mid-year assessment)
- Q3: End of September (pre-release audit)
- Q4: End of December (annual strategic review)

**Update Process:**
1. Run baseline collection (monthly automation)
2. Review metrics vs. targets (quarterly manual)
3. Update roadmap with new P0/P1 actions
4. Publish evaluation report (wiki)
5. Notify team of new priorities

**Authority:** This roadmap is maintained by the Architecture team.
Significant changes (> 20h effort reordering) require engineering leadership review.

---

## SUCCESS STORY MARKER

**When can we declare victory?**

End of Phase 4 (Week 8, 2026-05-31) with these conditions:
- ✓ Test coverage: 108 → 250+ tests (unit + integration)
- ✓ e2e.rs: 2,338 → ~150 lines (modularized)
- ✓ Duplication: 45% → 10% (helpers extracted)
- ✓ SLOs: Undefined → Enforced, 95%+ compliant
- ✓ Regression detection: Manual → Automated, < 1 hour latency
- ✓ Onboarding: 30+ min → < 30 min documented
- ✓ Zero new regressions introduced
- ✓ Quarterly evaluation ritual established & running

**When should we revisit this plan?**

If any of these occur:
- [ ] P0 gate fails two weeks in a row (reassess priorities)
- [ ] New team member can't onboard in < 30 min (update guidance)
- [ ] Test suite latency > 2 min (profile and optimize)
- [ ] Dependency graph becomes too complex (trigger Phase 3 work)
- [ ] New architectural pattern emerges (document in ADR)

---

**Document Status:** LIVE & ACTIVE
**Last Updated:** 2026-02-28 (initial publication)
**Next Review:** 2026-03-31 (end of Q1)
**Maintained By:** Architecture team
**Feedback:** Update via PR with AGENTS.md + roadmap context

