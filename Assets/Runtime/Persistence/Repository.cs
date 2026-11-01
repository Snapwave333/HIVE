using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using UnityEngine;

namespace HIVE.Persistence
{
    /// <summary>
    /// Resilient, cross-platform persistence repository for tasks, messages, decisions, audit logs, and memory.
    /// Thread-safe in-memory cache backed by durable JSON storage.
    /// </summary>
    public class Repository : IDisposable
    {
        private readonly string _dbPath;
        private readonly object _lock = new();

        private readonly ConcurrentDictionary<string, TaskRecord> _tasks = new();
        private readonly ConcurrentDictionary<string, MessageRecord> _messages = new();
        private readonly ConcurrentDictionary<string, DecisionRecord> _decisions = new();
        private readonly List<AuditEntry> _auditLog = new();
        private readonly ConcurrentDictionary<string, AgentStateRecord> _agentStates = new();
        private readonly ConcurrentDictionary<string, MemoryRecord> _memories = new();
        private readonly ConcurrentDictionary<string, RelationshipRecord> _relationships = new();

        public Repository(string dbPath = null)
        {
            _dbPath = dbPath ?? Path.Combine(Application.persistentDataPath, "hive_store.json");
            Initialize();
        }

        private void Initialize()
        {
            var dir = Path.GetDirectoryName(_dbPath);
            if (!string.IsNullOrEmpty(dir) && !Directory.Exists(dir))
                Directory.CreateDirectory(dir);

            if (File.Exists(_dbPath))
                LoadFromDisk();
            else
                SaveToDisk();
        }

        private void LoadFromDisk()
        {
            lock (_lock)
            {
                if (!File.Exists(_dbPath)) return;
                try
                {
                    var json = File.ReadAllText(_dbPath);
                    var payload = JsonSerializer.Deserialize<DataPayload>(json);
                    if (payload != null)
                    {
                        if (payload.Tasks != null)
                            foreach (var t in payload.Tasks) _tasks[t.Id] = t;
                        if (payload.Messages != null)
                            foreach (var m in payload.Messages) _messages[m.Id] = m;
                        if (payload.Decisions != null)
                            foreach (var d in payload.Decisions) _decisions[d.Id] = d;
                        if (payload.AuditLog != null)
                            _auditLog.AddRange(payload.AuditLog);
                        if (payload.AgentStates != null)
                            foreach (var kvp in payload.AgentStates) _agentStates[kvp.Key] = kvp.Value;
                        if (payload.Memories != null)
                            foreach (var m in payload.Memories) _memories[m.Id] = m;
                        if (payload.Relationships != null)
                            foreach (var r in payload.Relationships) _relationships[r.Id] = r;
                    }
                }
                catch (Exception e)
                {
                    Debug.LogWarning($"[Repository] Failed to read store from {_dbPath}: {e.Message}");
                }
            }
        }

        private void SaveToDisk()
        {
            lock (_lock)
            {
                try
                {
                    var payload = new DataPayload
                    {
                        Tasks = _tasks.Values.ToList(),
                        Messages = _messages.Values.ToList(),
                        Decisions = _decisions.Values.ToList(),
                        AuditLog = _auditLog.ToList(),
                        AgentStates = _agentStates.ToDictionary(k => k.Key, v => v.Value),
                        Memories = _memories.Values.ToList(),
                        Relationships = _relationships.Values.ToList()
                    };
                    var json = JsonSerializer.Serialize(payload, new JsonSerializerOptions { WriteIndented = true });
                    File.WriteAllText(_dbPath, json);
                }
                catch (Exception e)
                {
                    Debug.LogWarning($"[Repository] Failed to write store to {_dbPath}: {e.Message}");
                }
            }
        }

        // ========== Tasks ==========

        public void SaveTask(TaskRecord task)
        {
            if (task == null) return;
            task.Id = task.Id ?? Guid.NewGuid().ToString();
            _tasks[task.Id] = task;
            SaveToDisk();
        }

        public TaskRecord GetTask(string id)
        {
            if (id == null) return null;
            _tasks.TryGetValue(id, out var task);
            return task;
        }

