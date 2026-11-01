# Claude Code + OpenCode Autonomous Development System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a robust, enterprise-grade Coding Orchestration Layer for all 12 H.I.V.E. agents, pairing Claude Code CLI as the primary engine with OpenCode CLI as an automatic free fallback, backed by an out-of-process Python recovery supervisor enforcing a fail-closed Tiered Autonomy Deployment Policy with compare-and-swap Git ref rollback.

**Architecture:** Unity C# Runtime (.NET Standard 2.1) provides the unified `ICodingOrchestrator`, provider adapters, and tiered deployment coordination; an independent out-of-process Python supervisor (`scripts/recovery/hive_recovery.py`) manages isolated Git worktrees, atomic transaction journals with SHA-256 manifests, Windows Job Object process trees, and compare-and-swap ref deployment/rollback completely outside the agency's self-modifying boundary.

**Tech Stack:** Unity 6000.6.7f1 LTS, C# (.NET Standard 2.1), Python 3.13, Claude Code CLI (v2.1.282), OpenCode CLI (v2.2.5), Git 2.47+, Windows Job Objects / Win32 API.

---

## File Structure & Proposed Decomposition

```text
scripts/recovery/
    hive_recovery.py            # Standalone Python recovery supervisor & continuous watchdog
    hive_recovery.cmd           # Windows batch wrapper for CLI execution & watchdog daemon
    recovery_config.json        # Recovery paths, timeout thresholds, and quarantine policies
    test_recovery.py            # Independent test suite for supervisor, worktrees, locks, and rollback

Assets/Runtime/Coding/
    Abstractions/
        ICodingOrchestrator.cs      # Primary interface shared across all 12 agents
        ICodingCliAdapter.cs        # Unified contract for Claude Code & OpenCode adapters
        IRecoverySupervisor.cs      # C# client interface for IPC communication with hive_recovery.py
        IUsageTracker.cs            # Telemetry, quota forecasting, and cost accounting interface
        ITieredDeploymentManager.cs # Path classification and deployment coordinator contract
        CodingModels.cs             # Enums (ProviderState, DeploymentTier, TransactionStatus) and DTOs

    Providers/
        ClaudeCodeAdapter.cs        # Process launcher for claude.exe, structured output & telemetry parser
        OpenCodeAdapter.cs          # Process launcher for opencode, free-model verifier
        ProviderHealthMonitor.cs    # Provider status tracker with backoff retry & failure classification
        ProviderCapacityMonitor.cs  # Rate-limit, token quota, and capacity forecaster

    Usage/
        UsageTracker.cs             # Rolling usage store (per-agent, per-project, per-provider)
        TokenEstimator.cs           # Level 1-4 token accounting hierarchy
        CostEstimator.cs            # Model pricing and USD estimation
        UsageHistory.cs             # Durable persistence for usage logs

    Deployment/
        TieredDeploymentManager.cs  # In-engine path evaluator and fail-closed tier classifier
        DeploymentValidator.cs      # Static diff security scanner (SENTINEL) & test inventory matcher (GAUGE)
        DeploymentCoordinator.cs    # Two-phase transaction coordinator & Owner approval binder

    Orchestration/
        CodingOrchestrator.cs       # Master service orchestrator wired to HiveController
        CodingTaskRouter.cs         # Provider selection, fallback trigger, and context hand-off
        CodingTaskQueue.cs          # Durable task queue with crash survival
        CodingTaskPersistence.cs    # Disk persistence for active/interrupted tasks

    Audit/
        CodingAuditLogger.cs        # Append-only transaction logger emitting to MessageBus & Repository

Assets/Editor/
    HiveCodingTests.cs          # Unity batchmode integration test suite for the coding layer
```

---

## Task Decomposition

### Task 1: Repository Discovery & Baseline Test Inventory Audit

**Files:**
- Test: `Assets/Editor/HiveE2ETest.cs`
- Run: `hive.cmd test` and inspect actual test inventories

