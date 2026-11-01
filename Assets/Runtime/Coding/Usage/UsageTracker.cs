using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Linq;

namespace HIVE.Coding
{
    public class UsageTracker : IUsageTracker
    {
        private readonly UsageHistory _history;
        private readonly ConcurrentDictionary<string, UsageTelemetry> _agentTotals = new ConcurrentDictionary<string, UsageTelemetry>();
        private readonly ConcurrentDictionary<string, UsageTelemetry> _providerTotals = new ConcurrentDictionary<string, UsageTelemetry>();

        public UsageTracker(string storagePath = null)
        {
            _history = new UsageHistory(storagePath);
            ReplayHistory();
        }

        private void ReplayHistory()
        {
            foreach (var entry in _history.GetAll())
            {
                Accumulate(_agentTotals, entry.Agent, entry.Telemetry);
                Accumulate(_providerTotals, entry.Provider, entry.Telemetry);
            }
        }

        public void RecordUsage(string agent, string provider, string taskId, UsageTelemetry telemetry)
        {
            if (telemetry == null) return;

            _history.Append(agent, provider, taskId, telemetry);
            Accumulate(_agentTotals, agent, telemetry);
            Accumulate(_providerTotals, provider, telemetry);
        }

        public UsageTelemetry GetAgentUsage(string agent)
        {
            return _agentTotals.TryGetValue(agent, out var val) ? CloneTelemetry(val) : new UsageTelemetry();
        }

        public UsageTelemetry GetTotalUsage()
        {
            var total = new UsageTelemetry { IsMeasured = true };
            foreach (var val in _providerTotals.Values)
            {
                total.InputTokens += val.InputTokens;
                total.OutputTokens += val.OutputTokens;
                total.CachedTokens += val.CachedTokens;
                total.EstimatedCostUsd += val.EstimatedCostUsd;
                if (!val.IsMeasured) total.IsMeasured = false;
            }
            return total;
        }

        public decimal ForecastQuotaRemaining(string provider)
        {
            // Simple linear projection based on current session usage
            var used = _providerTotals.TryGetValue(provider, out var val) ? val.EstimatedCostUsd : 0m;
            decimal defaultCap = provider == "ClaudeCode" ? 100.00m : 0.00m;
            return Math.Max(0, defaultCap - used);
        }

        public IReadOnlyList<UsageTelemetry> GetHistory(int limit = 100)
        {
            return _history.GetAll()
                .OrderByDescending(e => e.Timestamp)
                .Take(limit)
                .Select(e => e.Telemetry)
                .ToList();
        }

        private static void Accumulate(ConcurrentDictionary<string, UsageTelemetry> dict, string key, UsageTelemetry add)
        {
            if (string.IsNullOrEmpty(key) || add == null) return;

            dict.AddOrUpdate(key,
                k => CloneTelemetry(add),
                (k, existing) =>
                {
                    existing.InputTokens += add.InputTokens;
                    existing.OutputTokens += add.OutputTokens;
                    existing.CachedTokens += add.CachedTokens;
                    existing.EstimatedCostUsd += add.EstimatedCostUsd;
                    if (!add.IsMeasured) existing.IsMeasured = false;
                    return existing;
                });
        }

        private static UsageTelemetry CloneTelemetry(UsageTelemetry src)
        {
            return new UsageTelemetry
            {
                IsMeasured = src.IsMeasured,
                InputTokens = src.InputTokens,
                OutputTokens = src.OutputTokens,
                CachedTokens = src.CachedTokens,
                EstimatedCostUsd = src.EstimatedCostUsd
            };
        }
    }
}