        public List<TaskRecord> GetTasks(string assignee = null, string state = null, string projectId = null)
        {
            var query = _tasks.Values.AsEnumerable();
            if (!string.IsNullOrEmpty(assignee)) query = query.Where(t => t.Assignee == assignee);
            if (!string.IsNullOrEmpty(state)) query = query.Where(t => t.State == state);
            if (!string.IsNullOrEmpty(projectId)) query = query.Where(t => t.ProjectId == projectId);
            return query.OrderByDescending(t => t.Priority).ThenByDescending(t => t.CreatedAt).ToList();
        }

        // ========== Messages ==========

        public void SaveMessage(MessageRecord msg)
        {
            if (msg == null) return;
            msg.Id = msg.Id ?? Guid.NewGuid().ToString();
            _messages[msg.Id] = msg;
            SaveToDisk();
        }

        public List<MessageRecord> GetMessages(string conversationId, int limit = 100, string beforeId = null)
        {
            var query = _messages.Values.Where(m => m.ConversationId == conversationId);
            return query.OrderBy(m => m.CreatedAt).Take(limit).ToList();
        }

        // ========== Decisions ==========

        public void SaveDecision(DecisionRecord d)
        {
            if (d == null) return;
            d.Id = d.Id ?? Guid.NewGuid().ToString();
            _decisions[d.Id] = d;
            SaveToDisk();
        }

        // ========== Audit Log ==========

        public void WriteAudit(AuditEntry entry)
        {
            if (entry == null) return;
            lock (_lock)
            {
                _auditLog.Add(entry);
                if (_auditLog.Count > 5000)
                    _auditLog.RemoveRange(0, 1000);
            }
            SaveToDisk();
        }

        public List<AuditEntry> QueryAudit(string actor = null, string type = null, DateTime? since = null, int limit = 100)
        {
            lock (_lock)
            {
                var query = _auditLog.AsEnumerable();
                if (!string.IsNullOrEmpty(actor)) query = query.Where(a => a.Actor == actor);
                if (!string.IsNullOrEmpty(type)) query = query.Where(a => a.EventType == type);
                if (since.HasValue) query = query.Where(a => a.CreatedAt >= since.Value);
                return query.OrderByDescending(a => a.CreatedAt).Take(limit).ToList();
            }
        }

        // ========== Agent States ==========

        public void UpdateAgentState(string agentId, AgentStateRecord state)
        {
            if (string.IsNullOrEmpty(agentId) || state == null) return;
            _agentStates[agentId] = state;
            SaveToDisk();
        }

        // ========== Memory ==========

        public void SaveMemory(MemoryRecord mem)
        {
            if (mem == null) return;
            mem.Id = mem.Id ?? Guid.NewGuid().ToString();
            _memories[mem.Id] = mem;
            SaveToDisk();
        }

        public List<MemoryRecord> SearchMemories(string agentId = null, string type = null, string[] tags = null, string query = null, int limit = 50)
        {
            var q = _memories.Values.AsEnumerable();
            if (!string.IsNullOrEmpty(agentId)) q = q.Where(m => m.AgentId == agentId);
            if (!string.IsNullOrEmpty(type)) q = q.Where(m => m.Type == type);
            if (tags != null && tags.Length > 0) q = q.Where(m => m.Tags != null && tags.Any(t => m.Tags.Contains(t)));
            if (!string.IsNullOrEmpty(query))
            {
                var lower = query.ToLowerInvariant();
                q = q.Where(m => (m.Content != null && m.Content.ToLowerInvariant().Contains(lower)) ||
                                 (m.Source != null && m.Source.ToLowerInvariant().Contains(lower)));
            }
            return q.OrderByDescending(m => m.CreatedAt).Take(limit).ToList();
        }

        // ========== Relationships ==========

        public void SaveRelationship(RelationshipRecord rel)
        {
            if (rel == null) return;
            rel.Id = rel.Id ?? Guid.NewGuid().ToString();
            _relationships[rel.Id] = rel;
            SaveToDisk();
        }