- [ ] **Step 1: Execute existing E2E test suite and extract exact baseline test inventory**
  Run: `.\hive.cmd test`
  Confirm all 16 existing tests pass and capture the exact method identifiers into an inventory manifest.

- [ ] **Step 2: Inspect Claude Code and OpenCode runtime environments**
  Verify executable paths, versions, and argument protocols:
  `claude.exe --version`
  `opencode --version`

- [ ] **Step 3: Document baseline environment manifest**
  Record baseline git commit SHA, active branches, and test identifiers in `docs/superpowers/specs/baseline_inventory.json`.

- [ ] **Step 4: Commit baseline audit documentation**
  ```bash
  git add docs/superpowers/specs/baseline_inventory.json
  git commit -m "chore(audit): establish baseline test and CLI inventory manifest"
  ```

---

### Task 2: Independent Out-of-Process Recovery Supervisor (`scripts/recovery/`)

**Files:**
- Create: `scripts/recovery/recovery_config.json`
- Create: `scripts/recovery/hive_recovery.py`
- Create: `scripts/recovery/hive_recovery.cmd`
- Create: `scripts/recovery/test_recovery.py`

- [ ] **Step 1: Write `recovery_config.json`**
  Define protected paths (Tier 3 supervisor files, Tier 2 core files, Tier 1 files), timeout limits, and quarantine paths.

- [ ] **Step 2: Write unit tests for the recovery supervisor in `scripts/recovery/test_recovery.py`**
  Cover:
  - Atomic transaction journal WAL writes via temp file replacement.
  - SHA-256 manifest calculation and verification.
  - Windows OS-level exclusive file locking with PID, TxId, and creation timestamp.
  - Isolated Git worktree creation and teardown.
  - Independent path classification (symlink and traversal safe).
  - Compare-and-swap ref update (`git update-ref` with old and new SHA).
  - Ownership-checked rollback logic preserving unrelated working-tree edits.

- [ ] **Step 3: Run `test_recovery.py` to verify it fails before implementation**
  Run: `py scripts/recovery/test_recovery.py`
  Expected: FAIL (module not found / implementation missing)

- [ ] **Step 4: Implement `scripts/recovery/hive_recovery.py`**
  Implement:
  - `RecoverySupervisor` class with Windows Job Object process trees (`CreateJobObjectW`, `AssignProcessToJobObject`).
  - Transaction state machine: `CREATED`, `WORKTREE_READY`, `EXECUTING`, `VALIDATING`, `STAGED_AWAITING_APPROVAL`, `APPROVED`, `PREPARED`, `DEPLOYING`, `DEPLOYED_PENDING_HEALTH`, `COMMITTED`, `ROLLBACK_PENDING`, `ROLLED_BACK`, `QUARANTINED`, `FAILED`.
  - Subcommands: `init-tx`, `classify`, `prepare`, `commit`, `rollback`, `watchdog`, `status`.

- [ ] **Step 5: Write `scripts/recovery/hive_recovery.cmd` wrapper**
  Forward CLI arguments with python execution and error level return.

- [ ] **Step 6: Run `py scripts/recovery/test_recovery.py` to verify passes**
  Run: `py scripts/recovery/test_recovery.py`
  Expected: All supervisor tests PASS.

- [ ] **Step 7: Commit Recovery Supervisor**
  ```bash
  git add scripts/recovery/
  git commit -m "feat(recovery): implement independent out-of-process recovery supervisor"
  ```

---

### Task 3: C# Coding Abstractions & Models (`Assets/Runtime/Coding/Abstractions/`)

**Files:**
- Create: `Assets/Runtime/Coding/Abstractions/CodingModels.cs`
- Create: `Assets/Runtime/Coding/Abstractions/ICodingOrchestrator.cs`
- Create: `Assets/Runtime/Coding/Abstractions/ICodingCliAdapter.cs`
- Create: `Assets/Runtime/Coding/Abstractions/IRecoverySupervisor.cs`
- Create: `Assets/Runtime/Coding/Abstractions/IUsageTracker.cs`
- Create: `Assets/Runtime/Coding/Abstractions/ITieredDeploymentManager.cs`

