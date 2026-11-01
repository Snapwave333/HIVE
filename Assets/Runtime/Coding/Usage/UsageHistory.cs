using System;
using System.Collections.Generic;
using System.IO;
using UnityEngine;

namespace HIVE.Coding
{
    [Serializable]
    public class UsageEntry
    {
        public string Agent;
        public string Provider;
        public string TaskId;
        public UsageTelemetry Telemetry;
        public string Timestamp;
    }

    [Serializable]
    public class UsageHistoryStore
    {
        public List<UsageEntry> Entries = new List<UsageEntry>();
    }

    public class UsageHistory
    {
        private readonly string _filePath;
        private readonly UsageHistoryStore _store = new UsageHistoryStore();

        public UsageHistory(string customPath = null)
        {
            _filePath = customPath ?? Path.Combine(Directory.GetCurrentDirectory(), ".hive", "usage_history.json");
            Load();
        }

        public void Append(string agent, string provider, string taskId, UsageTelemetry telemetry)
        {
            var entry = new UsageEntry
            {
                Agent = agent,
                Provider = provider,
                TaskId = taskId,
                Telemetry = telemetry,
                Timestamp = DateTime.UtcNow.ToString("o")
            };

            lock (_store)
            {
                _store.Entries.Add(entry);
                Save();
            }
        }

        public IReadOnlyList<UsageEntry> GetAll()
        {
            lock (_store)
            {
                return new List<UsageEntry>(_store.Entries);
            }
        }

        private void Load()
        {
            try
            {
                if (File.Exists(_filePath))
                {
                    var json = File.ReadAllText(_filePath);
                    var loaded = JsonUtility.FromJson<UsageHistoryStore>(json);
                    if (loaded != null && loaded.Entries != null)
                    {
                        _store.Entries = loaded.Entries;
                    }
                }
            }
            catch { }
        }

        private void Save()
        {
            try
            {
                var dir = Path.GetDirectoryName(_filePath);
                if (!string.IsNullOrEmpty(dir) && !Directory.Exists(dir))
                    Directory.CreateDirectory(dir);

                var json = JsonUtility.ToJson(_store, true);
                File.WriteAllText(_filePath, json);
            }
            catch { }
        }
    }
}
