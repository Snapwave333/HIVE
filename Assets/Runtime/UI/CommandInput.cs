using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.RegularExpressions;
using UnityEngine;
using UnityEngine.UIElements;
using HIVE.Core;
using HIVE.LLM;
using HIVE.Tasks;
using HIVE.Agents;

namespace HIVE.UI
{
    /// <summary>
    /// Unified command interface: chat, terminal, voice, attachments, visual controls.
    /// Context-aware with tab completion.
    /// </summary>
    [RequireComponent(typeof(UIDocument))]
    public class CommandInput : MonoBehaviour
    {
        public static CommandInput Instance { get; private set; }

        public enum InputMode { Chat, Terminal, Voice }

        [Header("UI References")]
        public UIDocument UIDocument;
        public VisualTreeAsset CommandInputTemplate;
        public VisualTreeAsset MessageTemplate;
        public VisualTreeAsset AttachmentTemplate;

        [Header("Settings")]
        public KeyCode ToggleKey = KeyCode.BackQuote; // ~
        public int MaxHistory = 1000;
        public float AutoCompleteDelay = 0.1f;

        private VisualElement _root;
        private TextField _inputField;
        private ListView _messageList;
        private DropdownField _contextDropdown;
        private Button _modeButton;
        private Button _voiceButton;
        private Button _attachButton;
        private VisualElement _attachmentsContainer;
        private Label _contextBadge;

        private InputMode _currentMode = InputMode.Chat;
        private readonly List<string> _history = new();
        private int _historyIndex = -1;
        private readonly List<AttachmentItem> _attachments = new();
        private string _activeContext = "Agency"; // Agency, Agent:{codename}, Project:{id}, Task:{id}
        private float _lastInputTime;
        private Coroutine _completionCoroutine;

        // Context options
        private readonly List<string> _contextOptions = new() { "Agency" };

        public string ActiveContext => _activeContext;
        public InputMode CurrentMode => _currentMode;

        private void Awake()
        {
            if (Instance == null) Instance = this;
            if (UIDocument == null) UIDocument = GetComponent<UIDocument>();
            if (UIDocument != null)
            {
                _root = UIDocument.rootVisualElement;
                if (CommandInputTemplate != null)
                    CommandInputTemplate.CloneTree(_root);

                _inputField = _root.Q<TextField>("command-input");
                _messageList = _root.Q<ListView>("message-list");
                _contextDropdown = _root.Q<DropdownField>("context-dropdown");
                _modeButton = _root.Q<Button>("mode-button");
                _voiceButton = _root.Q<Button>("voice-button");
                _attachButton = _root.Q<Button>("attach-button");
                _attachmentsContainer = _root.Q<VisualElement>("attachments-container");
                _contextBadge = _root.Q<Label>("context-badge");

                SetupUI();
            }
        }

        private void SetupUI()
        {
            _inputField.RegisterCallback<KeyDownEvent>(OnKeyDown);
            _inputField.RegisterValueChangedCallback(OnInputChanged);
            _contextDropdown.choices = _contextOptions;
            _contextDropdown.value = _contextOptions[0];
            _contextDropdown.RegisterValueChangedCallback(e => SetContext(e.newValue));

            _modeButton.clicked += CycleMode;
            _voiceButton.clicked += ToggleVoice;
            _attachButton.clicked += AttachFile;

            _messageList.makeItem = () => MessageTemplate.CloneTree();
            _messageList.bindItem = (e, i) => BindMessage(e, _messages[i]);
            _messageList.itemsSource = _messages;

            UpdateModeUI();
            UpdateContextUI();
        }

        private readonly List<MessageItem> _messages = new();

        private void Update()
        {
            if (Input.GetKeyDown(ToggleKey))
                ToggleFocus();

            // Process voice if active
            if (_currentMode == InputMode.Voice && _voiceRecording)
                ProcessVoiceInput();
        }

        private void ToggleFocus()
        {
            if (_inputField == null) return;
            var focused = _inputField.focusController?.focusedElement == _inputField;
            if (focused) _inputField.Blur(); else _inputField.Focus();
        }

        private void OnKeyDown(KeyDownEvent evt)
        {
            switch (evt.keyCode)
            {
                case KeyCode.Return when !evt.shiftKey:
                    evt.StopPropagation();
                    SubmitCommand();
                    break;
                case KeyCode.UpArrow when _history.Count > 0:
                    evt.StopPropagation();
                    NavigateHistory(-1);
                    break;
                case KeyCode.DownArrow when _history.Count > 0:
                    evt.StopPropagation();
                    NavigateHistory(1);
                    break;
                case KeyCode.Tab:
                    evt.StopPropagation();
                    TriggerAutoComplete();
                    break;
                case KeyCode.Escape:
                    _inputField.Blur();
                    break;
            }
        }

