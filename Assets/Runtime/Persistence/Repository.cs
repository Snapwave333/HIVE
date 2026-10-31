using System;
using System.Collections.Generic;
using System.Data;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using UnityEngine;

namespace HIVE.Persistence
{
    /// <summary>
    /// SQLite-backed repository for tasks, messages, decisions, audit log.
    /// Auto-migrates schema on startup.
    /// </summary>
    public class Repository : IDisposable
    {
        private readonly string _dbPath;
        private IDbConnection _conn;
        private readonly object _lock = new();

        public Repository(string dbPath = null)
        {
            _dbPath = dbPath ?? Path.Combine(Application.persistentDataPath, "hive.db");
            Initialize();
        }

        private void Initialize()
        {
            var dir = Path.GetDirectoryName(_dbPath);
            if (!Directory.Exists(dir)) Directory.CreateDirectory(dir);

            _conn = new Mono.Data.Sqlite.SqliteConnection($"Data Source={_dbPath};Version=3;");
            _conn.Open();

            Migrate();
        }

        private void Migrate()
        {
            var migrations = new[]
            {
                @"CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY);",
                @"CREATE TABLE IF NOT EXISTS tasks (
                    id TEXT PRIMARY KEY,
                    project_id TEXT,
                    title TEXT,
                    description TEXT,
                    state TEXT,
                    priority INTEGER,
                    assignee TEXT,
                    dependencies TEXT, -- JSON array
                    created_at TEXT,
                    updated_at TEXT,
                    completed_at TEXT,
                    verified_at TEXT,
                    deliverables TEXT, -- JSON
                    acceptance_criteria TEXT
                );",
                @"CREATE INDEX IF NOT EXISTS idx_tasks_assignee ON tasks(assignee);",
                @"CREATE INDEX IF NOT EXISTS idx_tasks_state ON tasks(state);",
                @"CREATE INDEX IF NOT EXISTS idx_tasks_project ON tasks(project_id);",

                @"CREATE TABLE IF NOT EXISTS messages (
                    id TEXT PRIMARY KEY,
                    conversation_id TEXT,
                    sender TEXT,
                    recipient TEXT, -- null = channel
                    content TEXT,
                    attachments TEXT, -- JSON
                    references TEXT, -- JSON: tasks, projects, decisions
                    created_at TEXT
                );",
                @"CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id);",
                @"CREATE INDEX IF NOT EXISTS idx_messages_sender ON messages(sender);",

                @"CREATE TABLE IF NOT EXISTS decisions (
                    id TEXT PRIMARY KEY,
                    proposal TEXT,
                    status TEXT, -- proposed, debating, decided, implemented
                    proposer TEXT,
                    participants TEXT, -- JSON
                    arguments_for TEXT, -- JSON
                    arguments_against TEXT, -- JSON
                    evidence TEXT, -- JSON
                    decision TEXT,
                    authority TEXT,
                    minority_opinion TEXT,
                    created_at TEXT,
                    decided_at TEXT
                );",

