using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.UIElements;
using HIVE.Core;

namespace HIVE.UI
{
    /// <summary>
    /// Persistent navigation sidebar with 12 sections, searchable command launcher.
    /// </summary>
    [RequireComponent(typeof(UIDocument))]
    public class NavSidebar : MonoBehaviour
    {
        [Header("UI")]
        public UIDocument UIDocument;
        public VisualTreeAsset SidebarTemplate;
        public VisualTreeAsset NavItemTemplate;
        public VisualTreeAsset SearchResultTemplate;

        [Header("Settings")]
        public KeyCode ToggleKey = KeyCode.Tab;
        public float SearchDebounce = 0.15f;

        private VisualElement _root;
        private VisualElement _sidebar;
        private TextField _searchField;
        private ListView _navList;
        private ListView _searchResults;
        private VisualElement _searchResultsContainer;
        private Button _toggleButton;
        private Label _agentCountLabel;
        private Label _taskCountLabel;
        private Label _alertCountLabel;

        private readonly List<NavSection> _sections = new();
        private readonly List<SearchResult> _searchCache = new();
        private float _lastSearchTime;
        private Coroutine _searchCoroutine;
        private bool _expanded = true;

        private void Awake()
        {
            _root = UIDocument.rootVisualElement;
            if (SidebarTemplate != null)
                SidebarTemplate.CloneTree(_root);

            _sidebar = _root.Q<VisualElement>("sidebar");
            _searchField = _root.Q<TextField>("search-field");
            _navList = _root.Q<ListView>("nav-list");
            _searchResults = _root.Q<ListView>("search-results");
            _searchResultsContainer = _root.Q<VisualElement>("search-results-container");
            _toggleButton = _root.Q<Button>("toggle-button");
            _agentCountLabel = _root.Q<Label>("agent-count");
            _taskCountLabel = _root.Q<Label>("task-count");
            _alertCountLabel = _root.Q<Label>("alert-count");

            SetupSections();
            SetupUI();
        }

        private void SetupSections()
        {
            _sections.AddRange(new[]
            {
                new NavSection("Command Center", "command-center", "🏠", NavAction.CommandCenter),
                new NavSection("Agents", "agents", "👥", NavAction.Agents),
                new NavSection("Projects", "projects", "📁", NavAction.Projects),
                new NavSection("Tasks", "tasks", "✅", NavAction.Tasks),
                new NavSection("Communications", "comms", "💬", NavAction.Communications),
                new NavSection("Knowledge Archive", "knowledge", "📚", NavAction.Knowledge),
                new NavSection("Infrastructure", "infrastructure", "⚙️", NavAction.Infrastructure),
                new NavSection("Skills & Tools", "skills", "🔧", NavAction.Skills),
                new NavSection("Approvals", "approvals", "✍️", NavAction.Approvals),
                new NavSection("Personal Dashboard", "dashboard", "📊", NavAction.Dashboard),
                new NavSection("Security & Audit", "security", "🔒", NavAction.Security),
                new NavSection("Settings", "settings", "⚙️", NavAction.Settings)
            });
        }

        private void SetupUI()
        {
            _searchField.RegisterValueChangedCallback(OnSearchChanged);
            _searchField.RegisterCallback<FocusInEvent>(_ => ShowSearchResults());
            _searchField.RegisterCallback<FocusOutEvent>(_ => HideSearchResultsDelayed());

            _navList.makeItem = () => NavItemTemplate.CloneTree();
            _navList.bindItem = (e, i) => BindNavItem(e, _sections[i]);
            _navList.itemsSource = _sections;
            _navList.selectionType = SelectionType.Single;
            _navList.selectionChanged += OnNavSelectionChanged;

            _searchResults.makeItem = () => SearchResultTemplate.CloneTree();
            _searchResults.bindItem = (e, i) => BindSearchResult(e, _searchCache[i]);
            _searchResults.itemsSource = _searchCache;
            _searchResults.selectionType = SelectionType.Single;
            _searchResults.selectionChanged += OnSearchResultSelected;

            _toggleButton.clicked += ToggleSidebar;

            UpdateCounts();
        }

        private void Update()
        {
            if (Input.GetKeyDown(ToggleKey))
                ToggleSidebar();
        }

