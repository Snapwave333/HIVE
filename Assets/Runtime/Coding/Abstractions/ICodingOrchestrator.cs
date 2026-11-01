using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;

namespace HIVE.Coding
{
    /// <summary>
    /// Master interface for all 12 H.I.V.E. agents to request autonomous coding,
    /// debugging, refactoring, testing, and self-modification services.
    /// </summary>
    public interface ICodingOrchestrator
    {
        Task<CodingTaskResult> ExecuteTaskAsync(CodingTaskRequest request, CancellationToken ct = default);
        Task<CodingTaskResult> ResumeTaskAsync(string taskId, CancellationToken ct = default);
        Task<bool> CancelTaskAsync(string taskId, CancellationToken ct = default);
        CodingTaskResult GetTaskStatus(string taskId);
        
        Dictionary<string, ProviderCapacity> GetProviderStatus();
        UsageTelemetry GetUsageMetrics(string agentCodename = null);
        IReadOnlyList<DeploymentRecord> GetDeploymentHistory(int limit = 50);
    }
}
