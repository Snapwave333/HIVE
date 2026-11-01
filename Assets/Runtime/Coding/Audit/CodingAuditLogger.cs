using System;
using System.IO;
using HIVE.Core;
using UnityEngine;

namespace HIVE.Coding
{
    public class CodingAuditEventMessage : MessageBus.IMessage
    {
        public string MessageId { get; set; } = Guid.NewGuid().ToString("N");
        public DateTime Timestamp { get; set; } = DateTime.UtcNow;
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; } = "AUDIT";
        public string Content { get; set; }
        public string TaskId { get; set; }
        public string Provider { get; set; }
        public string Status { get; set; }
    }

    public class CodingAuditLogger
    {
        private readonly string _logFilePath;

        public CodingAuditLogger(string customLogPath = null)
        {
            _logFilePath = customLogPath ?? Path.Combine(Directory.GetCurrentDirectory(), ".hive", "coding_audit.log");
        }

        public void LogEvent(string taskId, string agent, string provider, string status, string detail)
        {
            var timestamp = DateTime.UtcNow.ToString("o");
            var line = $"[{timestamp}] [TX:{taskId}] [AGENT:{agent}] [PROVIDER:{provider}] [STATUS:{status}] {detail}";

            // 1. Emit to MessageBus
            try
            {
                MessageBus.Publish(new CodingAuditEventMessage
                {
                    SourceAgent = agent,
                    TaskId = taskId,
                    Provider = provider,
                    Status = status,
                    Content = detail
                });
            }
            catch { }

            // 2. Append-only file write
            try
            {
                var dir = Path.GetDirectoryName(_logFilePath);
                if (!string.IsNullOrEmpty(dir) && !Directory.Exists(dir))
                    Directory.CreateDirectory(dir);

                File.AppendAllText(_logFilePath, line + Environment.NewLine);
            }
            catch { }
        }
    }
}