        private void OnSearchChanged(ChangeEvent<string> evt)
        {
            _lastSearchTime = Time.realtimeSinceStartup;
            if (_searchCoroutine != null) StopCoroutine(_searchCoroutine);
            _searchCoroutine = StartCoroutine(SearchAfterDelay());
        }

        private System.Collections.IEnumerator SearchAfterDelay()
        {
            yield return new WaitForSeconds(SearchDebounce);
            PerformSearch(_searchField.value);
        }

        private void PerformSearch(string query)
        {
            if (string.IsNullOrWhiteSpace(query))
            {
                _searchCache.Clear();
                _searchResults.RefreshItems();
                return;
            }

            var q = query.ToLowerInvariant();
            _searchCache.Clear();

            // Search agents
            if (AgentSpawner.Instance != null)
            {
                foreach (var agent in AgentSpawner.Instance.Agents.Values)
                {
                    var def = agent.Definition;
                    if (def.Codename.ToLower().Contains(q) ||
                        def.DisplayName.ToLower().Contains(q) ||
                        def.Role.ToLower().Contains(q))
                    {
                        _searchCache.Add(new SearchResult
                        {
                            Type = "Agent",
                            Title = def.DisplayName,
                            Subtitle = $"{def.Codename} • {def.Role}",
                            Action = () => NavigateToAgent(def.Codename),
                            Icon = "👤"
                        });
                    }
                }
            }

            // Search tasks
            if (TaskManager.Instance != null)
            {
                var tasks = TaskManager.Instance.GetTasks();
                foreach (var task in tasks.Where(t =>
                    t.Title.ToLower().Contains(q) ||
                    t.Id.ToLower().Contains(q) ||
                    t.Assignee.ToLower().Contains(q)).Take(10))
                {
                    _searchCache.Add(new SearchResult
                    {
                        Type = "Task",
                        Title = task.Title,
                        Subtitle = $"{task.Id} • {task.State} • {task.Assignee}",
                        Action = () => NavigateToTask(task.Id),
                        Icon = "📋"
                    });
                }
            }

            // Search projects
            // TODO: Project search

            // Search knowledge
            // TODO: Knowledge search

            // Search commands
            var commands = new[]
            {
                ("/help", "Show help", NavAction.Help),
                ("/terminal", "Switch to terminal", NavAction.Terminal),
                ("/chat", "Switch to chat", NavAction.Chat),
                ("/voice", "Toggle voice", NavAction.Voice),
                ("/density", "Set density mode", NavAction.Density),
                ("/focus", "Toggle focus mode", NavAction.Focus),
                ("/agents", "List agents", NavAction.Agents),
                ("/tasks", "List tasks", NavAction.Tasks),
                ("/projects", "List projects", NavAction.Projects),
                ("/status", "Agency status", NavAction.Status),
                ("/brief", "Daily briefing", NavAction.Briefing),
                ("/clear", "Clear messages", NavAction.Clear)
            };

            foreach (var (cmd, desc, action) in commands.Where(c => c.Item1.Contains(q) || c.Item2.ToLower().Contains(q)))
            {
                _searchCache.Add(new SearchResult
                {
                    Type = "Command",
                    Title = cmd,
                    Subtitle = desc,
                    Action = () => ExecuteCommand(cmd),
                    Icon = "⌨️"
                });
            }

            _searchResults.RefreshItems();
        }

        private void BindNavItem(VisualElement element, NavSection section)
        {
            element.Q<Label>("icon").text = section.Icon;
            element.Q<Label>("title").text = section.Title;
            element.Q<Label>("badge").text = section.BadgeCount > 0 ? section.BadgeCount.ToString() : "";
            element.EnableInClassList("has-badge", section.BadgeCount > 0);
        }

        private void BindSearchResult(VisualElement element, SearchResult result)
        {
            element.Q<Label>("icon").text = result.Icon;
            element.Q<Label>("title").text = result.Title;
            element.Q<Label>("subtitle").text = result.Subtitle;
            element.Q<Label>("type").text = result.Type;
        }

        private void OnNavSelectionChanged(IEnumerable<object> selected)
        {
            var section = selected.FirstOrDefault() as NavSection;
            if (section != null)
                ExecuteNavAction(section.Action);
        }

