using System;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace HIVE.Coding
{
    public class CodingTaskRouter
    {
        private readonly ICodingCliAdapter _primaryAdapter;
        private readonly ICodingCliAdapter _fallbackAdapter;
        private readonly ProviderHealthMonitor _healthMonitor;
        private readonly CodingAuditLogger _auditLogger;

        public CodingTaskRouter(
            ICodingCliAdapter primary = null,
            ICodingCliAdapter fallback = null,
            ProviderHealthMonitor healthMonitor = null,
            CodingAuditLogger auditLogger = null)
        {
            _primaryAdapter = primary ?? new ClaudeCodeAdapter();
            _fallbackAdapter = fallback ?? new OpenCodeAdapter();
            _healthMonitor = healthMonitor ?? new ProviderHealthMonitor();
            _auditLogger = auditLogger ?? new CodingAuditLogger();
        }

        public async Task<CodingTaskResult> RouteAndExecuteAsync(
            CodingTaskRequest request,
            string worktreePath,
            Action<string> onOutputLine = null,
            CancellationToken ct = default)
        {
            var result = new CodingTaskResult
            {
                TaskId = request.TaskId,
                WorktreePath = worktreePath
            };

            var cliReq = new CliExecutionRequest
            {
                TaskId = request.TaskId,
                WorktreePath = worktreePath,
                Prompt = request.Prompt,
                TargetFiles = request.TargetScope,
                TimeoutSeconds = request.TimeoutSeconds
            };

            // 1. Check Primary (Claude Code)
            var primaryState = _healthMonitor.GetState(_primaryAdapter.ProviderName);
            bool primaryEligible = primaryState == ProviderState.Available || primaryState == ProviderState.Recovering;

            if (primaryEligible && await _primaryAdapter.IsAvailableAsync(ct).ConfigureAwait(false))
            {
                _auditLogger.LogEvent(request.TaskId, request.AgentCodename, _primaryAdapter.ProviderName, "DISPATCHED", "Executing with primary engine Claude Code.");
                onOutputLine?.Invoke($"[HIVE ROUTER] Dispatching task {request.TaskId} to {_primaryAdapter.ProviderName}...");

                var primaryExec = await _primaryAdapter.ExecuteAsync(cliReq, onOutputLine, ct).ConfigureAwait(false);

                if (primaryExec.Success)
                {
                    _healthMonitor.RecordSuccess(_primaryAdapter.ProviderName);
                    _auditLogger.LogEvent(request.TaskId, request.AgentCodename, _primaryAdapter.ProviderName, "SUCCESS", "Primary execution completed successfully.");

                    result.Success = true;
                    result.ProviderName = _primaryAdapter.ProviderName;
                    result.ModelUsed = "claude-3-7-sonnet";
                    result.Telemetry = primaryExec.Telemetry;
                    result.OutputSummary = primaryExec.Stdout;
                    return result;
                }

                // Primary failed, classify failure
                _healthMonitor.RecordFailure(_primaryAdapter.ProviderName, primaryExec.Failure);
                _auditLogger.LogEvent(request.TaskId, request.AgentCodename, _primaryAdapter.ProviderName, "FAILED", $"Primary failed ({primaryExec.Failure}): {primaryExec.ErrorMessage}");
                onOutputLine?.Invoke($"[HIVE ROUTER] Primary provider {_primaryAdapter.ProviderName} failed ({primaryExec.Failure}). Initiating automatic fallback to {_fallbackAdapter.ProviderName}...");
            }
            else
            {
                onOutputLine?.Invoke($"[HIVE ROUTER] Primary provider {_primaryAdapter.ProviderName} is currently {primaryState}. Routing directly to fallback {_fallbackAdapter.ProviderName}...");
            }

            // 2. Fallback to OpenCode with Context Preservation
            if (await _fallbackAdapter.IsAvailableAsync(ct).ConfigureAwait(false))
            {
                _auditLogger.LogEvent(request.TaskId, request.AgentCodename, _fallbackAdapter.ProviderName, "DISPATCHED", "Executing fallback engine OpenCode with preserved worktree.");

                // Inject context into fallback prompt
                var fallbackPrompt = new StringBuilder();
                fallbackPrompt.AppendLine($"[OBJECTIVE] {request.Prompt}");
                fallbackPrompt.AppendLine($"[WORKTREE] Task ID {request.TaskId} is already isolated in the worktree. Inspect existing progress and complete the requested modifications.");
                cliReq.Prompt = fallbackPrompt.ToString();

                var fallbackExec = await _fallbackAdapter.ExecuteAsync(cliReq, onOutputLine, ct).ConfigureAwait(false);

                if (fallbackExec.Success)
                {
                    _healthMonitor.RecordSuccess(_fallbackAdapter.ProviderName);
                    _auditLogger.LogEvent(request.TaskId, request.AgentCodename, _fallbackAdapter.ProviderName, "SUCCESS", "Fallback execution completed successfully.");

                    result.Success = true;
                    result.ProviderName = _fallbackAdapter.ProviderName;
                    result.ModelUsed = "opencode-free";
                    result.Telemetry = fallbackExec.Telemetry;
                    result.OutputSummary = fallbackExec.Stdout;
                    return result;
                }

                _healthMonitor.RecordFailure(_fallbackAdapter.ProviderName, fallbackExec.Failure);
                result.Failure = fallbackExec.Failure;
                result.ErrorMessage = $"Both primary and fallback coding providers failed. Last error: {fallbackExec.ErrorMessage}";
            }
            else
            {
                result.Failure = FailureCategory.NetworkTimeout;
                result.ErrorMessage = "Both Claude Code and OpenCode are currently unavailable or offline.";
            }

            result.Success = false;
            _auditLogger.LogEvent(request.TaskId, request.AgentCodename, "NONE", "QUEUED", result.ErrorMessage);
            return result;
        }
    }
}
