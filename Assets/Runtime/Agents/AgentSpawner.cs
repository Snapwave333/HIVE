using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using UnityEngine;
using HIVE.Core;
using HIVE.Persistence;

namespace HIVE.Agents
{
    /// <summary>
    /// Spawns and manages the 12 permanent agents from JSON definitions.
    /// </summary>
    public class AgentSpawner : MonoBehaviour
    {
        [Header("Configuration")]
        public string AgentsPath = "Assets/StreamingAssets/agents";
        public GameObject AgentWorkspacePrefab;
        public Transform WorkspaceContainer;

        private readonly Dictionary<string, AgentInstance> _agents = new();
        private Repository _repository;
        private MessageBus.IMessagePersister _persister;

        public IReadOnlyDictionary<string, AgentInstance> Agents => _agents;

        public async void Initialize(Repository repository)
        {
            _repository = repository;
            _persister = new SqlitePersister(_repository);
            MessageBus.Initialize(_persister);

            await LoadAgentDefinitions();
            SpawnAllAgents();
            SubscribeToBus();
        }

        private async System.Threading.Tasks.Task LoadAgentDefinitions()
        {
            var path = Path.Combine(Application.dataPath, AgentsPath.Replace("Assets/", ""));
            if (!Directory.Exists(path))
            {
                Debug.LogError($"[AgentSpawner] Agents path not found: {path}");
                return;
            }

            var files = Directory.GetFiles(path, "*.json");
            foreach (var file in files)
            {
                try
                {
                    var json = File.ReadAllText(file);
                    var def = JsonSerializer.Deserialize<AgentDefinition>(json, new JsonSerializerOptions
                    {
                        PropertyNameCaseInsensitive = true
                    });
                    if (def != null)
                        _agents[def.Codename] = new AgentInstance { Definition = def };
                }
                catch (Exception e)
                {
                    Debug.LogError($"[AgentSpawner] Failed to load {file}: {e.Message}");
                }
            }

            Debug.Log($"[AgentSpawner] Loaded {_agents.Count} agent definitions");
        }

        private void SpawnAllAgents()
        {
            foreach (var kvp in _agents)
            {
                SpawnAgent(kvp.Value);
            }
        }

        private void SpawnAgent(AgentInstance instance)
        {
            var def = instance.Definition;

            // Create workspace
            GameObject workspace = null;
            if (AgentWorkspacePrefab != null && WorkspaceContainer != null)
            {
                workspace = Instantiate(AgentWorkspacePrefab, WorkspaceContainer);
                workspace.name = $"Workspace_{def.Codename}";
                var wsComponent = workspace.GetComponent<AgentWorkspace>();
                if (wsComponent != null) wsComponent.Initialize(def);
            }

            instance.Workspace = workspace;
            instance.State = new AgentStateRecord
            {
                Status = "Available",
                CurrentTask = null,
                CurrentProject = null,
                WorkspaceState = new Dictionary<string, object>(),
                LastHeartbeat = DateTime.UtcNow
            };

            // Register with repository
            _repository.UpdateAgentState(def.Codename, instance.State);

            // Subscribe to agent-specific messages
            instance.Subscription = MessageBus.Subscribe<AgentMessage>(msg =>
            {
                if (msg.TargetAgent == def.Codename || msg.TargetAgent == null)
                    HandleAgentMessage(instance, msg);
            });

            Debug.Log($"[AgentSpawner] Spawned agent: {def.Codename} ({def.DisplayName})");
        }

        private void SubscribeToBus()
        {
            // Global subscriptions
            MessageBus.Subscribe<TaskAssignedMessage>(OnTaskAssigned);
            MessageBus.Subscribe<TaskCompletedMessage>(OnTaskCompleted);
            MessageBus.Subscribe<AgentStatusRequest>(OnStatusRequest);
        }

        private void HandleAgentMessage(AgentInstance instance, AgentMessage msg)
        {
            // Route to agent's behavior (implemented in AgentRuntime)
            instance.MessageQueue.Enqueue(msg);
        }

        private void OnTaskAssigned(TaskAssignedMessage msg)
        {
            if (_agents.TryGetValue(msg.Assignee, out var instance))
            {
                instance.State.Status = "Active";
                instance.State.CurrentTask = msg.TaskId;
                _repository.UpdateAgentState(msg.Assignee, instance.State);
            }
        }

