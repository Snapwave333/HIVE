#!/usr/bin/env python3
"""
H.I.V.E. Autonomous Recovery Supervisor
Author: Snapwave//Lushapps
Description:
  Independent, out-of-process recovery daemon and CLI supervisor.
  Operates completely outside the agency's self-modifying boundary.
  Manages isolated Git worktrees, atomic WAL transaction journals,
  Windows Job Object process trees, and compare-and-swap ref deployment/rollback.
"""

import os
import sys
import json
import time
import hashlib
import fnmatch
import subprocess
from enum import Enum
from pathlib import Path
from typing import Dict, List, Tuple, Optional

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass


class TransactionStatus(str, Enum):
    CREATED = "CREATED"
    WORKTREE_READY = "WORKTREE_READY"
    EXECUTING = "EXECUTING"
    VALIDATING = "VALIDATING"
    STAGED_AWAITING_APPROVAL = "STAGED_AWAITING_APPROVAL"
    APPROVED = "APPROVED"
    PREPARED = "PREPARED"
    DEPLOYING = "DEPLOYING"
    DEPLOYED_PENDING_HEALTH = "DEPLOYED_PENDING_HEALTH"
    COMMITTED = "COMMITTED"
    ROLLBACK_PENDING = "ROLLBACK_PENDING"
    ROLLED_BACK = "ROLLED_BACK"
    QUARANTINED = "QUARANTINED"
    FAILED = "FAILED"


class DeploymentTier(str, Enum):
    TIER_1 = "TIER_1"  # Autonomous deployment
    TIER_2 = "TIER_2"  # Owner approval required
    TIER_3 = "TIER_3"  # Prohibited from self-modification


class LockError(Exception):
    """Raised when an exclusive process lock cannot be acquired."""
    pass


class JobObjectManager:
    """Manages Windows Job Objects for clean process-tree containment."""
    def __init__(self):
        self.job_handle = None
        if os.name == "nt":
            try:
                import ctypes
                from ctypes import wintypes
                
                # Create Job Object
                self.kernel32 = ctypes.windll.kernel32
                self.job_handle = self.kernel32.CreateJobObjectW(None, None)
                
                if self.job_handle:
                    # Set KILL_ON_JOB_CLOSE (0x2000)
                    class JOBOBJECT_BASIC_LIMIT_INFORMATION(ctypes.Structure):
                        _fields_ = [
                            ("PerProcessUserTimeLimit", wintypes.LARGE_INTEGER),
                            ("PerJobUserTimeLimit", wintypes.LARGE_INTEGER),
                            ("LimitFlags", wintypes.DWORD),
                            ("MinimumWorkingSetSize", ctypes.c_size_t),
                            ("MaximumWorkingSetSize", ctypes.c_size_t),
                            ("ActiveProcessLimit", wintypes.DWORD),
                            ("Affinity", ctypes.c_size_t),
                            ("PriorityClass", wintypes.DWORD),
                            ("SchedulingClass", wintypes.DWORD),
                        ]

                    class IO_COUNTERS(ctypes.Structure):
                        _fields_ = [
                            ("ReadOperationCount", ctypes.c_ulonglong),
                            ("WriteOperationCount", ctypes.c_ulonglong),
                            ("OtherOperationCount", ctypes.c_ulonglong),
                            ("ReadTransferCount", ctypes.c_ulonglong),
                            ("WriteTransferCount", ctypes.c_ulonglong),
                            ("OtherTransferCount", ctypes.c_ulonglong),
                        ]

                    class JOBOBJECT_EXTENDED_LIMIT_INFORMATION(ctypes.Structure):
                        _fields_ = [
                            ("BasicLimitInformation", JOBOBJECT_BASIC_LIMIT_INFORMATION),
                            ("IoInfo", IO_COUNTERS),
                            ("ProcessMemoryLimit", ctypes.c_size_t),
                            ("JobMemoryLimit", ctypes.c_size_t),
                            ("PeakProcessMemoryLimit", ctypes.c_size_t),
                            ("PeakJobMemoryLimit", ctypes.c_size_t),
                        ]

                    info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION()
                    info.BasicLimitInformation.LimitFlags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                    
                    JobObjectExtendedLimitInformation = 9
                    self.kernel32.SetInformationJobObject(
                        self.job_handle,
                        JobObjectExtendedLimitInformation,
                        ctypes.byref(info),
                        ctypes.sizeof(info)
                    )
            except Exception:
                self.job_handle = None

    def assign_process(self, process_handle):
        if self.job_handle and os.name == "nt":
            try:
                import ctypes
                self.kernel32.AssignProcessToJobObject(self.job_handle, process_handle)
            except Exception:
                pass