- [ ] **Step 1: Implement `CodingModels.cs`**
  Define enums (`ProviderState`, `DeploymentTier`, `TransactionStatus`, `FailureCategory`) and record/class models (`CodingTaskRequest`, `CodingTaskResult`, `CliExecutionResult`, `ProviderCapacity`, `UsageTelemetry`, `DeploymentManifest`, `OwnerApprovalRecord`). Ensure full .NET Standard 2.1 compatibility.

- [ ] **Step 2: Implement interfaces in `Abstractions/`**
  Write `ICodingOrchestrator.cs`, `ICodingCliAdapter.cs`, `IRecoverySupervisor.cs`, `IUsageTracker.cs`, and `ITieredDeploymentManager.cs` matching the approved specification.

- [ ] **Step 3: Verify compilation in Unity batchmode**
  Run: `.\hive.cmd test`
  Expected: Compilation passes with 16/16 baseline tests passing.

- [ ] **Step 4: Commit Abstractions**
  ```bash
  git add Assets/Runtime/Coding/Abstractions/
  git commit -m "feat(coding): add C# abstractions and data models for coding orchestration"
  ```

---

### Task 4: Provider Adapters & Health/Capacity Monitors (`Assets/Runtime/Coding/Providers/`)

**Files:**
- Create: `Assets/Runtime/Coding/Providers/ClaudeCodeAdapter.cs`
- Create: `Assets/Runtime/Coding/Providers/OpenCodeAdapter.cs`
- Create: `Assets/Runtime/Coding/Providers/ProviderHealthMonitor.cs`
- Create: `Assets/Runtime/Coding/Providers/ProviderCapacityMonitor.cs`

- [ ] **Step 1: Implement `ClaudeCodeAdapter.cs`**
  - Launch `claude.exe` within target worktree directory with `--print` or non-interactive flags.
  - Asynchronously stream standard output and standard error.
  - Parse CLI telemetry and structured exit codes (classify rate limits, auth errors, and clean completions).
  - Enforce clean process-tree termination on timeout/cancellation.

- [ ] **Step 2: Implement `OpenCodeAdapter.cs`**
  - Launch `opencode` with model policy verification (enforce free model selection).
  - Format fallback prompt containing original objective, completed work summary, and partial Git diff.
  - Parse output and telemetry.

- [ ] **Step 3: Implement `ProviderHealthMonitor.cs` and `ProviderCapacityMonitor.cs`**
  - Track states (`Available`, `RateLimited`, `AuthFailed`, `UsageExhausted`, etc.).
  - Implement exponential backoff retry scheduling (5m, 15m, 30m with jitter).
  - Maintain quotas and forecast capacity.

- [ ] **Step 4: Verify Unity compilation and run batchmode tests**
  Run: `.\hive.cmd test`
  Expected: Clean compilation, all tests pass.

- [ ] **Step 5: Commit Provider Adapters**
  ```bash
  git add Assets/Runtime/Coding/Providers/
  git commit -m "feat(coding): add Claude Code and OpenCode adapters with health monitoring"
  ```

---

### Task 5: Usage Tracking & Cost Estimation (`Assets/Runtime/Coding/Usage/`)

**Files:**
- Create: `Assets/Runtime/Coding/Usage/UsageTracker.cs`
- Create: `Assets/Runtime/Coding/Usage/TokenEstimator.cs`
- Create: `Assets/Runtime/Coding/Usage/CostEstimator.cs`
- Create: `Assets/Runtime/Coding/Usage/UsageHistory.cs`

- [ ] **Step 1: Implement `TokenEstimator.cs` and `CostEstimator.cs`**
  - Support the 4-tier telemetry hierarchy (exact provider, CLI reported, tokenizer, character heuristic flagged with `isMeasured: false`).
  - Calculate input, output, cached tokens, and estimated USD expenditure.

- [ ] **Step 2: Implement `UsageTracker.cs` and `UsageHistory.cs`**
  - Track rolling usage per agent, per project, and per provider.
  - Implement atomic write-ahead disk persistence to `hive_store.json` or `.hive/usage_history.json`.

