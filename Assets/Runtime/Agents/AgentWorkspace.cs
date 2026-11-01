using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using HIVE.Core;
using HIVE.LLM;
using HIVE.Tasks;
using HIVE.Memory;
using HIVE.Persistence;
using HIVE.UI;

namespace HIVE.Agents
{
    /// <summary>
    /// Individual agent workspace: avatar, status, task display, holographic surfaces, communication interface.
    /// </summary>
    public class AgentWorkspace : MonoBehaviour
    {
        [Header("Agent Definition")]
        public AgentDefinition Definition;

        [Header("UI")]
        public UIDocument UIDocument;
        public VisualTreeAsset WorkspaceTemplate;

        [Header("Visual")]
        public GameObject AvatarPrefab;
        public Material HologramMaterial;
        public Transform HologramAnchor;
        public Transform TaskDisplayAnchor;
        public Transform NetworkConnectionsAnchor;

        private VisualElement _root;
        private Label _nameLabel;
        private Label _roleLabel;
        private Label _statusLabel;
        private Label _currentTaskLabel;
        private ListView _taskList;
        private Button _chatButton;
        private VisualElement _hologramContainer;
        private VisualElement _networkConnections;

        private AgentInstance _instance;
        private GameObject _avatar;
        private readonly List<TaskRecord> _activeTasks = new();
        private Coroutine _statusUpdateCoroutine;

        public void Initialize(AgentDefinition def)
        {
            Definition = def;
            _instance = AgentSpawner.Instance?.Agents.GetValueOrDefault(def.Codename);

            SetupUI();
            SpawnAvatar();
            StartStatusUpdates();
        }

        private void SetupUI()
        {
            if (UIDocument == null) UIDocument = GetComponent<UIDocument>();
            if (UIDocument == null) return;
            _root = UIDocument.rootVisualElement;
            if (_root == null) return;

            if (WorkspaceTemplate != null)
                WorkspaceTemplate.CloneTree(_root);

            _nameLabel = _root.Q<Label>("agent-name");
            _roleLabel = _root.Q<Label>("agent-role");
            _statusLabel = _root.Q<Label>("agent-status");
            _currentTaskLabel = _root.Q<Label>("current-task");
            _taskList = _root.Q<ListView>("task-list");
            _chatButton = _root.Q<Button>("chat-button");
            _hologramContainer = _root.Q<VisualElement>("hologram-container");
            _networkConnections = _root.Q<VisualElement>("network-connections");

            if (_nameLabel != null) _nameLabel.text = Definition?.DisplayName ?? "";
            if (_roleLabel != null) _roleLabel.text = Definition?.Role ?? "";
            if (_statusLabel != null) _statusLabel.text = _instance?.State?.Status ?? "Initializing";

            if (_chatButton != null) _chatButton.clicked += OpenChat;

            if (_taskList != null)
            {
                _taskList.makeItem = () => new Label();
                _taskList.bindItem = (e, i) =>
                {
                    if (i < _activeTasks.Count)
                    {
                        var task = _activeTasks[i];
                        if (e is Label lbl)
                        {
                            lbl.text = $"[{task.State}] {task.Title}";
                            lbl.tooltip = task.Description;
                        }
                    }
                };
                _taskList.itemsSource = _activeTasks;
            }
        }

        private void SpawnAvatar()
        {
            if (AvatarPrefab != null)
            {
                _avatar = Instantiate(AvatarPrefab, transform);
                _avatar.name = $"Avatar_{Definition.Codename}";

                // Apply avatar config
                var renderer = _avatar.GetComponentInChildren<Renderer>();
                if (renderer != null && Definition.AvatarConfig != null)
                {
                    var primary = ColorUtility.TryParseHtmlString(Definition.AvatarConfig.PrimaryColor, out var c1) ? c1 : Color.green;
                    var secondary = ColorUtility.TryParseHtmlString(Definition.AvatarConfig.SecondaryColor, out var c2) ? c2 : Color.cyan;
                    renderer.material.SetColor("_PrimaryColor", primary);
                    renderer.material.SetColor("_SecondaryColor", secondary);
                }
            }
        }

        private void StartStatusUpdates()
        {
            _statusUpdateCoroutine = StartCoroutine(StatusUpdateLoop());
        }

        private System.Collections.IEnumerator StatusUpdateLoop()
        {
            while (true)
            {
                UpdateStatus();
                yield return new WaitForSeconds(2f);
            }
        }

        private void UpdateStatus()
        {
            if (_instance == null) return;

            if (_statusLabel != null) _statusLabel.text = _instance.State?.Status ?? "Available";
            if (_currentTaskLabel != null) _currentTaskLabel.text = _instance.State?.CurrentTask ?? "None";

            // Update task list from TaskManager
            if (TaskManager.Instance != null && _taskList != null)
            {
                var tasks = TaskManager.Instance.GetTasks(assignee: Definition.Codename);
                _activeTasks.Clear();
                _activeTasks.AddRange(tasks);
                _taskList.RefreshItems();
            }

            // Update hologram displays
            UpdateHolograms();

            // Update network connections
            UpdateNetworkConnections();
        }

        private void UpdateHolograms()
        {
            // TODO: Spawn/update holographic task displays, project visualizations
            // This would instantiate hologram prefabs at HologramAnchor
        }

        private void UpdateNetworkConnections()
        {
            // TODO: Show visual connections to collaborating agents
            // Query relationship graph from Repository
        }

        private void OpenChat()
        {
            if (CommandInput.Instance != null)
            {
                CommandInput.Instance.SetContext($"Agent:{Definition.Codename}");
                CommandInput.Instance.FocusInput();
            }
        }

        public void AssignTask(TaskRecord task)
        {
            _instance.State.Status = "Active";
            _instance.State.CurrentTask = task.Id;
            _activeTasks.Add(task);
            _taskList.RefreshItems();
        }

        public void CompleteTask(string taskId)
        {
            var task = _activeTasks.Find(t => t.Id == taskId);
            if (task != null)
            {
                task.State = "Completed";
                _taskList.RefreshItems();
            }
            _instance.State.Status = "Available";
            _instance.State.CurrentTask = null;
        }

        public void ShowHolographicTask(TaskRecord task)
        {
            // TODO: Spawn holographic task display at HologramAnchor
        }

        public void ShowHolographicProject(string projectId)
        {
            // TODO: Spawn holographic project visualization
        }

        private void OnDestroy()
        {
            if (_statusUpdateCoroutine != null) StopCoroutine(_statusUpdateCoroutine);
            if (_avatar != null) Destroy(_avatar);
        }
    }
}