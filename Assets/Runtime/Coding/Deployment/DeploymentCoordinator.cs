using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;

namespace HIVE.Coding
{
    public class DeploymentCoordinator
    {
        private readonly ITieredDeploymentManager _deploymentManager;
        private readonly List<DeploymentRecord> _history = new List<DeploymentRecord>();

        public DeploymentCoordinator(ITieredDeploymentManager deploymentManager = null)
        {
            _deploymentManager = deploymentManager ?? new TieredDeploymentManager();
        }

        public async Task<bool> CoordinateDeploymentAsync(
            string txId,
            string agent,
            List<string> touchedFiles,
            string diffText,
            OwnerApprovalRecord approval = null,
            CancellationToken ct = default)
        {
            // 1. Classification
            var tier = _deploymentManager.ClassifyPaths(touchedFiles);

            if (tier == DeploymentTier.Tier3_Prohibited)
            {
                RecordDeployment(txId, agent, tier, TransactionStatus.Failed, false);
                return false;
            }

            // 2. Static security diff check (SENTINEL)
            var securityPassed = await _deploymentManager.ValidateSecurityDiffAsync(diffText, ct).ConfigureAwait(false);
            if (!securityPassed)
            {
                RecordDeployment(txId, agent, tier, TransactionStatus.Failed, false);
                return false;
            }

            // 3. Tier 2 Owner gate check
            if (tier == DeploymentTier.Tier2_OwnerRequired)
            {
                if (approval == null)
                {
                    RecordDeployment(txId, agent, tier, TransactionStatus.StagedAwaitingApproval, false);
                    return false; // Staged awaiting Owner approval
                }

                var authorized = await _deploymentManager.AuthorizeTier2DeploymentAsync(txId, approval, ct).ConfigureAwait(false);
                if (!authorized)
                {
                    RecordDeployment(txId, agent, tier, TransactionStatus.Failed, false);
                    return false;
                }
            }

            // 4. Two-Phase Commit
            var manifest = new DeploymentManifest
            {
                TxId = txId,
                Tier = tier,
                TouchedFiles = touchedFiles,
                AuthorizingOwner = tier == DeploymentTier.Tier1_Autonomous ? "AUTONOMOUS_POLICY" : approval?.OwnerSignature,
                Timestamp = DateTime.UtcNow
            };

            var committed = await _deploymentManager.CommitDeploymentAsync(txId, manifest, ct).ConfigureAwait(false);
            RecordDeployment(txId, agent, tier, committed ? TransactionStatus.Committed : TransactionStatus.Failed, committed);
            return committed;
        }

        private void RecordDeployment(string txId, string agent, DeploymentTier tier, TransactionStatus status, bool healthVerified)
        {
            lock (_history)
            {
                _history.Add(new DeploymentRecord
                {
                    TxId = txId,
                    Agent = agent,
                    Tier = tier,
                    Status = status,
                    DeployedAt = DateTime.UtcNow,
                    HealthVerified = healthVerified
                });
            }
        }

        public IReadOnlyList<DeploymentRecord> GetHistory(int limit = 50)
        {
            lock (_history)
            {
                var count = Math.Min(limit, _history.Count);
                return _history.GetRange(_history.Count - count, count);
            }
        }
    }
}
