using System.Collections.Generic;

namespace HIVE.Coding
{
    /// <summary>
    /// Tracks rolling token consumption, cost accounting, and quota forecasting.
    /// </summary>
    public interface IUsageTracker
    {
        void RecordUsage(string agent, string provider, string taskId, UsageTelemetry telemetry);
        UsageTelemetry GetAgentUsage(string agent);
        UsageTelemetry GetTotalUsage();
        decimal ForecastQuotaRemaining(string provider);
        IReadOnlyList<UsageTelemetry> GetHistory(int limit = 100);
    }
}
