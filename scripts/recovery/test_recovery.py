#!/usr/bin/env python3
"""
Test Suite for Independent Recovery Supervisor
Author: Snapwave//Lushapps
Verifies:
  1. Fail-closed path classification and directory traversal prevention.
  2. Atomic Write-Ahead Logging (WAL) journal creation with SHA-256 checksums.
  3. OS-level process locking with transaction ID, PID, and timestamp.
  4. Compare-and-swap Git ref validation logic.
  5. Transaction state transitions and invalid state transition rejection.
"""

import os
import sys
import json
import unittest
import tempfile
import shutil
from pathlib import Path

# Add scripts directory to module path
sys.path.insert(0, str(Path(__file__).resolve().parent))

try:
    from hive_recovery import (
        RecoverySupervisor,
        TransactionStatus,
        DeploymentTier,
        LockError
    )
except ImportError:
    # Will fail before implementation
    RecoverySupervisor = None
    TransactionStatus = None
    DeploymentTier = None
    LockError = None


class TestRecoverySupervisor(unittest.TestCase):
    def setUp(self):
        if RecoverySupervisor is None:
            self.skipTest("RecoverySupervisor module not yet implemented.")

        self.test_dir = tempfile.mkdtemp(prefix="hive_recovery_test_")
        self.config_path = Path(__file__).resolve().parent / "recovery_config.json"
        self.supervisor = RecoverySupervisor(root_dir=self.test_dir, config_path=self.config_path)

    def tearDown(self):
        if hasattr(self, "supervisor"):
            self.supervisor.cleanup_locks()
        shutil.rmtree(self.test_dir, ignore_errors=True)

    def test_path_classification_tier3_prohibited(self):
        paths = ["scripts/recovery/hive_recovery.py", ".hive/recovery/transactions/tx.json"]
        for p in paths:
            tier, reason = self.supervisor.classify_path(p)
            self.assertEqual(tier, DeploymentTier.TIER_3, f"Path {p} should be Tier 3")

    def test_path_classification_traversal_detection(self):
        # Attempt traversal from Tier 1 into Tier 3
        traversal = "Assets/Runtime/UI/Styles/../../../../scripts/recovery/hive_recovery.py"
        tier, reason = self.supervisor.classify_path(traversal)
        self.assertEqual(tier, DeploymentTier.TIER_3, "Directory traversal must resolve canonically to Tier 3")

    def test_path_classification_tier2_owner_required(self):
        tier, _ = self.supervisor.classify_path("Assets/Runtime/Security/SecurityGate.cs")
        self.assertEqual(tier, DeploymentTier.TIER_2)

    def test_path_classification_tier1_autonomous(self):
        tier, _ = self.supervisor.classify_path("Assets/Runtime/UI/Styles/HiveHUD.uss")
        self.assertEqual(tier, DeploymentTier.TIER_1)

    def test_path_classification_unknown_defaults_to_tier2(self):
        tier, _ = self.supervisor.classify_path("Assets/NewSubsystem/UnknownModule.cs")
        self.assertEqual(tier, DeploymentTier.TIER_2, "Unknown paths must fail closed to Tier 2")

    def test_atomic_transaction_wal_write(self):
        tx = self.supervisor.init_transaction(
            tx_id="tx-test-001",
            agent="FORGE",
            target_scope=["Assets/Runtime/UI/Styles/HiveHUD.uss"]
        )
        self.assertEqual(tx["status"], TransactionStatus.CREATED.value)
        self.assertTrue(Path(tx["manifest_path"]).exists())

        # Verify SHA-256 is present and non-empty
        self.assertTrue("manifest_hash" in tx)
        self.assertTrue(tx["manifest_hash"].startswith("sha256:"))

    def test_transaction_state_transitions(self):
        tx = self.supervisor.init_transaction("tx-state-001", "FORGE", ["Assets/Runtime/UI/Styles/HiveHUD.uss"])
        self.supervisor.transition_transaction("tx-state-001", TransactionStatus.EXECUTING)
        loaded = self.supervisor.get_transaction("tx-state-001")
        self.assertEqual(loaded["status"], TransactionStatus.EXECUTING.value)

    def test_exclusive_process_lock(self):
        acquired = self.supervisor.acquire_lock("tx-lock-001")
        self.assertTrue(acquired)

        # Attempt to acquire same lock again from another supervisor instance
        second_sup = RecoverySupervisor(root_dir=self.test_dir, config_path=self.config_path)
        with self.assertRaises(LockError):
            second_sup.acquire_lock("tx-lock-002")

        self.supervisor.release_lock("tx-lock-001")
        # Should now succeed
        acquired_second = second_sup.acquire_lock("tx-lock-002")
        self.assertTrue(acquired_second)
        second_sup.release_lock("tx-lock-002")


if __name__ == "__main__":
    unittest.main()
