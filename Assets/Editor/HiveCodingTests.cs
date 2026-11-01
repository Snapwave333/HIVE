using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using UnityEngine;
using HIVE.Coding;

namespace HIVE.Editor
{
    public static class HiveCodingTests
    {
        public static (int passed, int failed) RunCodingTests(Action<string, bool, string> assertCallback)
        {
            int passed = 0;
            int failed = 0;

            void Assert(string testName, bool condition, string detail = null)
            {
                if (condition)
                {
                    passed++;
                    assertCallback?.Invoke($"[CODING] {testName}", true, detail);
                }
                else
                {
                    failed++;
                    assertCallback?.Invoke($"[CODING] {testName}", false, detail);
                }
            }

            // TEST 1: Claude Code Adapter Initialization & Availability
            var claudeAdapter = new ClaudeCodeAdapter();
            var claudeAvailable = claudeAdapter.IsAvailableAsync(CancellationToken.None).GetAwaiter().GetResult();
            Assert("ClaudeCodeAdapter Availability", claudeAvailable, $"Executable: {claudeAdapter.ProviderName}");

            // TEST 2: OpenCode Adapter Free-Model Fallback
            var openCodeAdapter = new OpenCodeAdapter(freeModel: "free");
            var openCodeAvailable = openCodeAdapter.IsAvailableAsync(CancellationToken.None).GetAwaiter().GetResult();
            Assert("OpenCodeAdapter Availability & Policy", openCodeAvailable && openCodeAdapter.IsFallback, "Configured as verified free model fallback");

            // TEST 3: Fail-Closed Tiered Path Classification
            var tierManager = new TieredDeploymentManager();
            var t1 = tierManager.ClassifyPaths(new[] { "Assets/Runtime/UI/Styles/HiveHUD.uss" });
            var t2 = tierManager.ClassifyPaths(new[] { "Assets/Runtime/Core/HiveController.cs" });
            var t3 = tierManager.ClassifyPaths(new[] { "scripts/recovery/hive_recovery.py" });
            var unknown = tierManager.ClassifyPaths(new[] { "Assets/Unknown/NewModule.cs" });

            bool tierChecks = (t1 == DeploymentTier.Tier1_Autonomous) &&
                              (t2 == DeploymentTier.Tier2_OwnerRequired) &&
                              (t3 == DeploymentTier.Tier3_Prohibited) &&
                              (unknown == DeploymentTier.Tier2_OwnerRequired); // fail-closed

            Assert("Fail-Closed Tiered Classification", tierChecks, $"T1: {t1}, T2: {t2}, T3: {t3}, Unknown: {unknown}");

            // TEST 4: SENTINEL Static Diff Security Scanner
            var validator = new DeploymentValidator();
            var safeDiff = "--- a/test.txt\n+++ b/test.txt\n@@ -1 +1 @@\n-old\n+new";
            var leakedSecretDiff = "--- a/test.txt\n+++ b/test.txt\n@@ -1 +1 @@\n+sk-ant-api03-abcdefghijklmnopqrstuvwxyz123456";

            var safeResult = validator.ValidateSecurityDiffAsync(safeDiff).GetAwaiter().GetResult();
            var leakedResult = validator.ValidateSecurityDiffAsync(leakedSecretDiff).GetAwaiter().GetResult();
            Assert("SENTINEL Security Diff Validation", safeResult && !leakedResult, "Clean diff passed; leaked secret rejected");

            // TEST 5: GAUGE Test Inventory Matcher
            var baseline = new List<string> { "TestA", "TestB", "TestC" };
            var candidatePass = new List<string> { "TestA", "TestB", "TestC", "TestD" };
            var candidateDrop = new List<string> { "TestA", "TestC" }; // Missing TestB!

            var inventoryPass = validator.ValidateTestInventory(baseline, candidatePass);
            var inventoryFail = !validator.ValidateTestInventory(baseline, candidateDrop);
            Assert("GAUGE Test Inventory Matching", inventoryPass && inventoryFail, "Disallowed silent test skipping");

            // TEST 6: Usage Tracker & Cost Accounting
            var tracker = new UsageTracker();
            var telemetry = new UsageTelemetry
            {
                IsMeasured = true,
                InputTokens = 1000,
                OutputTokens = 500,
                CachedTokens = 200,
                EstimatedCostUsd = 0.0105m
            };
            tracker.RecordUsage("FORGE", "ClaudeCode", "tx-test-01", telemetry);
            var forgeUsage = tracker.GetAgentUsage("FORGE");
            Assert("UsageTracker Accounting", forgeUsage != null && forgeUsage.InputTokens >= 1000, $"Recorded {forgeUsage?.InputTokens} tokens, Cost: ${forgeUsage?.EstimatedCostUsd}");

            // TEST 7: Owner Approval Gate Enforcement
            var coordinator = new DeploymentCoordinator(tierManager);
            var approval = new OwnerApprovalRecord
            {
                TxId = "tx-core-01",
                CandidateCommit = "abc1234",
                ManifestHash = "sha256-dummy",
                OwnerSignature = "OWNER_SIG_AUTHORIZED_TOKEN_999",
                Timestamp = DateTime.UtcNow
            };

            var tier2WithoutApproval = coordinator.CoordinateDeploymentAsync("tx-core-01", "FORGE", new List<string> { "Assets/Runtime/Core/HiveController.cs" }, safeDiff, null).GetAwaiter().GetResult();
            var tier2WithApproval = coordinator.CoordinateDeploymentAsync("tx-core-01", "FORGE", new List<string> { "Assets/Runtime/Core/HiveController.cs" }, safeDiff, approval).GetAwaiter().GetResult();
            Assert("Owner Approval Gate Enforcement", !tier2WithoutApproval && tier2WithApproval, "Unapproved Tier 2 held; Owner signed Tier 2 deployed");

            // TEST 8: Task Persistence & Queue
            var queue = new CodingTaskQueue();
            queue.Enqueue(new CodingTaskRequest { TaskId = "tx-queue-test", AgentCodename = "FORGE", Prompt = "Unit test" });
            bool dequeued = queue.TryDequeue(out var req);
            Assert("Durable Task Queue", dequeued && req.TaskId == "tx-queue-test", $"Dequeued Task: {req?.TaskId}");

            return (passed, failed);
        }
    }
}
