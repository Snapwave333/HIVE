using System;
using System.Threading;
using System.Threading.Tasks;

namespace HIVE.Coding
{
    /// <summary>
    /// Unified abstraction for coding engine adapters (Claude Code and OpenCode).
    /// </summary>
    public interface ICodingCliAdapter
    {
        string ProviderName { get; }
        bool IsFallback { get; }
        
        Task<bool> IsAvailableAsync(CancellationToken ct = default);
        Task<ProviderCapacity> CheckCapacityAsync(CancellationToken ct = default);
        Task<CliExecutionResult> ExecuteAsync(CliExecutionRequest request, Action<string> onOutputLine, CancellationToken ct = default);
        FailureCategory ClassifyExit(int exitCode, string stderr, string stdout);
    }
}
