using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.UIElements;
using HIVE.Core;

namespace HIVE.UI
{
    /// <summary>
    /// Notification system with priority tiers, quiet periods, per-agent/project mute.
    /// </summary>
    public class NotificationCenter : MonoBehaviour
    {
        public enum Priority { Critical, ActionRequired, Important, Informational }

        [Header("Settings")]
        public float DisplayDuration = 8f;
        public int MaxVisible = 5;
        public KeyCode HistoryKey = KeyCode.F2;

        [Header("Quiet Periods")]
        public bool EnableQuietPeriods = true;
        [System.NonSerialized] public TimeSpan QuietStart = new TimeSpan(22, 0, 0); // 10 PM
        [System.NonSerialized] public TimeSpan QuietEnd = new TimeSpan(7, 0, 0);    // 7 AM

        [Header("Mute Lists")]
        public List<string> MutedAgents = new();
        public List<string> MutedProjects = new();

        private readonly Queue<Notification> _queue = new();
        private readonly List<Notification> _history = new();
        private VisualElement _toastContainer;
        private VisualElement _historyPanel;
        private bool _showingHistory;

        public event Action<Notification> OnNotification;

        private void Awake()
        {
            var doc = GetComponent<UIDocument>();
            if (doc != null)
            {
                _toastContainer = doc.rootVisualElement.Q<VisualElement>("toast-container");
                _historyPanel = doc.rootVisualElement.Q<VisualElement>("history-panel");
            }
        }

        private void Update()
        {
            if (Input.GetKeyDown(HistoryKey))
                ToggleHistory();
        }

        public void ShowNotification(Notification notification)
        {
            // Check quiet period
            if (EnableQuietPeriods && IsQuietPeriod() && notification.Priority < Priority.ActionRequired)
            {
                _history.Add(notification);
                return;
            }

            // Check mute lists
            if (notification.SourceAgent != null && MutedAgents.Contains(notification.SourceAgent)) return;
            if (notification.ProjectId != null && MutedProjects.Contains(notification.ProjectId)) return;

            _queue.Enqueue(notification);
            _history.Add(notification);
            OnNotification?.Invoke(notification);

            if (_toastContainer != null)
                ShowToast(notification);

            // Trim history
            if (_history.Count > 1000) _history.RemoveRange(0, _history.Count - 1000);
        }

        private bool IsQuietPeriod()
        {
            var now = DateTime.Now.TimeOfDay;
            if (QuietStart <= QuietEnd)
                return now >= QuietStart && now <= QuietEnd;
            return now >= QuietStart || now <= QuietEnd;
        }

        private void ShowToast(Notification notification)
        {
            var toast = new VisualElement();
            toast.AddToClassList("notification-toast");
            toast.AddToClassList(notification.Priority.ToString().ToLower());

            var icon = new Label(GetPriorityIcon(notification.Priority));
            icon.AddToClassList("icon");

            var content = new VisualElement();
            content.AddToClassList("content");
            content.Add(new Label(notification.Title) { name = "title" });
            if (!string.IsNullOrEmpty(notification.Message))
                content.Add(new Label(notification.Message) { name = "message" });

            var actions = new VisualElement();
            actions.AddToClassList("actions");
            if (notification.ActionLabel != null && notification.Action != null)
            {
                var btn = new Button(() => { notification.Action(); toast.RemoveFromHierarchy(); });
                btn.text = notification.ActionLabel;
                actions.Add(btn);
            }
            var dismiss = new Button(() => toast.RemoveFromHierarchy());
            dismiss.text = "✕";
            dismiss.AddToClassList("dismiss");
            actions.Add(dismiss);

            toast.Add(icon);
            toast.Add(content);
            toast.Add(actions);

            _toastContainer.Add(toast);

            // Limit visible toasts
            while (_toastContainer.childCount > MaxVisible)
                _toastContainer[0].RemoveFromHierarchy();

            // Auto-dismiss
            if (notification.Priority != Priority.Critical)
            {
                var duration = DisplayDuration * (notification.Priority == Priority.Important ? 1.5f : 1f);
                StartCoroutine(AutoDismiss(toast, duration));
            }
        }

        private System.Collections.IEnumerator AutoDismiss(VisualElement toast, float delay)
        {
            yield return new WaitForSeconds(delay);
            if (toast != null && toast.parent != null)
                toast.RemoveFromHierarchy();
        }

        private string GetPriorityIcon(Priority p) => p switch
        {
            Priority.Critical => "🔴",
            Priority.ActionRequired => "🟠",
            Priority.Important => "🟡",
            _ => "🔵"
        };

        public void ToggleHistory()
        {
            _showingHistory = !_showingHistory;
            if (_historyPanel != null)
                _historyPanel.style.display = _showingHistory ? DisplayStyle.Flex : DisplayStyle.None;

            if (_showingHistory)
                RefreshHistory();
        }

        private void RefreshHistory()
        {
            if (_historyPanel == null) return;
            _historyPanel.Clear();

            foreach (var n in Enumerable.Reverse(_history).Take(50))
            {
                var item = new VisualElement();
                item.AddToClassList("history-item");
                item.AddToClassList(n.Priority.ToString().ToLower());

                var header = new VisualElement();
                header.AddToClassList("header");
                header.Add(new Label(GetPriorityIcon(n.Priority)) { name = "icon" });
                header.Add(new Label(n.Title) { name = "title" });
                header.Add(new Label(n.Timestamp.ToString("HH:mm:ss")) { name = "time" });

                var body = new Label(n.Message) { name = "body" };

                item.Add(header);
                if (!string.IsNullOrEmpty(n.Message)) item.Add(body);

                _historyPanel.Add(item);
            }
        }

        public void MuteAgent(string agentId)
        {
            if (!MutedAgents.Contains(agentId)) MutedAgents.Add(agentId);
        }

        public void UnmuteAgent(string agentId) => MutedAgents.Remove(agentId);

        public void MuteProject(string projectId)
        {
            if (!MutedProjects.Contains(projectId)) MutedProjects.Add(projectId);
        }

        public void UnmuteProject(string projectId) => MutedProjects.Remove(projectId);

        public List<Notification> GetHistory(int count = 100) => _history.Skip(Math.Max(0, _history.Count - count)).ToList();

        [Serializable]
        public class Notification
        {
            public string Id { get; set; } = Guid.NewGuid().ToString();
            public string Title { get; set; }
            public string Message { get; set; }
            public Priority Priority { get; set; }
            public string SourceAgent { get; set; }
            public string ProjectId { get; set; }
            public string Category { get; set; }
            public DateTime Timestamp { get; set; } = DateTime.UtcNow;
            public string ActionLabel { get; set; }
            public Action Action { get; set; }
        }
    }
}