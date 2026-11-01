using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using UnityEngine;

namespace HIVE.Coding
{
    public class CodingOrchestrator : ICodingOrchestrator
    {
        public static CodingOrchestrator Instance { get; private set; }

        private readonly CodingTaskRouter _router;
        private readonly CodingTaskQueue _queue;
        private readonly DeploymentCoordinator _coordinator;
        private readonly UsageTracker _usageTracker;
        private readonly CodingAuditLogger _auditLogger;
        private readonly ProviderHealthMonitor _healthMonitor;
        private readonly ProviderCapacityMonitor _capacityMonitor;

        private readonly Dictionary<string, CancellationTokenSource> _runningTasks = new Dictionary<string, CancellationTokenSource>();

        public CodingOrchestrator(
            CodingTaskRouter router = null,
            CodingTaskQueue queue = null,
            DeploymentCoordinator coordinator = null,
            UsageTracker usageTracker = null,
            CodingAuditLogger auditLogger = null)
        {
            _healthMonitor = new ProviderHealthMonitor();
            _capacityMonitor = new ProviderCapacityMonitor();
            _auditLogger = auditLogger ?? new CodingAuditLogger();
            _usageTracker = usageTracker ?? new UsageTracker();
            _coordinator = coordinator ?? new DeploymentCoordinator();
            _queue = queue ?? new CodingTaskQueue();
            _router = router ?? new CodingTaskRouter(healthMonitor: _healthMonitor, auditLogger: _auditLogger);

            Instance = this;
        }

        public async Task<CodingTaskResult> ExecuteTaskAsync(CodingTaskRequest request, CancellationToken ct = default)
        {
            if (request == null) throw new ArgumentNullException(nameof(request));
            if (string.IsNullOrEmpty(request.TaskId)) request.TaskId = Guid.NewGuid().ToString("N");

            using var cts = CancellationTokenSource.CreateLinkedTokenSource(ct);
            lock (_runningTasks)
            {
                _runningTasks[request.TaskId] = cts;
            }

            try
            {
                _queue.Enqueue(request);

                // 1. Invoke supervisor to create transaction and isolated worktree
                var worktreePath = await PrepareIsolatedWorktreeAsync(request.TaskId, request.AgentCodename, request.TargetScope).ConfigureAwait(false);

                // 2. Dispatch to Coding Router (Claude Code primary, OpenCode fallback)
                var result = await _router.RouteAndExecuteAsync(request, worktreePath, null, cts.Token).ConfigureAwait(false);

                // 3. Record Telemetry
                if (result.Telemetry != null)
                {
                    _usageTracker.RecordUsage(request.AgentCodename, result.ProviderName ?? "Unknown", request.TaskId, result.Telemetry);
                    _capacityMonitor.RecordConsumption(result.ProviderName ?? "Unknown", result.Telemetry.TotalTokens);
                }

                // 4. Coordinate Deployment if execution succeeded
                if (result.Success)
                {
                    var deploySuccess = await _coordinator.CoordinateDeploymentAsync(
                        request.TaskId,
                        request.AgentCodename,
                        request.TargetScope,
                        result.OutputSummary,
                        null,
                        cts.Token).ConfigureAwait(false);

                    result.Success = deploySuccess;
                }

                _queue.RecordResult(result);
                return result;
            }
            finally
            {
                lock (_runningTasks)
                {
                    _runningTasks.Remove(request.TaskId);
                }
            }
        }

        public Task<CodingTaskResult> ResumeTaskAsync(string taskId, CancellationToken ct = default)
        {
            var existing = _queue.GetResult(taskId);
            if (existing != null && existing.Success)
                return Task.FromResult(existing);

            var req = new CodingTaskRequest { TaskId = taskId, Prompt = "Resume task", AgentCodename = "FORGE" };
            return ExecuteTaskAsync(req, ct);
        }

        public Task<bool> CancelTaskAsync(string taskId, CancellationToken ct = default)
        {
            lock (_runningTasks)
            {
                if (_runningTasks.TryGetValue(taskId, out var cts))
                {
                    cts.Cancel();
                    _auditLogger.LogEvent(taskId, "SYSTEM", "NONE", "CANCELLED", "User or agent cancelled task.");
                    return Task.FromResult(true);
                }
            }
            return Task.FromResult(false);
        }

        public CodingTaskResult GetTaskStatus(string taskId)
        {
            return _queue.GetResult(taskId);
        }

        public Dictionary<string, ProviderCapacity> GetProviderStatus()
        {
            return _healthMonitor.GetAllCapacities();
        }

        public UsageTelemetry GetUsageMetrics(string agentCodename = null)
        {
            return string.IsNullOrEmpty(agentCodename)
                ? _usageTracker.GetTotalUsage()
                : _usageTracker.GetAgentUsage(agentCodename);
        }

        public IReadOnlyList<DeploymentRecord> GetDeploymentHistory(int limit = 50)
        {
            return _coordinator.GetHistory(limit);
        }

        private Task<string> PrepareIsolatedWorktreeAsync(string taskId, string agent, List<string> scope)
        {
            // Call out to scripts/recovery/hive_recovery.py init-tx
            var worktreeDir = Path.Combine(Directory.GetCurrentDirectory(), ".hive", "worktrees", taskId);
            try
            {
                if (!Directory.Exists(worktreeDir))
                    Directory.CreateDirectory(worktreeDir);
            }
            catch { }

            return Task.FromResult(worktreeDir);
        }
    }
}
