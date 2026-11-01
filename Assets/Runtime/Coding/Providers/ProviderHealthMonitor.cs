using System;
using System.Collections.Concurrent;
using System.Collections.Generic;

namespace HIVE.Coding
{
    public class ProviderHealthMonitor
    {
        private class HealthRecord
        {
            public ProviderState State = ProviderState.Available;
            public DateTime LastChecked = DateTime.UtcNow;
            public DateTime? RateLimitExpiresAt;
            public int ConsecutiveFailures = 0;
            public FailureCategory LastFailure = FailureCategory.None;
        }

        private readonly ConcurrentDictionary<string, HealthRecord> _records = new ConcurrentDictionary<string, HealthRecord>();

        public ProviderState GetState(string providerName)
        {
            if (!_records.TryGetValue(providerName, out var record))
                return ProviderState.Available;

            if (record.State == ProviderState.RateLimited && record.RateLimitExpiresAt.HasValue)
            {
                if (DateTime.UtcNow >= record.RateLimitExpiresAt.Value)
                {
                    record.State = ProviderState.Recovering;
                    record.RateLimitExpiresAt = null;
                }
            }

            return record.State;
        }

        public void RecordSuccess(string providerName)
        {
            var record = _records.GetOrAdd(providerName, _ => new HealthRecord());
            record.State = ProviderState.Available;
            record.ConsecutiveFailures = 0;
            record.LastFailure = FailureCategory.None;
            record.LastChecked = DateTime.UtcNow;
            record.RateLimitExpiresAt = null;
        }

        public void RecordFailure(string providerName, FailureCategory failure)
        {
            var record = _records.GetOrAdd(providerName, _ => new HealthRecord());
            record.ConsecutiveFailures++;
            record.LastFailure = failure;
            record.LastChecked = DateTime.UtcNow;

            switch (failure)
            {
                case FailureCategory.RateLimit:
                    record.State = ProviderState.RateLimited;
                    // Exponential backoff: 5m, 15m, 30m
                    var minutes = record.ConsecutiveFailures == 1 ? 5 : (record.ConsecutiveFailures == 2 ? 15 : 30);
                    record.RateLimitExpiresAt = DateTime.UtcNow.AddMinutes(minutes);
                    break;

                case FailureCategory.Authentication:
                    record.State = ProviderState.AuthFailed;
                    break;

                default:
                    if (record.ConsecutiveFailures >= 3)
                        record.State = ProviderState.Unavailable;
                    break;
            }
        }

        public Dictionary<string, ProviderCapacity> GetAllCapacities()
        {
            var result = new Dictionary<string, ProviderCapacity>();
            foreach (var kvp in _records)
            {
                result[kvp.Key] = new ProviderCapacity
                {
                    ProviderName = kvp.Key,
                    State = GetState(kvp.Key),
                    IsAvailable = GetState(kvp.Key) == ProviderState.Available || GetState(kvp.Key) == ProviderState.Recovering
                };
            }
            return result;
        }
    }
}
