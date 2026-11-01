using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;

namespace HIVE.Coding
{
    /// <summary>
    /// Contract for communicating with the out-of-process recovery supervisor (hive_recovery.py).
    /// </summary>
    public interface IRecoverySupervisor
    {
        Task<DeploymentManifest> InitTransactionAsync(string txId, string agent, List<string> targetScope, CancellationToken ct = default);
        Task<DeploymentTier> ClassifyPathsAsync(List<string> paths, CancellationToken ct = default);
        Task<string> CreateWorktreeAsync(string txId, string baseCommit, CancellationToken ct = default);
        Task<bool> ProposeDeploymentAsync(string txId, DeploymentManifest manifest, CancellationToken ct = default);
        Task<bool> ExecuteRollbackAsync(string txId, string baselineCommit, string deployedCommit, CancellationToken ct = default);
        Task<bool> VerifyWatchdogHealthAsync(CancellationToken ct = default);
    }
}
