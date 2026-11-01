using System;
using System.Collections.Generic;
using System.IO;
using UnityEngine;

namespace HIVE.Coding
{
    [Serializable]
    public class PersistentTaskQueueData
    {
        public List<CodingTaskRequest> PendingRequests = new List<CodingTaskRequest>();
        public List<CodingTaskResult> CompletedResults = new List<CodingTaskResult>();
    }

    public class CodingTaskPersistence
    {
        private readonly string _filePath;

        public CodingTaskPersistence(string customPath = null)
        {
            _filePath = customPath ?? Path.Combine(Directory.GetCurrentDirectory(), ".hive", "coding_tasks.json");
        }

        public void Save(List<CodingTaskRequest> pending, List<CodingTaskResult> completed)
        {
            try
            {
                var dir = Path.GetDirectoryName(_filePath);
                if (!string.IsNullOrEmpty(dir) && !Directory.Exists(dir))
                    Directory.CreateDirectory(dir);

                var data = new PersistentTaskQueueData
                {
                    PendingRequests = pending ?? new List<CodingTaskRequest>(),
                    CompletedResults = completed ?? new List<CodingTaskResult>()
                };

                var json = JsonUtility.ToJson(data, true);
                File.WriteAllText(_filePath, json);
            }
            catch { }
        }

        public (List<CodingTaskRequest> pending, List<CodingTaskResult> completed) Load()
        {
            try
            {
                if (File.Exists(_filePath))
                {
                    var json = File.ReadAllText(_filePath);
                    var data = JsonUtility.FromJson<PersistentTaskQueueData>(json);
                    if (data != null)
                    {
                        return (data.PendingRequests ?? new List<CodingTaskRequest>(),
                                data.CompletedResults ?? new List<CodingTaskResult>());
                    }
                }
            }
            catch { }

            return (new List<CodingTaskRequest>(), new List<CodingTaskResult>());
        }
    }
}
