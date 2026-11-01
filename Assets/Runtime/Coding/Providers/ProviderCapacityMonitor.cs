using System.Collections.Concurrent;

namespace HIVE.Coding
{
    public class ProviderCapacityMonitor
    {
        private readonly ConcurrentDictionary<string, long> _consumedTokens = new ConcurrentDictionary<string, long>();
        private readonly ConcurrentDictionary<string, long> _dailyQuotas = new ConcurrentDictionary<string, long>();

        public ProviderCapacityMonitor()
        {
            _dailyQuotas["ClaudeCode"] = 2000000;
            _dailyQuotas["OpenCode"] = 10000000;
        }

        public void RecordConsumption(string provider, long tokens)
        {
            _consumedTokens.AddOrUpdate(provider, tokens, (k, old) => old + tokens);
        }

        public long GetRemainingCapacity(string provider)
        {
            var quota = _dailyQuotas.TryGetValue(provider, out var q) ? q : 1000000;
            var consumed = _consumedTokens.TryGetValue(provider, out var c) ? c : 0;
            return System.Math.Max(0, quota - consumed);
        }

        public bool IsWithinQuota(string provider, long requiredTokens)
        {
            return GetRemainingCapacity(provider) >= requiredTokens;
        }
    }
}