- [ ] **Step 3: Verify Unity batchmode execution**
  Run: `.\hive.cmd test`
  Expected: Pass.

- [ ] **Step 4: Commit Usage Layer**
  ```bash
  git add Assets/Runtime/Coding/Usage/
  git commit -m "feat(coding): implement usage tracking, token estimation, and cost accounting"
  ```

---

### Task 6: Tiered Deployment Manager & Coordinator (`Assets/Runtime/Coding/Deployment/`)

**Files:**
- Create: `Assets/Runtime/Coding/Deployment/TieredDeploymentManager.cs`
- Create: `Assets/Runtime/Coding/Deployment/DeploymentValidator.cs`
- Create: `Assets/Runtime/Coding/Deployment/DeploymentCoordinator.cs`

- [ ] **Step 1: Implement `DeploymentValidator.cs`**
  - SENTINEL check: Scan diffs for leaked credentials (`sk-ant-*`, `nvapi-*`), `.env` files, permission escalations, and dangerous file deletions.
  - GAUGE check: Compare baseline test inventory against candidate test inventory (disallow silent skipping or coverage drop).

- [ ] **Step 2: Implement `TieredDeploymentManager.cs`**
  - Fail-closed path evaluator classifying candidate files into Tier 1, Tier 2, or Tier 3.
  - Communicate with `hive_recovery.py` via IPC/transaction commands.

- [ ] **Step 3: Implement `DeploymentCoordinator.cs`**
  - Orchestrate Phase 1 (Prepare) and Phase 2 (Commit).
  - Enforce Owner authorization requirement for Tier 2: cryptographically verify Owner signature bound to `txId`, candidate commit hash, and manifest SHA-256.

- [ ] **Step 4: Verify Unity batchmode execution**
  Run: `.\hive.cmd test`
  Expected: Pass.

- [ ] **Step 5: Commit Deployment Manager**
  ```bash
  git add Assets/Runtime/Coding/Deployment/
  git commit -m "feat(coding): implement tiered deployment manager and two-phase coordinator"
  ```

---

### Task 7: Master Coding Orchestrator & Task Router (`Assets/Runtime/Coding/Orchestration/` & `Audit/`)

**Files:**
- Create: `Assets/Runtime/Coding/Orchestration/CodingOrchestrator.cs`
- Create: `Assets/Runtime/Coding/Orchestration/CodingTaskRouter.cs`
- Create: `Assets/Runtime/Coding/Orchestration/CodingTaskQueue.cs`
- Create: `Assets/Runtime/Coding/Orchestration/CodingTaskPersistence.cs`
- Create: `Assets/Runtime/Coding/Audit/CodingAuditLogger.cs`

- [ ] **Step 1: Implement `CodingTaskQueue.cs` and `CodingTaskPersistence.cs`**
  - Persistent disk queue capable of surviving crashes and restarts.
  - Restore pending or interrupted tasks on engine boot.

- [ ] **Step 2: Implement `CodingTaskRouter.cs`**
  - Dispatch task to `ClaudeCodeAdapter`.
  - Handle rate limit / offline detection: cleanly terminate Claude process, format fallback payload with partial worktree state and diff, and execute `OpenCodeAdapter`.

- [ ] **Step 3: Implement `CodingAuditLogger.cs`**
  - Stream events to `MessageBus`, record transactions in `Repository`, and write immutable local audit entries.

- [ ] **Step 4: Implement master `CodingOrchestrator.cs`**
  - Ties together Router, Queue, Adapters, Deployment Coordinator, Usage Tracker, and Audit Logger.
  - Implements `ICodingOrchestrator`.

- [ ] **Step 5: Verify Unity batchmode execution**
  Run: `.\hive.cmd test`
  Expected: Pass.

- [ ] **Step 6: Commit Master Orchestrator**
  ```bash
  git add Assets/Runtime/Coding/Orchestration/ Assets/Runtime/Coding/Audit/
  git commit -m "feat(coding): implement master coding orchestrator, task router, and audit logger"
  ```