        public List<RelationshipRecord> GetRelationships(string agentId = null)
        {
            var q = _relationships.Values.AsEnumerable();
            if (!string.IsNullOrEmpty(agentId))
                q = q.Where(r => r.SourceAgent == agentId || r.TargetAgent == agentId);
            return q.OrderByDescending(r => r.Weight).ToList();
        }

        public void Dispose()
        {
            SaveToDisk();
        }

        private class DataPayload
        {
            public List<TaskRecord> Tasks { get; set; }
            public List<MessageRecord> Messages { get; set; }
            public List<DecisionRecord> Decisions { get; set; }
            public List<AuditEntry> AuditLog { get; set; }
            public Dictionary<string, AgentStateRecord> AgentStates { get; set; }
            public List<MemoryRecord> Memories { get; set; }
            public List<RelationshipRecord> Relationships { get; set; }
        }
    }

    // ========== Record Types ==========

    public class TaskRecord
    {
        public string Id { get; set; }
        public string ProjectId { get; set; }
        public string Title { get; set; }
        public string Description { get; set; }
        public string State { get; set; } // Draft, Ready, Queued, InProgress, Waiting, Blocked, AwaitingApproval, Review, Completed, Verified, Failed, Cancelled
        public int Priority { get; set; }
        public string Assignee { get; set; }
        public List<string> Dependencies { get; set; }
        public DateTime CreatedAt { get; set; }
        public DateTime UpdatedAt { get; set; }
        public DateTime? CompletedAt { get; set; }
        public DateTime? VerifiedAt { get; set; }
        public List<string> Deliverables { get; set; }
        public string AcceptanceCriteria { get; set; }
    }

    public class MessageRecord
    {
        public string Id { get; set; }
        public string ConversationId { get; set; }
        public string Sender { get; set; }
        public string Recipient { get; set; }
        public string Content { get; set; }
        public List<string> Attachments { get; set; }
        public MessageReferences References { get; set; }
        public DateTime CreatedAt { get; set; }
    }

    public class MessageReferences
    {
        public List<string> Tasks { get; set; }
        public List<string> Projects { get; set; }
        public List<string> Decisions { get; set; }
        public List<string> Deliverables { get; set; }
    }

    public class DecisionRecord
    {
        public string Id { get; set; }
        public string Proposal { get; set; }
        public string Status { get; set; }
        public string Proposer { get; set; }
        public List<string> Participants { get; set; }
        public List<string> ArgumentsFor { get; set; }
        public List<string> ArgumentsAgainst { get; set; }
        public List<string> Evidence { get; set; }
        public string Decision { get; set; }
        public string Authority { get; set; }
        public string MinorityOpinion { get; set; }
        public DateTime CreatedAt { get; set; }
        public DateTime? DecidedAt { get; set; }
    }

    public class AuditEntry
    {
        public string Id { get; set; } = Guid.NewGuid().ToString();
        public string EventType { get; set; }
        public string Actor { get; set; }
        public string Target { get; set; }
        public string Action { get; set; }
        public Dictionary<string, object> Details { get; set; }
        public string IpAddress { get; set; }
        public string UserAgent { get; set; }
        public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
    }

    public class AgentStateRecord
    {
        public string Status { get; set; }
        public string CurrentTask { get; set; }
        public string CurrentProject { get; set; }
        public Dictionary<string, object> WorkspaceState { get; set; }
        public DateTime LastHeartbeat { get; set; } = DateTime.UtcNow;
    }

    public class MemoryRecord
    {
        public string Id { get; set; } = Guid.NewGuid().ToString();
        public string AgentId { get; set; }
        public string Type { get; set; }
        public string Content { get; set; }
        public List<string> Tags { get; set; }
        public string Source { get; set; }
        public float Confidence { get; set; } = 1.0f;
        public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
        public DateTime? ExpiresAt { get; set; }
    }

    public class RelationshipRecord
    {
        public string Id { get; set; } = Guid.NewGuid().ToString();
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string Type { get; set; }
        public float Weight { get; set; } = 1.0f;
        public Dictionary<string, object> Context { get; set; }
        public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
        public DateTime LastReinforced { get; set; } = DateTime.UtcNow;
    }
}