        private void OnInputChanged(ChangeEvent<string> evt)
        {
            _lastInputTime = Time.realtimeSinceStartup;
            if (_completionCoroutine != null) StopCoroutine(_completionCoroutine);
            _completionCoroutine = StartCoroutine(AutoCompleteAfterDelay());
        }

        private System.Collections.IEnumerator AutoCompleteAfterDelay()
        {
            yield return new WaitForSeconds(AutoCompleteDelay);
            ShowAutoComplete();
        }

        public void SetInputText(string text)
        {
            if (_inputField != null) _inputField.value = text;
        }

        public void FocusInput()
        {
            _inputField?.Focus();
        }

        public void SubmitCommand()
        {
            var text = _inputField.value.Trim();
            if (string.IsNullOrEmpty(text)) return;

            _history.Add(text);
            _historyIndex = _history.Count;

            // Parse command
            var parsed = ParseCommand(text);
            ExecuteCommand(parsed);

            _inputField.value = "";
            _attachments.Clear();
            UpdateAttachmentsUI();
        }

        private ParsedCommand ParseCommand(string text)
        {
            var cmd = new ParsedCommand { Raw = text, Context = _activeContext };

            // @agent syntax
            var agentMatch = Regex.Match(text, @"^@(\w+)\s+(.+)");
            if (agentMatch.Success)
            {
                cmd.TargetAgent = agentMatch.Groups[1].Value;
                cmd.Content = agentMatch.Groups[2].Value;
                cmd.Type = CommandType.DirectMessage;
                return cmd;
            }

            // /command syntax
            var slashMatch = Regex.Match(text, @"^/(\w+)(?:\s+(.*))?");
            if (slashMatch.Success)
            {
                cmd.Type = CommandType.SlashCommand;
                cmd.Command = slashMatch.Groups[1].Value.ToLower();
                cmd.Content = slashMatch.Groups[2].Value ?? "";
                return cmd;
            }

            // Default: chat message to current context
            cmd.Type = CommandType.Chat;
            cmd.Content = text;
            return cmd;
        }

        private async void ExecuteCommand(ParsedCommand cmd)
        {
            AddMessage(new MessageItem
            {
                Type = MessageType.User,
                Content = cmd.Raw,
                Timestamp = DateTime.Now,
                Context = _activeContext,
                Attachments = _attachments.Select(a => a.Path).ToList()
            });

            try
            {
                switch (cmd.Type)
                {
                    case CommandType.DirectMessage:
                        await SendToAgent(cmd.TargetAgent, cmd.Content);
                        break;
                    case CommandType.SlashCommand:
                        await ExecuteSlashCommand(cmd);
                        break;
                    case CommandType.Chat:
                        await SendToContext(cmd.Content);
                        break;
                }
            }
            catch (Exception e)
            {
                AddMessage(new MessageItem
                {
                    Type = MessageType.Error,
                    Content = $"Error: {e.Message}",
                    Timestamp = DateTime.Now
                });
            }
        }

        private async System.Threading.Tasks.Task SendToAgent(string agentCodename, string content)
        {
            var system = $"You are {agentCodename}, an agent in the H.I.V.E. agency. Respond concisely.";
            var response = await NimCascadeRouter.CompleteAsync(system, content, freeOnly: false);

            AddMessage(new MessageItem
            {
                Type = MessageType.Agent,
                Content = response,
                Timestamp = DateTime.Now,
                Sender = agentCodename,
                Context = _activeContext
            });
        }

        private async System.Threading.Tasks.Task SendToContext(string content)
        {
            // Route based on context
            if (_activeContext.StartsWith("Agent:"))
            {
                var agent = _activeContext.Substring(6);
                await SendToAgent(agent, content);
            }
            else if (_activeContext.StartsWith("Project:"))
            {
                // Send to project channel
                AddMessage(new MessageItem
                {
                    Type = MessageType.System,
                    Content = $"[Project {_activeContext.Substring(8)}] {content}",
                    Timestamp = DateTime.Now
                });
                // TODO: Publish to message bus
            }
            else
            {
                // Agency-wide
                AddMessage(new MessageItem
                {
                    Type = MessageType.System,
                    Content = $"[Agency] {content}",
                    Timestamp = DateTime.Now
                });
            }
        }