---

### Task 8: Agency Integration & Skill Binding

**Files:**
- Modify: `Assets/Runtime/Core/HiveController.cs`
- Modify: `Assets/Runtime/Skills/SkillRegistry.cs`
- Modify: `Assets/Runtime/Security/SecurityGate.cs`
- Modify: `Assets/Runtime/Tasks/TaskManager.cs`

- [ ] **Step 1: Register `ICodingOrchestrator` on `HiveController`**
  Initialize `CodingOrchestrator` during boot, wire `hive_recovery.py` watchdog checks, and register instance on `MessageBus`.

- [ ] **Step 2: Add `coding_orchestration` skill to `SkillRegistry`**
  Allow agents with coding permissions (`FORGE`, `SENTINEL`, `GAUGE`, `PRIME`, etc.) to invoke coding actions.

- [ ] **Step 3: Enforce RBAC in `SecurityGate`**
  Validate agent permissions before initiating coding tasks. Prevent unauthorized agents from executing coding actions.

- [ ] **Step 4: Verify Unity batchmode execution**
  Run: `.\hive.cmd test`
  Expected: Pass.

- [ ] **Step 5: Commit Agency Integration**
  ```bash
  git add Assets/Runtime/Core/ Assets/Runtime/Skills/ Assets/Runtime/Security/ Assets/Runtime/Tasks/
  git commit -m "feat(coding): integrate coding orchestrator into HiveController, SkillRegistry, and SecurityGate"
  ```

---

### Task 9: Comprehensive Integration Tests, Failure Injection & End-to-End Verification

**Files:**
- Create: `Assets/Editor/HiveCodingTests.cs`
- Modify: `Assets/Editor/HiveE2ETest.cs`
- Run: `hive.cmd test`
- Run: `hive.cmd capture --all`

- [ ] **Step 1: Author `Assets/Editor/HiveCodingTests.cs`**
  Write tests covering:
  1. `ClaudeCodeAdapter` execution and telemetry extraction.
  2. `OpenCodeAdapter` fallback dispatch upon simulated Claude rate-limit / offline state.
  3. Worktree isolation and on-disk atomic transaction journal schema.
  4. Path tier classification (Tier 1 auto-deploy candidate, Tier 2 Owner-hold, Tier 3 rejection).
  5. Owner authorization validation (token binding to `txId` + manifest hash).
  6. Failure injection: simulated build/test failure triggers compare-and-swap rollback to baseline commit.
  7. Exact test inventory matching (detecting missing or skipped tests).

- [ ] **Step 2: Wire `HiveCodingTests` into `HiveE2ETest.RunAllTests`**
  Update the comprehensive test runner to include the new coding test suite.

- [ ] **Step 3: Run comprehensive test suite via CLI**
  Run: `.\hive.cmd test`
  Expected: ALL SUITES PASS with 100% success rate.

- [ ] **Step 4: Capture verified scene screenshots**
  Run: `.\hive.cmd capture --all --res 1920x1080 --out Screenshots`
  Confirm all 7 operational scenes render cleanly.

- [ ] **Step 5: Commit and push final validated system**
  ```bash
  git add Assets/Editor/ Screenshots/
  git commit -m "feat(coding): complete Claude Code + OpenCode Autonomous Development System with verified integration tests"
  git push origin master
  ```

---

## Plan Verification Checkpoints

1. **Supervisor Independence**: `scripts/recovery/hive_recovery.py` runs and verifies rollback without Unity running.
2. **Provider Failover**: Task started on Claude Code fails over to OpenCode with context intact when Claude is simulated as rate-limited.
3. **Fail-Closed Security**: Any attempt to touch `scripts/recovery/` or `.hive/recovery/` is hard-rejected.
4. **Compare-and-Swap Rollback**: Target ref verified before and after rollback; no `git reset --hard` performed.
5. **No Broken Builds**: Every deployment candidate runs the test suite; failed candidates are quarantined to `.hive/quarantine/`.