        private void OnTaskCompleted(TaskCompletedMessage msg)
        {
            if (_agents.TryGetValue(msg.Assignee, out var instance))
            {
                instance.State.Status = "Available";
                instance.State.CurrentTask = null;
                _repository.UpdateAgentState(msg.Assignee, instance.State);
            }
        }

        private void OnStatusRequest(AgentStatusRequest req)
        {
            var response = new AgentStatusResponse
            {
                CorrelationId = req.CorrelationId,
                Success = true,
                Agents = _agents.Values.Select(a => new AgentStatus
                {
                    Codename = a.Definition.Codename,
                    DisplayName = a.Definition.DisplayName,
                    Role = a.Definition.Role,
                    Status = a.State.Status,
                    CurrentTask = a.State.CurrentTask,
                    CurrentProject = a.State.CurrentProject,
                    ModelTier = a.Definition.ModelTier
                }).ToList()
            };
            MessageBus.Publish(response);
        }

        private void Update()
        {
            // Heartbeat
            foreach (var instance in _agents.Values)
            {
                instance.State.LastHeartbeat = DateTime.UtcNow;
                _repository.UpdateAgentState(instance.Definition.Codename, instance.State);
            }
        }

        private void OnDestroy()
        {
            MessageBus.Shutdown();
        }
    }

    [Serializable]
    public class AgentDefinition
    {
        public string Codename { get; set; }
        public string DisplayName { get; set; }
        public string Role { get; set; }
        public string ModelTier { get; set; }
        public Personality Personality { get; set; }
        public List<string> Skills { get; set; }
        public List<string> Tools { get; set; }
        public Permissions Permissions { get; set; }
        public string MemoryIndex { get; set; }
        public string WorkspaceTheme { get; set; }
        public AvatarConfig AvatarConfig { get; set; }
    }

    [Serializable]
    public class Personality
    {
        public List<string> Traits { get; set; }
        public string CommunicationStyle { get; set; }
        public string DecisionMaking { get; set; }
    }

    [Serializable]
    public class Permissions
    {
        public List<string> Read { get; set; }
        public List<string> Write { get; set; }
        public List<string> Execute { get; set; }
        public List<string> ExternalComms { get; set; }
        public List<string> Financial { get; set; }
    }

    [Serializable]
    public class AvatarConfig
    {
        public string Silhouette { get; set; }
        public string PrimaryColor { get; set; }
        public string SecondaryColor { get; set; }
        public string AnimationSet { get; set; }
    }

    public class AgentInstance
    {
        public AgentDefinition Definition { get; set; }
        public GameObject Workspace { get; set; }
        public AgentStateRecord State { get; set; }
        public IDisposable Subscription { get; set; }
        public Queue<AgentMessage> MessageQueue { get; } = new Queue<AgentMessage>();
    }

    // ========== Message Types ==========

    public class AgentMessage : MessageBus.IMessage
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string Content { get; set; }
        public Dictionary<string, object> Payload { get; set; }
    }

    public class TaskAssignedMessage : MessageBus.IMessage
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string TaskId { get; set; }
        public string Assignee { get; set; }
        public string ProjectId { get; set; }
    }

    public class TaskCompletedMessage : MessageBus.IMessage
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string TaskId { get; set; }
        public string Assignee { get; set; }
        public bool Verified { get; set; }
    }

    public class AgentStatusRequest : MessageBus.IRequest<AgentStatusResponse>
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string CorrelationId { get; set; }
    }

    public class AgentStatusResponse : MessageBus.IResponse
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string CorrelationId { get; set; }
        public bool Success { get; set; }
        public string Error { get; set; }
        public List<AgentStatus> Agents { get; set; }
    }

    public class AgentStatus
    {
        public string Codename { get; set; }
        public string DisplayName { get; set; }
        public string Role { get; set; }
        public string Status { get; set; }
        public string CurrentTask { get; set; }
        public string CurrentProject { get; set; }
        public string ModelTier { get; set; }
    }

    // SQLite persister for MessageBus
    internal class SqlitePersister : MessageBus.IMessagePersister
    {
        private readonly Repository _repo;
        public SqlitePersister(Repository repo) => _repo = repo;

        public Task PersistAsync(IReadOnlyList<MessageBus.Envelope> messages)
        {
            foreach (var env in messages)
            {
                // Store in audit_log or dedicated messages table
                // For now, just log
                Debug.Log($"[Persist] {env.Type}: {env.Json.Substring(0, Math.Min(100, env.Json.Length))}");
            }
            return Task.CompletedTask;
        }
    }
}