        private async System.Threading.Tasks.Task ExecuteSlashCommand(ParsedCommand cmd)
        {
            switch (cmd.Command)
            {
                case "help":
                    ShowHelp();
                    break;
                case "terminal":
                    SetMode(InputMode.Terminal);
                    break;
                case "chat":
                    SetMode(InputMode.Chat);
                    break;
                case "voice":
                    SetMode(InputMode.Voice);
                    break;
                case "density":
                    SetDensity(cmd.Content);
                    break;
                case "focus":
                    SetFocusMode(cmd.Content == "on");
                    break;
                case "agents":
                    ListAgents();
                    break;
                case "tasks":
                    ListTasks(cmd.Content);
                    break;
                case "projects":
                    ListProjects();
                    break;
                case "approve":
                    ApproveTask(cmd.Content);
                    break;
                case "reject":
                    RejectTask(cmd.Content);
                    break;
                case "status":
                    RequestStatus();
                    break;
                case "brief":
                    RequestBriefing();
                    break;
                case "clear":
                    _messages.Clear();
                    _messageList.RefreshItems();
                    break;
                default:
                    AddMessage(new MessageItem
                    {
                        Type = MessageType.Error,
                        Content = $"Unknown command: /{cmd.Command}. Type /help for list.",
                        Timestamp = DateTime.Now
                    });
                    break;
            }
        }

