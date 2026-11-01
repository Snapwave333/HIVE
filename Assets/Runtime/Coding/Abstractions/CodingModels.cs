using System;
using System.Collections.Generic;

namespace HIVE.Coding
{
    public enum ProviderState
    {
        Available,
        Unavailable,
        RateLimited,
        AuthFailed,
        UsageExhausted,
        Recovering,
        Disabled
    }

    public enum DeploymentTier
    {
        Tier1_Autonomous = 1,
        Tier2_OwnerRequired = 2,
        Tier3_Prohibited = 3
    }

    public enum TransactionStatus
    {
        Created,
        WorktreeReady,
        Executing,
        Validating,
        StagedAwaitingApproval,
        Approved,
        Prepared,
        Deploying,
        DeployedPendingHealth,
        Committed,
        RollbackPending,
        RolledBack,
        Quarantined,
        Failed
    }

    public enum FailureCategory
    {
        None,
        RateLimit,
        Authentication,
        NetworkTimeout,
        ProcessCrash,
        CompilerError,
        TestFailure,
        SecurityDiffViolation,
        RefConflict,
        SupervisorRejection
    }

    [Serializable]
    public class UsageTelemetry
    {
        public bool IsMeasured;
        public long InputTokens;
        public long OutputTokens;
        public long CachedTokens;
        public decimal EstimatedCostUsd;

        public long TotalTokens => InputTokens + OutputTokens;
    }

    [Serializable]
    public class CodingTaskRequest
    {
        public string TaskId;
        public string AgentCodename;
        public string Prompt;
        public List<string> TargetScope = new List<string>();
        public long EstimatedTokenBudget;
        public int TimeoutSeconds = 300;
    }

    [Serializable]
    public class CodingTaskResult
    {
        public string TaskId;
        public bool Success;
        public string ProviderName;
        public string ModelUsed;
        public UsageTelemetry Telemetry = new UsageTelemetry();
        public string WorktreePath;
        public DeploymentTier Tier;
        public string DiffSummary;
        public string OutputSummary;
        public FailureCategory Failure;
        public string ErrorMessage;
    }

    [Serializable]
    public class CliExecutionRequest
    {
        public string TaskId;
        public string WorktreePath;
        public string Prompt;
        public List<string> TargetFiles = new List<string>();
        public int TimeoutSeconds = 300;
    }

    [Serializable]
    public class CliExecutionResult
    {
        public bool Success;
        public int ExitCode;
        public string Stdout;
        public string Stderr;
        public UsageTelemetry Telemetry = new UsageTelemetry();
        public FailureCategory Failure;
        public string ErrorMessage;
    }

    [Serializable]
    public class ProviderCapacity
    {
        public string ProviderName;
        public bool IsAvailable;
        public ProviderState State;
        public long RemainingDailyTokens;
        public int RateLimitResetSeconds;
    }

    [Serializable]
    public class DeploymentManifest
    {
        public string TxId;
        public string BaselineCommit;
        public string CandidateCommit;
        public string ManifestHash;
        public DeploymentTier Tier;
        public List<string> TouchedFiles = new List<string>();
        public string AuthorizingOwner;
        public DateTime Timestamp;
    }

    [Serializable]
    public class OwnerApprovalRecord
    {
        public string TxId;
        public string CandidateCommit;
        public string ManifestHash;
        public string OwnerSignature;
        public DateTime Timestamp;
    }

    [Serializable]
    public class DeploymentRecord
    {
        public string TxId;
        public string Agent;
        public DeploymentTier Tier;
        public TransactionStatus Status;
        public string BaselineCommit;
        public string DeployedCommit;
        public DateTime DeployedAt;
        public bool HealthVerified;
    }
}