                @"CREATE TABLE IF NOT EXISTS audit_log (
                    id TEXT PRIMARY KEY,
                    event_type TEXT,
                    actor TEXT,
                    target TEXT,
                    action TEXT,
                    details TEXT, -- JSON
                    ip_address TEXT,
                    user_agent TEXT,
                    created_at TEXT
                );",
                @"CREATE INDEX IF NOT EXISTS idx_audit_actor ON audit_log(actor);",
                @"CREATE INDEX IF NOT EXISTS idx_audit_type ON audit_log(event_type);",
                @"CREATE INDEX IF NOT EXISTS idx_audit_created ON audit_log(created_at);",

                @"CREATE TABLE IF NOT EXISTS agent_state (
                    agent_id TEXT PRIMARY KEY,
                    status TEXT,
                    current_task TEXT,
                    current_project TEXT,
                    workspace_state TEXT, -- JSON
                    last_heartbeat TEXT
                );",

                @"CREATE TABLE IF NOT EXISTS memories (
                    id TEXT PRIMARY KEY,
                    agent_id TEXT, -- null = shared
                    type TEXT, -- fact, lesson, experience, decision_ref
                    content TEXT,
                    tags TEXT, -- JSON
                    source TEXT,
                    confidence REAL,
                    created_at TEXT,
                    expires_at TEXT
                );",
                @"CREATE INDEX IF NOT EXISTS idx_memories_agent ON memories(agent_id);",
                @"CREATE INDEX IF NOT EXISTS idx_memories_type ON memories(type);",
                @"CREATE INDEX IF NOT EXISTS idx_memories_tags ON memories(tags);",

                @"CREATE TABLE IF NOT EXISTS relationships (
                    id TEXT PRIMARY KEY,
                    source_agent TEXT,
                    target_agent TEXT,
                    type TEXT, -- collaborated, handed_off, disagreed, mentored
                    weight REAL,
                    context TEXT, -- JSON
                    created_at TEXT,
                    last_reinforced TEXT
                );",
                @"CREATE INDEX IF NOT EXISTS idx_rel_source ON relationships(source_agent);",
                @"CREATE INDEX IF NOT EXISTS idx_rel_target ON relationships(target_agent);"
            };

            lock (_lock)
            {
                using (var cmd = _conn.CreateCommand())
                {
                    // Check current version
                    cmd.CommandText = "SELECT COALESCE(MAX(version), 0) FROM schema_version;";
                    var currentVersion = Convert.ToInt32(cmd.ExecuteScalar() ?? 0);

                    // Apply pending migrations
                    for (int i = currentVersion; i < migrations.Length; i++)
                    {
                        cmd.CommandText = migrations[i];
                        cmd.ExecuteNonQuery();
                        cmd.CommandText = $"INSERT INTO schema_version (version) VALUES ({i + 1});";
                        cmd.ExecuteNonQuery();
                    }
                }
            }
        }

        // ========== Tasks ==========

        public void SaveTask(TaskRecord task)
        {
            lock (_lock)
            using (var cmd = _conn.CreateCommand())
            {
                cmd.CommandText = @"
                    INSERT OR REPLACE INTO tasks
                    (id, project_id, title, description, state, priority, assignee, dependencies,
                     created_at, updated_at, completed_at, verified_at, deliverables, acceptance_criteria)
                    VALUES
                    (@id, @project, @title, @desc, @state, @priority, @assignee, @deps,
                     @created, @updated, @completed, @verified, @deliverables, @criteria);";

                AddParam(cmd, "@id", task.Id);
                AddParam(cmd, "@project", task.ProjectId);
                AddParam(cmd, "@title", task.Title);
                AddParam(cmd, "@desc", task.Description);
                AddParam(cmd, "@state", task.State);
                AddParam(cmd, "@priority", task.Priority);
                AddParam(cmd, "@assignee", task.Assignee);
                AddParam(cmd, "@deps", JsonUtility.ToJson(task.Dependencies));
                AddParam(cmd, "@created", task.CreatedAt.ToString("o"));
                AddParam(cmd, "@updated", task.UpdatedAt.ToString("o"));
                AddParam(cmd, "@completed", task.CompletedAt?.ToString("o") ?? "");
                AddParam(cmd, "@verified", task.VerifiedAt?.ToString("o") ?? "");
                AddParam(cmd, "@deliverables", JsonUtility.ToJson(task.Deliverables));
                AddParam(cmd, "@criteria", task.AcceptanceCriteria);

                cmd.ExecuteNonQuery();
            }
        }

        public TaskRecord GetTask(string id)
        {
            lock (_lock)
            using (var cmd = _conn.CreateCommand())
            {
                cmd.CommandText = "SELECT * FROM tasks WHERE id = @id;";
                AddParam(cmd, "@id", id);
                using (var reader = cmd.ExecuteReader())
                {
                    if (reader.Read()) return ReadTask(reader);
                    return null;
                }
            }
        }

        public List<TaskRecord> GetTasks(string assignee = null, string state = null, string projectId = null)
        {
            var results = new List<TaskRecord>();
            lock (_lock)
            using (var cmd = _conn.CreateCommand())
            {
                var where = new List<string>();
                if (assignee != null) { where.Add("assignee = @assignee"); AddParam(cmd, "@assignee", assignee); }
                if (state != null) { where.Add("state = @state"); AddParam(cmd, "@state", state); }
                if (projectId != null) { where.Add("project_id = @project"); AddParam(cmd, "@project", projectId); }

                cmd.CommandText = "SELECT * FROM tasks" + (where.Count > 0 ? " WHERE " + string.Join(" AND ", where) : "") + " ORDER BY priority DESC, created_at ASC;";
                using (var reader = cmd.ExecuteReader())
                    while (reader.Read()) results.Add(ReadTask(reader));
            }
            return results;
        }

        private TaskRecord ReadTask(IDataReader r) => new TaskRecord
        {
            Id = r["id"].ToString(),
            ProjectId = r["project_id"].ToString(),
            Title = r["title"].ToString(),
            Description = r["description"].ToString(),
            State = r["state"].ToString(),
            Priority = Convert.ToInt32(r["priority"]),
            Assignee = r["assignee"].ToString(),
            Dependencies = JsonUtility.FromJson<List<string>>(r["dependencies"].ToString()),
            CreatedAt = DateTime.Parse(r["created_at"].ToString()),
            UpdatedAt = DateTime.Parse(r["updated_at"].ToString()),
            CompletedAt = string.IsNullOrEmpty(r["completed_at"].ToString()) ? null : DateTime.Parse(r["completed_at"].ToString()),
            VerifiedAt = string.IsNullOrEmpty(r["verified_at"].ToString()) ? null : DateTime.Parse(r["verified_at"].ToString()),
            Deliverables = JsonUtility.FromJson<List<string>>(r["deliverables"].ToString()),
            AcceptanceCriteria = r["acceptance_criteria"].ToString()
        };

        // ========== Messages ==========

        public void SaveMessage(MessageRecord msg)
        {
            lock (_lock)
            using (var cmd = _conn.CreateCommand())
            {
                cmd.CommandText = @"
                    INSERT OR REPLACE INTO messages
                    (id, conversation_id, sender, recipient, content, attachments, references, created_at)
                    VALUES (@id, @conv, @sender, @recip, @content, @attach, @refs, @created);";

                AddParam(cmd, "@id", msg.Id);
                AddParam(cmd, "@conv", msg.ConversationId);
                AddParam(cmd, "@sender", msg.Sender);
                AddParam(cmd, "@recip", msg.Recipient);
                AddParam(cmd, "@content", msg.Content);
                AddParam(cmd, "@attach", JsonUtility.ToJson(msg.Attachments));
                AddParam(cmd, "@refs", JsonUtility.ToJson(msg.References));
                AddParam(cmd, "@created", msg.CreatedAt.ToString("o"));

                cmd.ExecuteNonQuery();
            }
        }

        public List<MessageRecord> GetMessages(string conversationId, int limit = 100, string beforeId = null)
        {
            var results = new List<MessageRecord>();
            lock (_lock)
            using (var cmd = _conn.CreateCommand())
            {
                cmd.CommandText = "SELECT * FROM messages WHERE conversation_id = @conv" +
                    (beforeId != null ? " AND id < @before" : "") +
                    " ORDER BY created_at DESC LIMIT @limit;";
                AddParam(cmd, "@conv", conversationId);
                if (beforeId != null) AddParam(cmd, "@before", beforeId);
                AddParam(cmd, "@limit", limit);
                using (var reader = cmd.ExecuteReader())
                    while (reader.Read()) results.Add(ReadMessage(reader));
            }
            results.Reverse();
            return results;
        }

        private MessageRecord ReadMessage(IDataReader r) => new MessageRecord
        {
            Id = r["id"].ToString(),
            ConversationId = r["conversation_id"].ToString(),
            Sender = r["sender"].ToString(),
            Recipient = r["recipient"] as string,
            Content = r["content"].ToString(),
            Attachments = JsonUtility.FromJson<List<string>>(r["attachments"].ToString()),
            References = JsonUtility.FromJson<MessageReferences>(r["references"].ToString()),
            CreatedAt = DateTime.Parse(r["created_at"].ToString())
        };

        // ========== Decisions ==========

        public void SaveDecision(DecisionRecord d)
        {
            lock (_lock)
            using (var cmd = _conn.CreateCommand())
            {
                cmd.CommandText = @"
                    INSERT OR REPLACE INTO decisions
                    (id, proposal, status, proposer, participants, arguments_for, arguments_against,
                     evidence, decision, authority, minority_opinion, created_at, decided_at)
                    VALUES (@id, @prop, @status, @proposer, @parts, @for, @against, @evid, @dec, @auth, @minor, @created, @decided);";

                AddParam(cmd, "@id", d.Id);
                AddParam(cmd, "@prop", d.Proposal);
                AddParam(cmd, "@status", d.Status);
                AddParam(cmd, "@proposer", d.Proposer);
                AddParam(cmd, "@parts", JsonUtility.ToJson(d.Participants));
                AddParam(cmd, "@for", JsonUtility.ToJson(d.ArgumentsFor));
                AddParam(cmd, "@against", JsonUtility.ToJson(d.ArgumentsAgainst));
                AddParam(cmd, "@evid", JsonUtility.ToJson(d.Evidence));
                AddParam(cmd, "@dec", d.Decision);
                AddParam(cmd, "@auth", d.Authority);
                AddParam(cmd, "@minor", d.MinorityOpinion);
                AddParam(cmd, "@created", d.CreatedAt.ToString("o"));
                AddParam(cmd, "@decided", d.DecidedAt?.ToString("o") ?? "");

                cmd.ExecuteNonQuery();
            }
        }

        // ========== Audit ==========

        public void WriteAudit(AuditEntry entry)
        {
            lock (_lock)
            using (var cmd = _conn.CreateCommand())
            {
                cmd.CommandText = @"
                    INSERT INTO audit_log (id, event_type, actor, target, action, details, ip_address, user_agent, created_at)
                    VALUES (@id, @type, @actor, @target, @action, @details, @ip, @ua, @created);";

                AddParam(cmd, "@id", entry.Id);
                AddParam(cmd, "@type", entry.EventType);
                AddParam(cmd, "@actor", entry.Actor);
                AddParam(cmd, "@target", entry.Target);
                AddParam(cmd, "@action", entry.Action);
                AddParam(cmd, "@details", JsonUtility.ToJson(entry.Details));
                AddParam(cmd, "@ip", entry.IpAddress);
                AddParam(cmd, "@ua", entry.UserAgent);
                AddParam(cmd, "@created", entry.CreatedAt.ToString("o"));

                cmd.ExecuteNonQuery();
            }
        }

        public List<AuditEntry> QueryAudit(string actor = null, string type = null, DateTime? since = null, int limit = 100)
        {
            var results = new List<AuditEntry>();
            lock (_lock)
            using (var cmd = _conn.CreateCommand())
            {
                var where = new List<string>();
                if (actor != null) { where.Add("actor = @actor"); AddParam(cmd, "@actor", actor); }
                if (type != null) { where.Add("event_type = @type"); AddParam(cmd, "@type", type); }
                if (since.HasValue) { where.Add("created_at >= @since"); AddParam(cmd, "@since", since.Value.ToString("o")); }

                cmd.CommandText = "SELECT * FROM audit_log" + (where.Count > 0 ? " WHERE " + string.Join(" AND ", where) : "") + " ORDER BY created_at DESC LIMIT @limit;";
                AddParam(cmd, "@limit", limit);
                using (var reader = cmd.ExecuteReader())
                    while (reader.Read()) results.Add(ReadAudit(reader));
            }
            return results;
        }

        private AuditEntry ReadAudit(IDataReader r) => new AuditEntry
        {
            Id = r["id"].ToString(),
            EventType = r["event_type"].ToString(),
            Actor = r["actor"].ToString(),
            Target = r["target"].ToString(),
            Action = r["action"].ToString(),
            Details = JsonUtility.FromJson<Dictionary<string, object>>(r["details"].ToString()),
            IpAddress = r["ip_address"].ToString(),
            UserAgent = r["user_agent"].ToString(),
            CreatedAt = DateTime.Parse(r["created_at"].ToString())
        };

        // ========== Agent State ==========

        public void UpdateAgentState(string agentId, AgentStateRecord state)
        {
            lock (_lock)
            using (var cmd = _conn.CreateCommand())
            {
                cmd.CommandText = @"
                    INSERT OR REPLACE INTO agent_state
                    (agent_id, status, current_task, current_project, workspace_state, last_heartbeat)
                    VALUES (@id, @status, @task, @project, @ws, @hb);";

                AddParam(cmd, "@id", agentId);
                AddParam(cmd, "@status", state.Status);
                AddParam(cmd, "@task", state.CurrentTask);
                AddParam(cmd, "@project", state.CurrentProject);
                AddParam(cmd, "@ws", JsonUtility.ToJson(state.WorkspaceState));
                AddParam(cmd, "@hb", state.LastHeartbeat.ToString("o"));

                cmd.ExecuteNonQuery();
            }
        }

        // ========== Memories ==========

        public void SaveMemory(MemoryRecord mem)
        {
            lock (_lock)
            using (var cmd = _conn.CreateCommand())
            {
                cmd.CommandText = @"
                    INSERT OR REPLACE INTO memories
                    (id, agent_id, type, content, tags, source, confidence, created_at, expires_at)
                    VALUES (@id, @agent, @type, @content, @tags, @source, @conf, @created, @expires);";

                AddParam(cmd, "@id", mem.Id);
                AddParam(cmd, "@agent", mem.AgentId);
                AddParam(cmd, "@type", mem.Type);
                AddParam(cmd, "@content", mem.Content);
                AddParam(cmd, "@tags", JsonUtility.ToJson(mem.Tags));
                AddParam(cmd, "@source", mem.Source);
                AddParam(cmd, "@conf", mem.Confidence);
                AddParam(cmd, "@created", mem.CreatedAt.ToString("o"));
                AddParam(cmd, "@expires", mem.ExpiresAt?.ToString("o") ?? "");

                cmd.ExecuteNonQuery();
            }
        }

        public List<MemoryRecord> SearchMemories(string agentId = null, string type = null, string[] tags = null, string query = null, int limit = 50)
        {
            var results = new List<MemoryRecord>();
            lock (_lock)
            using (var cmd = _conn.CreateCommand())
            {
                var where = new List<string>();
                if (agentId != null) { where.Add("agent_id = @agent"); AddParam(cmd, "@agent", agentId); }
                if (type != null) { where.Add("type = @type"); AddParam(cmd, "@type", type); }
                if (tags != null && tags.Length > 0)
                {
                    // Simple tag matching - in production use FTS or JSON1 extension
                    where.Add("(tags LIKE @tag0" + string.Join("", tags.Skip(1).Select((_, i) => " OR tags LIKE @tag" + i)) + ")");
                    for (int i = 0; i < tags.Length; i++) AddParam(cmd, "@tag" + i, "%" + tags[i] + "%");
                }
                if (query != null) { where.Add("content LIKE @query"); AddParam(cmd, "@query", "%" + query + "%"); }

                cmd.CommandText = "SELECT * FROM memories" + (where.Count > 0 ? " WHERE " + string.Join(" AND ", where) : "") + " ORDER BY created_at DESC LIMIT @limit;";
                AddParam(cmd, "@limit", limit);
                using (var reader = cmd.ExecuteReader())
                    while (reader.Read()) results.Add(ReadMemory(reader));
            }
            return results;
        }

        private MemoryRecord ReadMemory(IDataReader r) => new MemoryRecord
        {
            Id = r["id"].ToString(),
            AgentId = r["agent_id"] as string,
            Type = r["type"].ToString(),
            Content = r["content"].ToString(),
            Tags = JsonUtility.FromJson<List<string>>(r["tags"].ToString()),
            Source = r["source"].ToString(),
            Confidence = Convert.ToSingle(r["confidence"]),
            CreatedAt = DateTime.Parse(r["created_at"].ToString()),
            ExpiresAt = string.IsNullOrEmpty(r["expires_at"].ToString()) ? null : DateTime.Parse(r["expires_at"].ToString())
        };

        // ========== Relationships ==========

        public void SaveRelationship(RelationshipRecord rel)
        {
            lock (_lock)
            using (var cmd = _conn.CreateCommand())
            {
                cmd.CommandText = @"
                    INSERT OR REPLACE INTO relationships
                    (id, source_agent, target_agent, type, weight, context, created_at, last_reinforced)
                    VALUES (@id, @src, @tgt, @type, @weight, @ctx, @created, @reinforced);";

                AddParam(cmd, "@id", rel.Id);
                AddParam(cmd, "@src", rel.SourceAgent);
                AddParam(cmd, "@tgt", rel.TargetAgent);
                AddParam(cmd, "@type", rel.Type);
                AddParam(cmd, "@weight", rel.Weight);
                AddParam(cmd, "@ctx", JsonUtility.ToJson(rel.Context));
                AddParam(cmd, "@created", rel.CreatedAt.ToString("o"));
                AddParam(cmd, "@reinforced", rel.LastReinforced.ToString("o"));

                cmd.ExecuteNonQuery();
            }
        }

        public List<RelationshipRecord> GetRelationships(string agentId = null)
        {
            var results = new List<RelationshipRecord>();
            lock (_lock)
            using (var cmd = _conn.CreateCommand())
            {
                cmd.CommandText = agentId != null
                    ? "SELECT * FROM relationships WHERE source_agent = @id OR target_agent = @id;"
                    : "SELECT * FROM relationships;";
                if (agentId != null) AddParam(cmd, "@id", agentId);
                using (var reader = cmd.ExecuteReader())
                    while (reader.Read()) results.Add(ReadRelationship(reader));
            }
            return results;
        }

        private RelationshipRecord ReadRelationship(IDataReader r) => new RelationshipRecord
        {
            Id = r["id"].ToString(),
            SourceAgent = r["source_agent"].ToString(),
            TargetAgent = r["target_agent"].ToString(),
            Type = r["type"].ToString(),
            Weight = Convert.ToSingle(r["weight"]),
            Context = JsonUtility.FromJson<Dictionary<string, object>>(r["context"].ToString()),
            CreatedAt = DateTime.Parse(r["created_at"].ToString()),
            LastReinforced = DateTime.Parse(r["last_reinforced"].ToString())
        };

        private void AddParam(IDbCommand cmd, string name, object value)
        {
            var p = cmd.CreateParameter();
            p.ParameterName = name;
            p.Value = value ?? DBNull.Value;
            cmd.Parameters.Add(p);
        }

        public void Dispose()
        {
            _conn?.Close();
            _conn?.Dispose();
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