        private void ShowHelp()
        {
            var help = @"
<b>H.I.V.E. Command Interface</b>

<b>Direct Messages:</b>
  @PRIME status          → Send to specific agent
  @SAGE research topic  → Ask agent to research

<b>Slash Commands:</b>
  /help                  → This help
  /terminal              → Switch to terminal mode
  /chat                  → Switch to chat mode
  /voice                 → Toggle voice mode
  /density dense|comfortable|focus  → Set UI density
  /focus on|off          → Toggle focus mode
  /agents                → List all agents
  /tasks [agent|project] → List tasks
  /projects              → List projects
  /approve task-id       → Approve pending task
  /reject task-id        → Reject pending task
  /status                → Request agency status
  /brief                 → Request daily briefing
  /clear                 → Clear message history

<b>Context:</b>
  Dropdown → Select Agency, Agent, Project, or Task
  Context persists across commands

<b>Attachments:</b>
  📎 button → Attach files (logs, images, code)
  Files sent with next command

<b>Shortcuts:</b>
  ` (tilde) → Toggle focus
  ↑/↓      → History navigation
  Tab      → Auto-complete
  F        → Focus camera on selected agent
";
            AddMessage(new MessageItem
            {
                Type = MessageType.Help,
                Content = help,
                Timestamp = DateTime.Now
            });
        }

        private void ListAgents()
        {
            // Request agent status via message bus
            var req = new AgentStatusRequest
            {
                CorrelationId = Guid.NewGuid().ToString(),
                SourceAgent = "COMMAND_INPUT"
            };
            MessageBus.Publish(req);
        }

        private void ListTasks(string filter) { /* TODO */ }
        private void ListProjects() { /* TODO */ }
        private void ApproveTask(string taskId) { /* TODO */ }
        private void RejectTask(string taskId) { /* TODO */ }
        private void RequestStatus() { /* TODO */ }
        private void RequestBriefing() { /* TODO */ }

        private void SetDensity(string mode)
        {
            if (Enum.TryParse<DensityController.DensityMode>(mode, true, out var dm))
            {
                // Notify DensityController
                AddMessage(new MessageItem { Type = MessageType.System, Content = $"Density set to {dm}", Timestamp = DateTime.Now });
            }
        }

        private void SetFocusMode(bool on)
        {
            AddMessage(new MessageItem { Type = MessageType.System, Content = $"Focus mode {(on ? "enabled" : "disabled")}", Timestamp = DateTime.Now });
        }

        private void CycleMode()
        {
            _currentMode = (InputMode)(((int)_currentMode + 1) % 3);
            UpdateModeUI();
        }

        private void SetMode(InputMode mode)
        {
            _currentMode = mode;
            UpdateModeUI();
        }

        private void UpdateModeUI()
        {
            _modeButton.text = _currentMode.ToString().ToUpper();
            _voiceButton.style.display = _currentMode == InputMode.Voice ? DisplayStyle.Flex : DisplayStyle.None;
            _inputField.label = _currentMode == InputMode.Terminal ? "$ " : "> ";
        }

        private bool _voiceRecording;
        private void ToggleVoice()
        {
            _voiceRecording = !_voiceRecording;
            _voiceButton.text = _voiceRecording ? "🔴 Recording" : "🎤 Voice";
            if (_voiceRecording) StartVoiceRecording(); else StopVoiceRecording();
        }

        private void StartVoiceRecording() { /* TODO: Whisper.cpp integration */ }
        private void StopVoiceRecording() { /* TODO */ }
        private void ProcessVoiceInput() { /* TODO */ }

        private void AttachFile()
        {
            // Open file dialog
            var path = UnityEditor.EditorUtility.OpenFilePanel("Attach File", "", "txt,log,json,cs,py,md,png,jpg,pdf");
            if (!string.IsNullOrEmpty(path))
            {
                _attachments.Add(new AttachmentItem { Path = path, Name = System.IO.Path.GetFileName(path) });
                UpdateAttachmentsUI();
            }
        }

        private void UpdateAttachmentsUI()
        {
            _attachmentsContainer.Clear();
            foreach (var att in _attachments)
            {
                var item = AttachmentTemplate.CloneTree();
                item.Q<Label>("name").text = att.Name;
                item.Q<Button>("remove").clicked += () => { _attachments.Remove(att); UpdateAttachmentsUI(); };
                _attachmentsContainer.Add(item);
            }
        }

        public void SetContext(string context)
        {
            _activeContext = context;
            UpdateContextUI();
        }

        private void UpdateContextUI()
        {
            _contextBadge.text = _activeContext;
            _contextBadge.style.backgroundColor = GetContextColor(_activeContext);
        }

        private Color GetContextColor(string ctx)
        {
            if (ctx == "Agency") return new Color(0.2f, 0.4f, 0.2f);
            if (ctx.StartsWith("Agent:")) return new Color(0.4f, 0.2f, 0.4f);
            if (ctx.StartsWith("Project:")) return new Color(0.2f, 0.3f, 0.5f);
            if (ctx.StartsWith("Task:")) return new Color(0.5f, 0.3f, 0.2f);
            return Color.gray;
        }

        private void NavigateHistory(int direction)
        {
            if (_history.Count == 0) return;
            _historyIndex = Mathf.Clamp(_historyIndex + direction, 0, _history.Count - 1);
            _inputField.value = _history[_historyIndex];
        }

        private void TriggerAutoComplete() { /* TODO: Show completion popup */ }
        private void ShowAutoComplete() { /* TODO */ }

        private void BindMessage(VisualElement element, MessageItem msg)
        {
            element.Q<Label>("sender").text = msg.Sender ?? (msg.Type == MessageType.User ? "You" : "System");
            element.Q<Label>("content").text = msg.Content;
            element.Q<Label>("time").text = msg.Timestamp.ToString("HH:mm:ss");
            element.RemoveFromClassList("user");
            element.RemoveFromClassList("agent");
            element.RemoveFromClassList("system");
            element.RemoveFromClassList("error");
            element.RemoveFromClassList("help");
            element.AddToClassList(msg.Type.ToString().ToLower());
        }

        private void AddMessage(MessageItem msg)
        {
            _messages.Add(msg);
            _messageList.RefreshItems();
            _messageList.ScrollToItem(_messages.Count - 1);
        }

        // Auto-complete context options
        public void RefreshContextOptions()
        {
            _contextOptions.Clear();
            _contextOptions.Add("Agency");
            // Add agents, projects, tasks from registry
            // TODO: Populate from AgentSpawner, TaskManager
            _contextDropdown.choices = _contextOptions;
        }

        // Message types
        private class MessageItem
        {
            public MessageType Type { get; set; }
            public string Content { get; set; }
            public DateTime Timestamp { get; set; }
            public string Sender { get; set; }
            public string Context { get; set; }
            public List<string> Attachments { get; set; }
        }

        private enum MessageType { User, Agent, System, Error, Help }

        private class ParsedCommand
        {
            public CommandType Type { get; set; }
            public string Raw { get; set; }
            public string Context { get; set; }
            public string TargetAgent { get; set; }
            public string Command { get; set; }
            public string Content { get; set; }
        }

        private enum CommandType { Chat, DirectMessage, SlashCommand }

        private class AttachmentItem
        {
            public string Path { get; set; }
            public string Name { get; set; }
        }
    }
}