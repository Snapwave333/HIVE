using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using UnityEditor;
using UnityEngine;
using HIVE.Core;
using HIVE.Persistence;
using HIVE.Agents;
using HIVE.Tasks;
using HIVE.Memory;
using HIVE.Security;
using HIVE.Skills;
using HIVE.LLM;

namespace HIVE.Editor
{
    /// <summary>
    /// Comprehensive end-to-end integration and smoke test suite for the H.I.V.E. platform.
    /// Can be executed from Unity batchmode or via the Unity Editor menu.
    /// </summary>
    public static class HiveE2ETest
    {
        [MenuItem("HIVE/Run End-to-End Test Suite")]
        public static void RunAllTests()
        {
            Debug.Log("==================================================================");
            Debug.Log("[HIVE E2E] STARTING COMPREHENSIVE END-TO-END VALIDATION SUITE");
            Debug.Log("==================================================================");

            int passed = 0;
            int failed = 0;

            void AssertTrue(string testName, bool condition, string detail = null)
            {
                if (condition)
                {
                    Debug.Log($"[HIVE E2E] [PASS] {testName} {(detail != null ? $"-- {detail}" : "")}");
                    passed++;
                }
                else
                {
                    Debug.LogError($"[HIVE E2E] [FAIL] {testName} {(detail != null ? $"-- {detail}" : "")}");
                    failed++;
                }
            }

            try
            {
                // TEST 1: Repository & SQLite Persistence
                var testDb = Path.Combine(Application.temporaryCachePath, $"hive_e2e_{Guid.NewGuid():N}.db");
                using var repo = new Repository(testDb);
                AssertTrue("Repository Initialization", File.Exists(testDb), $"Database active at {testDb}");

                // TEST 2: Agent Roster JSON Schemas
                var agentsDir = Path.Combine(Application.streamingAssetsPath, "agents");
                var agentFiles = Directory.GetFiles(agentsDir, "*.json");
                AssertTrue("12 Agent Schemas Present", agentFiles.Length == 12, $"Found {agentFiles.Length} agent schema files");

                var expectedAgents = new[] { "PRIME", "SAGE", "FORGE", "SENTINEL", "VOICE", "ARCHIVIST", "CONDUCTOR", "VISION", "LEDGER", "LINK", "GAUGE", "SPARK" };
                var loadedCodenames = agentFiles.Select(f => Path.GetFileNameWithoutExtension(f)).ToArray();
                var allPresent = expectedAgents.All(a => loadedCodenames.Contains(a));
                AssertTrue("Permanent Roster Matching", allPresent, $"All 12 codenames verified: {string.Join(", ", loadedCodenames)}");

                // TEST 3: MessageBus Pub/Sub & Request/Response
                bool receivedEvent = false;
                var sub = MessageBus.Subscribe<TestEventMessage>(msg =>
                {
                    if (msg.Content == "Borg_E2E_Test") receivedEvent = true;
                });
                MessageBus.Publish(new TestEventMessage { Content = "Borg_E2E_Test", SourceAgent = "PRIME" });
                sub.Dispose();
                AssertTrue("MessageBus Pub/Sub", receivedEvent, "Published message routed to typed subscriber");

                // TEST 4: TaskManager Lifecycle
                var taskGO = new GameObject("TestTaskManager");
                var taskManager = taskGO.AddComponent<TaskManager>();
                taskManager.Initialize(repo);

                var taskId = taskManager.CreateTask(new TaskRecord
                {
                    Title = "Deploy High-Throughput Routing Node",
                    Description = "Verify NIM cascade auto-routing and circuit breakers",
                    Priority = 1,
                    AcceptanceCriteria = "Zero 5xx unhandled errors"
                });
                AssertTrue("Task Creation", !string.IsNullOrEmpty(taskId), $"Created task ID: {taskId}");

                taskManager.AssignTask(taskId, "FORGE", "PROJECT_NIM");
                var task = taskManager.GetTask(taskId);
                AssertTrue("Task Assignment", task != null && task.Assignee == "FORGE" && task.State == "Queued", $"State: {task?.State}, Assignee: {task?.Assignee}");

                taskManager.UpdateState(taskId, "InProgress", "FORGE");
                taskManager.UpdateState(taskId, "Completed", "FORGE");
                taskManager.UpdateState(taskId, "Verified", "GAUGE");
                task = taskManager.GetTask(taskId);
                AssertTrue("Task Lifecycle Verification", task != null && task.State == "Verified", $"Final terminal state: {task?.State}");
                GameObject.DestroyImmediate(taskGO);

                // TEST 5: SecurityGate RBAC & Audit Trails
                var secGO = new GameObject("TestSecurityGate");
                var secGate = secGO.AddComponent<SecurityGate>();
                secGate.Initialize(repo);

                var primeDef = new AgentDefinition { Codename = "PRIME", Permissions = new Permissions { Execute = new List<string> { "all" } } };
                var sparkDef = new AgentDefinition { Codename = "SPARK", Permissions = new Permissions { Read = new List<string> { "all" }, Execute = new List<string> { "prototype" } } };

                var primeAllowed = secGate.CheckPermission(primeDef, "execute", "agent_spawn");
                var sparkAllowed = secGate.CheckPermission(sparkDef, "execute", "prototype");
                var sparkDenied = !secGate.CheckPermission(sparkDef, "execute", "agent_spawn");

                AssertTrue("SecurityGate RBAC Permissions", primeAllowed && sparkAllowed && sparkDenied, "PRIME wildcard passed, SPARK restricted properly");

                secGate.EngageEmergencyLockdown("E2E Lockdown Verification");
                AssertTrue("Emergency Lockdown Engagement", secGate.EmergencyLockdownActive, "Lockdown flag active");

                var blockedUnderLockdown = !secGate.CheckPermission(sparkDef, "execute", "prototype");
                AssertTrue("Lockdown Action Interception", blockedUnderLockdown, "Actions blocked during active lockdown");

                secGate.DisengageEmergencyLockdown();
                AssertTrue("Emergency Lockdown Disengagement", !secGate.EmergencyLockdownActive, "Lockdown lifted cleanly");
                GameObject.DestroyImmediate(secGO);

                // TEST 6: SkillRegistry Execution
                var skillGO = new GameObject("TestSkillRegistry");
                var skillReg = skillGO.AddComponent<SkillRegistry>();
                skillReg.Initialize(repo);

                var skillTask = skillReg.ExecuteSkillAsync("strategic_planning", "PRIME");
                skillTask.Wait(TimeSpan.FromSeconds(5));
                var skillRes = skillTask.Result;
                AssertTrue("Skill Execution", skillRes.Success, $"Output: {skillRes.Output}");
                GameObject.DestroyImmediate(skillGO);

                // TEST 7: MemoryStore
                var memGO = new GameObject("TestMemoryStore");
                var memStore = memGO.AddComponent<MemoryStore>();
                memStore.Initialize(repo);

                memStore.Store(new MemoryRecord
                {
                    Type = "Episodic",
                    Content = "Initial agency standup completed at 08:00 UTC",
                    Source = "PRIME",
                    AgentId = "PRIME"
                });
                var memQuery = memStore.Search(query: "agency standup");
                AssertTrue("MemoryStore Query", memQuery != null && memQuery.Count > 0, $"Retrieved {memQuery?.Count ?? 0} memory records matching query");
                GameObject.DestroyImmediate(memGO);

                // TEST 8: NIM Cascade Router Health State & Configuration
                var activeProvider = NimCascadeRouter.GetProvider();
                var textModels = NimCascadeRouter.GetCascade("text");
                var visionModels = NimCascadeRouter.GetCascade("vision");
                AssertTrue("NimCascadeRouter State Check", textModels != null && textModels.Length > 0, $"Active Provider: {activeProvider}, Text Models: {textModels?.Length ?? 0}, Vision Models: {visionModels?.Length ?? 0}");

                // TEST 9: Scene Generation & Build Settings
                HiveSceneBuilder.BuildAll();
                var buildScenes = EditorBuildSettings.scenes;
                AssertTrue("7 HQ Operational Scenes Built", buildScenes.Length == 7 && buildScenes[0].path.Contains("CommandChamber"), $"Configured {buildScenes.Length} scenes. Root: {buildScenes[0].path}");

                // TEST 10: Prefab Integrity
                var wsPrefab = AssetDatabase.LoadAssetAtPath<GameObject>("Assets/Prefabs/AgentWorkspacePrefab.prefab");
                var avatarPrefab = AssetDatabase.LoadAssetAtPath<GameObject>("Assets/Prefabs/AvatarPrefab.prefab");
                AssertTrue("Cybernetic Prefabs Validated", wsPrefab != null && avatarPrefab != null && wsPrefab.GetComponent<AgentWorkspace>() != null, "AgentWorkspacePrefab and AvatarPrefab active on disk");

                // TEST 11-18: Autonomous Coding Orchestration & Recovery Suite
                HiveCodingTests.RunCodingTests((name, ok, detail) => AssertTrue(name, ok, detail));

                // Clean up test DB
                try { if (File.Exists(testDb)) File.Delete(testDb); } catch { }
            }
            catch (Exception ex)
            {
                Debug.LogError($"[HIVE E2E] UNHANDLED EXCEPTION DURING TESTS: {ex}");
                failed++;
            }

            Debug.Log("==================================================================");
            Debug.Log($"[HIVE E2E] VALIDATION COMPLETE -- PASSED: {passed} | FAILED: {failed}");
            Debug.Log("==================================================================");

            if (failed > 0)
            {
                EditorApplication.Exit(1);
            }
            else
            {
                EditorApplication.Exit(0);
            }
        }

        public class TestEventMessage : MessageBus.IMessage
        {
            public string MessageId { get; set; }
            public DateTime Timestamp { get; set; }
            public string SourceAgent { get; set; }
            public string TargetAgent { get; set; }
            public string Content { get; set; }
        }
    }
}