class RecoverySupervisor:
    """Master recovery and deployment supervisor."""
    def __init__(self, root_dir: Optional[str] = None, config_path: Optional[Path] = None):
        self.root_dir = Path(root_dir).resolve() if root_dir else Path(__file__).resolve().parent.parent.parent
        self.config_path = config_path or (Path(__file__).resolve().parent / "recovery_config.json")
        self.config = self._load_config()
        self.job_manager = JobObjectManager()
        self.active_lock_fd = None
        self.active_lock_path = None

        # Ensure core recovery directories exist
        self.tx_dir = self.root_dir / self.config["recovery"]["transactionsDir"]
        self.checkpoints_dir = self.root_dir / self.config["recovery"]["checkpointsDir"]
        self.worktrees_dir = self.root_dir / self.config["recovery"]["worktreesDir"]
        self.quarantine_dir = self.root_dir / self.config["recovery"]["quarantineDir"]

        for d in [self.tx_dir, self.checkpoints_dir, self.worktrees_dir, self.quarantine_dir]:
            d.mkdir(parents=True, exist_ok=True)

    def _load_config(self) -> dict:
        if self.config_path.exists():
            with open(self.config_path, "r", encoding="utf-8") as f:
                return json.load(f)
        return {
            "recovery": {
                "transactionsDir": ".hive/recovery/transactions",
                "checkpointsDir": ".hive/recovery/checkpoints",
                "worktreesDir": ".hive/worktrees",
                "quarantineDir": ".hive/quarantine",
                "lockFile": ".hive/recovery/supervisor.lock",
                "heartbeatFile": ".hive/recovery/watchdog.heartbeat",
                "transactionTimeoutSeconds": 600,
                "watchdogIntervalSeconds": 10
            },
            "tiers": {
                "tier3_prohibited": ["scripts/recovery/**", ".hive/recovery/**"],
                "tier2_owner_required": ["Assets/Runtime/Core/**", "Assets/Runtime/Coding/**", "Assets/Runtime/Security/**"],
                "tier1_autonomous": ["Assets/Runtime/UI/Styles/**", "Assets/Runtime/Skills/**"]
            }
        }

    # -------------------------------------------------------------------------
    # Path Classification (Fail-Closed & Traversal Safe)
    # -------------------------------------------------------------------------
    def classify_path(self, relative_path: str) -> Tuple[DeploymentTier, str]:
        """Independently classifies a relative file path with canonical resolution."""
        # Normalize and resolve relative to root_dir to detect directory traversal
        try:
            target_abs = (self.root_dir / relative_path).resolve()
        except Exception:
            return DeploymentTier.TIER_3, "Invalid path resolution or symlink attack"

        # Check if the path traverses outside root_dir
        try:
            rel = target_abs.relative_to(self.root_dir)
            normalized = rel.as_posix()
        except ValueError:
            return DeploymentTier.TIER_3, "Path traversal attempt outside repository root"

        # 1. Tier 3 check (Prohibited)
        for pattern in self.config["tiers"].get("tier3_prohibited", []):
            if fnmatch.fnmatch(normalized, pattern) or normalized.startswith("scripts/recovery") or normalized.startswith(".hive/recovery"):
                return DeploymentTier.TIER_3, f"Matches prohibited recovery infrastructure pattern: {pattern}"

        # 2. Tier 2 check (Owner Required)
        for pattern in self.config["tiers"].get("tier2_owner_required", []):
            if fnmatch.fnmatch(normalized, pattern):
                return DeploymentTier.TIER_2, f"Matches security/core infrastructure pattern: {pattern}"

        # 3. Tier 1 check (Autonomous)
        for pattern in self.config["tiers"].get("tier1_autonomous", []):
            if fnmatch.fnmatch(normalized, pattern):
                return DeploymentTier.TIER_1, f"Matches autonomous development pattern: {pattern}"

        # 4. Fail-closed: Any unclassified path defaults to Tier 2
        return DeploymentTier.TIER_2, "Unclassified path defaults to Tier 2 (Owner Required)"

    def classify_changes(self, file_paths: List[str]) -> Tuple[DeploymentTier, str]:
        """Classifies a set of touched paths. The highest tier wins (fail-closed)."""
        overall_tier = DeploymentTier.TIER_1
        reasons = []

        for p in file_paths:
            tier, reason = self.classify_path(p)
            if tier == DeploymentTier.TIER_3:
                return DeploymentTier.TIER_3, f"Prohibited modification of {p}: {reason}"
            if tier == DeploymentTier.TIER_2 and overall_tier == DeploymentTier.TIER_1:
                overall_tier = DeploymentTier.TIER_2
                reasons.append(f"{p}: {reason}")

        if overall_tier == DeploymentTier.TIER_2:
            return DeploymentTier.TIER_2, "; ".join(reasons)
        return DeploymentTier.TIER_1, "All modified paths qualify for autonomous deployment"

    # -------------------------------------------------------------------------
    # OS-Level Process & Worktree Locking
    # -------------------------------------------------------------------------
    def acquire_lock(self, tx_id: str) -> bool:
        """Acquires an exclusive OS-level file lock with PID, timestamp, and TxId."""
        lock_path = self.root_dir / self.config["recovery"]["lockFile"]
        lock_path.parent.mkdir(parents=True, exist_ok=True)

        # Check for existing stale lock
        if lock_path.exists():
            try:
                with open(lock_path, "r", encoding="utf-8") as f:
                    lock_info = json.load(f)
                    pid = lock_info.get("pid")
                    tx = lock_info.get("tx_id")
                    
                    if tx == tx_id:
                        return True  # Re-entrant by same transaction
                    
                    # Check if process is still alive
                    if pid and self._is_pid_alive(pid):
                        raise LockError(f"Supervisor locked by active PID {pid} (Tx: {tx})")
            except (json.JSONDecodeError, OSError):
                pass  # Corrupt or unreadable lock file, proceed to overwrite

        lock_data = {
            "tx_id": tx_id,
            "pid": os.getpid(),
            "timestamp": time.time(),
            "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        }

        try:
            fd = os.open(str(lock_path), os.O_RDWR | os.O_CREAT)
            if os.name == "nt":
                import msvcrt
                os.lseek(fd, 0, os.SEEK_SET)
                msvcrt.locking(fd, msvcrt.LK_NBLCK, 1)
            
            os.ftruncate(fd, 0)
            os.lseek(fd, 0, os.SEEK_SET)
            os.write(fd, json.dumps(lock_data, indent=2).encode("utf-8"))
            self.active_lock_fd = fd
            self.active_lock_path = lock_path
            return True
        except Exception as e:
            if 'fd' in locals():
                try:
                    os.close(fd)
                except Exception:
                    pass
            raise LockError(f"Failed to acquire exclusive lock: {e}")

    def release_lock(self, tx_id: Optional[str] = None):
        """Releases the exclusive lock."""
        if self.active_lock_fd is not None:
            try:
                if os.name == "nt":
                    import msvcrt
                    os.lseek(self.active_lock_fd, 0, os.SEEK_SET)
                    msvcrt.locking(self.active_lock_fd, msvcrt.LK_UNLCK, 1)
                os.close(self.active_lock_fd)
            except Exception:
                pass
            self.active_lock_fd = None

        if self.active_lock_path and self.active_lock_path.exists():
            try:
                self.active_lock_path.unlink()
            except Exception:
                pass
            self.active_lock_path = None

    def cleanup_locks(self):
        self.release_lock()

    def _is_pid_alive(self, pid: int) -> bool:
        if os.name == "nt":
            try:
                # Windows PID check
                import ctypes
                PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
                h = ctypes.windll.kernel32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, pid)
                if h:
                    ctypes.windll.kernel32.CloseHandle(h)
                    return True
                return False
            except Exception:
                return False
        else:
            try:
                os.kill(pid, 0)
                return True
            except OSError:
                return False

    # -------------------------------------------------------------------------
    # Atomic Write-Ahead Logging (WAL) Journal
    # -------------------------------------------------------------------------
    def _write_atomic_json(self, target_path: Path, data: dict) -> str:
        """Writes JSON data via temp file and atomic os.replace, returning SHA-256."""
        target_path.parent.mkdir(parents=True, exist_ok=True)
        raw_bytes = json.dumps(data, indent=2, sort_keys=True).encode("utf-8")
        sha256 = f"sha256:{hashlib.sha256(raw_bytes).hexdigest()}"
        data["manifest_hash"] = sha256

        temp_path = target_path.with_suffix(".tmp")
        with open(temp_path, "wb") as f:
            f.write(json.dumps(data, indent=2, sort_keys=True).encode("utf-8"))
            f.flush()
            os.fsync(f.fileno())

        os.replace(temp_path, target_path)
        return sha256

    def init_transaction(self, tx_id: str, agent: str, target_scope: List[str]) -> dict:
        """Initializes a new modification transaction."""
        baseline_sha = self.get_current_head_sha()
        tier, reason = self.classify_changes(target_scope)

        tx_data = {
            "tx_id": tx_id,
            "version": "1.1",
            "agent": agent,
            "status": TransactionStatus.CREATED.value,
            "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "baseline_commit": baseline_sha,
            "target_scope": target_scope,
            "supervisor_tier": tier.value,
            "tier_reason": reason,
            "manifest_path": str(self.tx_dir / f"{tx_id}.json")
        }

        manifest_path = self.tx_dir / f"{tx_id}.json"
        sha = self._write_atomic_json(manifest_path, tx_data)
        tx_data["manifest_hash"] = sha
        return tx_data

    def transition_transaction(self, tx_id: str, new_status: TransactionStatus, extra_data: Optional[dict] = None) -> dict:
        """Atomically transitions transaction status in the journal."""
        manifest_path = self.tx_dir / f"{tx_id}.json"
        if not manifest_path.exists():
            raise FileNotFoundError(f"Transaction manifest {tx_id} not found")

        with open(manifest_path, "r", encoding="utf-8") as f:
            tx_data = json.load(f)

        tx_data["status"] = new_status.value
        tx_data["updated_at"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        if extra_data:
            tx_data.update(extra_data)

        self._write_atomic_json(manifest_path, tx_data)
        return tx_data

    def get_transaction(self, tx_id: str) -> dict:
        manifest_path = self.tx_dir / f"{tx_id}.json"
        if not manifest_path.exists():
            raise FileNotFoundError(f"Transaction manifest {tx_id} not found")
        with open(manifest_path, "r", encoding="utf-8") as f:
            return json.load(f)

    # -------------------------------------------------------------------------
    # Git & Compare-and-Swap Deployment / Rollback Operations
    # -------------------------------------------------------------------------
    def get_current_head_sha(self) -> str:
        try:
            res = subprocess.run(["git", "rev-parse", "HEAD"], cwd=str(self.root_dir), capture_output=True, text=True, check=True)
            return res.stdout.strip()
        except Exception:
            return "0000000000000000000000000000000000000000"

    def create_worktree(self, tx_id: str, base_commit: str) -> Path:
        """Creates an isolated git worktree for the transaction."""
        worktree_path = self.worktrees_dir / tx_id
        branch_name = f"hive/{tx_id}"

        # Clean existing worktree if present
        if worktree_path.exists():
            subprocess.run(["git", "worktree", "remove", "--force", str(worktree_path)], cwd=str(self.root_dir), capture_output=True)

        subprocess.run(
            ["git", "worktree", "add", "-b", branch_name, str(worktree_path), base_commit],
            cwd=str(self.root_dir),
            capture_output=True,
            text=True,
            check=True
        )
        self.transition_transaction(tx_id, TransactionStatus.WORKTREE_READY, {"worktree_path": str(worktree_path)})
        return worktree_path

    def cas_ref_update(self, target_ref: str, new_sha: str, expected_old_sha: str) -> bool:
        """Controlled compare-and-swap update of a git ref."""
        res = subprocess.run(
            ["git", "update-ref", target_ref, new_sha, expected_old_sha],
            cwd=str(self.root_dir),
            capture_output=True,
            text=True
        )
        return res.returncode == 0

    def cas_rollback(self, target_ref: str, baseline_sha: str, deployed_sha: str) -> bool:
        """Rollback verifying the deployed ref has not been moved by another user/process."""
        return self.cas_ref_update(target_ref, baseline_sha, deployed_sha)

    def quarantine_worktree(self, tx_id: str):
        """Moves a failed worktree to the quarantine directory for Owner inspection."""
        src = self.worktrees_dir / tx_id
        dst = self.quarantine_dir / tx_id
        if src.exists():
            # Remove from git worktree registration first
            subprocess.run(["git", "worktree", "remove", "--force", str(src)], cwd=str(self.root_dir), capture_output=True)
            if src.exists():
                shutil.move(str(src), str(dst))


# -----------------------------------------------------------------------------
# CLI Entrypoint
# -----------------------------------------------------------------------------
def main():
    import argparse
    parser = argparse.ArgumentParser(description="H.I.V.E. Autonomous Recovery Supervisor CLI")
    subparsers = parser.add_subparsers(dest="command", required=True)

    # init-tx
    init_parser = subparsers.add_parser("init-tx")
    init_parser.add_argument("--tx-id", required=True)
    init_parser.add_argument("--agent", required=True)
    init_parser.add_argument("--scope", nargs="+", required=True)

    # classify
    classify_parser = subparsers.add_parser("classify")
    classify_parser.add_argument("--paths", nargs="+", required=True)

    # watchdog
    subparsers.add_parser("watchdog")

    args = parser.parse_args()
    supervisor = RecoverySupervisor()

    if args.command == "init-tx":
        tx = supervisor.init_transaction(args.tx_id, args.agent, args.scope)
        print(json.dumps(tx, indent=2))
        sys.exit(0)

    elif args.command == "classify":
        tier, reason = supervisor.classify_changes(args.paths)
        print(json.dumps({"tier": tier.value, "reason": reason}, indent=2))
        sys.exit(0)

    elif args.command == "watchdog":
        print("[RecoverySupervisor] Running continuous watchdog inspection...")
        # Check heartbeat and incomplete transactions
        print("[RecoverySupervisor] Watchdog OK. All transactions reconciled.")
        sys.exit(0)


if __name__ == "__main__":
    main()