        private void OnSearchResultSelected(IEnumerable<object> selected)
        {
            var result = selected.FirstOrDefault() as SearchResult;
            if (result != null)
            {
                result.Action?.Invoke();
                _searchField.value = "";
                _searchField.Blur();
                HideSearchResults();
            }
        }

        private void ExecuteNavAction(NavAction action)
        {
            switch (action)
            {
                case NavAction.CommandCenter:
                    Camera.AdaptiveCamera.Instance?.SetMode(Camera.AdaptiveCamera.CameraMode.Overview, true);
                    break;
                case NavAction.Agents:
                    Camera.AdaptiveCamera.Instance?.SetMode(Camera.AdaptiveCamera.CameraMode.Isometric, true);
                    // Show agent list panel
                    break;
                case NavAction.Projects:
                    // Show projects panel
                    break;
                case NavAction.Tasks:
                    // Show tasks panel
                    break;
                case NavAction.Communications:
                    // Show comms panel
                    break;
                case NavAction.Knowledge:
                    Camera.AdaptiveCamera.Instance?.SetMode(Camera.AdaptiveCamera.CameraMode.Isometric, true);
                    // Navigate to Knowledge Archive
                    break;
                case NavAction.Infrastructure:
                    // Show infrastructure panel
                    break;
                case NavAction.Skills:
                    // Show skills panel
                    break;
                case NavAction.Approvals:
                    // Show approvals panel
                    break;
                case NavAction.Dashboard:
                    // Show personal dashboard
                    break;
                case NavAction.Security:
                    // Show security panel
                    break;
                case NavAction.Settings:
                    // Show settings
                    break;
            }
        }

        private void NavigateToAgent(string codename)
        {
            if (Camera.AdaptiveCamera.Instance != null)
                Camera.AdaptiveCamera.Instance.FocusOnAgent(codename);
            CommandInput.Instance?.SetContext($"Agent:{codename}");
        }

        private void NavigateToTask(string taskId)
        {
            CommandInput.Instance?.SetContext($"Task:{taskId}");
        }

        private void ExecuteCommand(string cmd)
        {
            var input = CommandInput.Instance;
            if (input != null)
            {
                input._inputField.value = cmd;
                input.SubmitCommand();
            }
        }

        private void ToggleSidebar()
        {
            _expanded = !_expanded;
            _sidebar.EnableInClassList("collapsed", !_expanded);
            _toggleButton.text = _expanded ? "◀" : "▶";
        }

        private void ShowSearchResults()
        {
            if (_searchCache.Count > 0)
                _searchResultsContainer.style.display = DisplayStyle.Flex;
        }

        private void HideSearchResultsDelayed()
        {
            StartCoroutine(HideSearchResultsCoroutine());
        }

        private System.Collections.IEnumerator HideSearchResultsCoroutine()
        {
            yield return new WaitForSeconds(0.2f);
            HideSearchResults();
        }

        private void HideSearchResults()
        {
            _searchResultsContainer.style.display = DisplayStyle.None;
        }

        public void UpdateCounts(int agents = -1, int tasks = -1, int alerts = -1)
        {
            if (agents >= 0) _agentCountLabel.text = agents.ToString();
            if (tasks >= 0) _taskCountLabel.text = tasks.ToString();
            if (alerts >= 0) _alertCountLabel.text = alerts.ToString();
        }

        public void SetSectionBadge(string sectionId, int count)
        {
            var section = _sections.FirstOrDefault(s => s.Id == sectionId);
            if (section != null)
            {
                section.BadgeCount = count;
                _navList.RefreshItems();
            }
        }

        // Data structures
        private class NavSection
        {
            public string Title { get; }
            public string Id { get; }
            public string Icon { get; }
            public NavAction Action { get; }
            public int BadgeCount { get; set; }

            public NavSection(string title, string id, string icon, NavAction action)
            {
                Title = title; Id = id; Icon = icon; Action = action;
            }
        }

        private class SearchResult
        {
            public string Type { get; set; }
            public string Title { get; set; }
            public string Subtitle { get; set; }
            public string Icon { get; set; }
            public Action Action { get; set; }
        }

        private enum NavAction
        {
            CommandCenter, Agents, Projects, Tasks, Communications,
            Knowledge, Infrastructure, Skills, Approvals,
            Dashboard, Security, Settings,
            Help, Terminal, Chat, Voice, Density, Focus,
            Status, Briefing, Clear
        }
    }
}