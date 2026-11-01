using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;

namespace HIVE.Coding
{
    /// <summary>
    /// Manages tiered path evaluation, SENTINEL security audits, GAUGE test matching,
    /// and Owner authorization gates.
    /// </summary>
    public interface ITieredDeploymentManager
    {
        DeploymentTier ClassifyPaths(IEnumerable<string> paths);
        Task<bool> ValidateSecurityDiffAsync(string worktreePath, CancellationToken ct = default);
        Task<bool> ValidateTestInventoryAsync(string worktreePath, CancellationToken ct = default);
        Task<bool> AuthorizeTier2DeploymentAsync(string txId, OwnerApprovalRecord approval, CancellationToken ct = default);
        Task<bool> CommitDeploymentAsync(string txId, DeploymentManifest manifest, CancellationToken ct = default);
    }
}
