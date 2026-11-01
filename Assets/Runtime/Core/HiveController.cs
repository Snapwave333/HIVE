using System;
using System.Collections;
using System.Linq;
using UnityEngine;
using UnityEngine.SceneManagement;
using HIVE.Core;
using HIVE.LLM;
using HIVE.Persistence;
using HIVE.Agents;
using HIVE.Tasks;
using HIVE.Memory;
using HIVE.UI;
using HIVE.Camera;

namespace HIVE.Core
{
    /// <summary>
    /// Main entry point: initializes all H.I.V.E. subsystems, manages scene loading, coordinates startup/shutdown.
    /// </summary>
    public class HiveController : MonoBehaviour
    {
        public static HiveController Instance { get; private set; }

        [Header("Scene Management")]
        public string[] StartupScenes = new[]
        {
            "CommandChamber",
            "Workspaces",
            "CollabChamber",
            "KnowledgeArchive",
            "Infrastructure",
            "OwnerControl"
        };

        [Header("System Prefabs")]
        public GameObject AgentWorkspacePrefab;
        public Transform WorkspaceContainer;

        [Header("Settings")]
        public bool AutoInitialize = true;
        public bool LoadAdditiveScenes = true;

        // Subsystems
        public Repository Repository { get; private set; }
        public AgentSpawner AgentSpawner { get; private set; }
        public TaskManager TaskManager { get; private set; }
        public MemoryStore MemoryStore { get; private set; }
        public CommandInput CommandInput { get; private set; }
        public NavSidebar NavSidebar { get; private set; }
        public AdaptiveCamera AdaptiveCamera { get; private set; }
        public DensityController DensityController { get; private set; }
        public NotificationCenter NotificationCenter { get; private set; }
        public HIVE.Security.SecurityGate SecurityGate { get; private set; }
        public HIVE.Skills.SkillRegistry SkillRegistry { get; private set; }
        public HIVE.Voice.VoiceController VoiceController { get; private set; }
        public HIVE.Coding.ICodingOrchestrator CodingOrchestrator { get; private set; }
        public HIVE.Integrations.GoogleEcosystemBridge GoogleEcosystem { get; private set; }

        public bool IsInitialized { get; private set; }

        private void Awake()
        {
            if (Instance != null && Instance != this)
            {
                Destroy(gameObject);
                return;
            }
            Instance = this;
            DontDestroyOnLoad(gameObject);

            if (AutoInitialize)
                StartCoroutine(InitializeAsync());
        }

        private IEnumerator InitializeAsync()
        {
            Debug.Log("[HIVE] Starting initialization...");

            // 1. Core persistence
            yield return InitializeRepository();

            // 2. LLM Router health check
            yield return CheckLLMHealth();

            // 3. Spawn agents
            yield return InitializeAgents();

            // 4. Task system
            yield return InitializeTasks();

            // 5. Memory system
            yield return InitializeMemory();

            // 6. Security & Skills
            yield return InitializeSecurityAndSkills();

            // 7. UI systems
            yield return InitializeUI();

            // 7. Camera system
            yield return InitializeCamera();

            // 8. Load 3D scenes
            yield return LoadScenes();

            // 9. Post-init
            yield return PostInitialization();

            IsInitialized = true;
            Debug.Log("[HIVE] Initialization complete!");

            // Fire ready event
            MessageBus.Publish(new HiveReadyMessage());
        }

        private IEnumerator InitializeRepository()
        {
            Debug.Log("[HIVE] Initializing Repository...");
            Repository = new Repository();
            yield return null;
        }

        private IEnumerator CheckLLMHealth()
        {
            Debug.Log("[HIVE] Checking LLM health...");
            var healthTask = System.Threading.Tasks.Task.Run(() => NimCascadeRouter.GetHealthAsync());
            while (!healthTask.IsCompleted)
            {
                yield return null;
            }
            if (healthTask.IsCompletedSuccessfully)
            {
                var health = healthTask.Result;
                Debug.Log($"[HIVE] LLM Health: {(health.ok ? "OK" : "DEGRADED")} - {health.detail}");
            }
        }

        private IEnumerator InitializeAgents()
        {
            Debug.Log("[HIVE] Spawning agents...");
            var spawnerGO = new GameObject("AgentSpawner");
            spawnerGO.transform.SetParent(transform);
            AgentSpawner = spawnerGO.AddComponent<AgentSpawner>();
            AgentSpawner.AgentWorkspacePrefab = AgentWorkspacePrefab;
            AgentSpawner.WorkspaceContainer = WorkspaceContainer;
            AgentSpawner.Initialize(Repository);
            yield return null;
        }

        private IEnumerator InitializeTasks()
        {
            Debug.Log("[HIVE] Initializing TaskManager...");
            var taskGO = new GameObject("TaskManager");
            taskGO.transform.SetParent(transform);
            TaskManager = taskGO.AddComponent<TaskManager>();
            TaskManager.Initialize(Repository);
            yield return null;
        }

        private IEnumerator InitializeMemory()
        {
            Debug.Log("[HIVE] Initializing MemoryStore...");
            var memGO = new GameObject("MemoryStore");
            memGO.transform.SetParent(transform);
            MemoryStore = memGO.AddComponent<MemoryStore>();
            MemoryStore.Initialize(Repository);
            yield return null;
        }

        private IEnumerator InitializeSecurityAndSkills()
        {
            Debug.Log("[HIVE] Initializing SecurityGate, SkillRegistry, and VoiceController...");
            var secGO = new GameObject("SecurityGate");
            secGO.transform.SetParent(transform);
            SecurityGate = secGO.AddComponent<HIVE.Security.SecurityGate>();
            SecurityGate.Initialize(Repository);

            var skillGO = new GameObject("SkillRegistry");
            skillGO.transform.SetParent(transform);
            SkillRegistry = skillGO.AddComponent<HIVE.Skills.SkillRegistry>();
            SkillRegistry.Initialize(Repository);

            var voiceGO = new GameObject("VoiceController");
            voiceGO.transform.SetParent(transform);
            voiceGO.AddComponent<AudioSource>();
            VoiceController = voiceGO.AddComponent<HIVE.Voice.VoiceController>();

            // Initialize Coding Orchestrator
            Debug.Log("[HIVE] Initializing CodingOrchestrator...");
            CodingOrchestrator = new HIVE.Coding.CodingOrchestrator();

            // Initialize Google Account Ecosystem Bridge (gog CLI)
            Debug.Log("[HIVE] Initializing GoogleEcosystemBridge...");
            GoogleEcosystem = HIVE.Integrations.GoogleEcosystemBridge.Instance;
            System.Threading.Tasks.Task.Run(async () =>
            {
                try
                {
                    var status = await GoogleEcosystem.CheckAuthStatusAsync();
                    if (status.Authenticated)
                    {
                        Debug.Log($"[HIVE] Google Ecosystem connected: {status.Email} (Client: {status.Client})");
                    }
                    else
                    {
                        Debug.LogWarning($"[HIVE] Google Ecosystem unauthenticated: {status.Error}");
                    }
                }
                catch (Exception ex)
                {
                    Debug.LogWarning($"[HIVE] Google Ecosystem status probe error: {ex.Message}");
                }
            });

            yield return null;
        }

        private IEnumerator InitializeUI()
        {
            Debug.Log("[HIVE] Initializing UI systems...");

            // Command Input
            var cmdGO = GameObject.Find("CommandInput");
            if (cmdGO != null)
                CommandInput = cmdGO.GetComponent<CommandInput>();
            if (CommandInput == null)
            {
                cmdGO = new GameObject("CommandInput");
                cmdGO.transform.SetParent(transform);
                CommandInput = cmdGO.AddComponent<CommandInput>();
            }

            // Nav Sidebar
            var navGO = GameObject.Find("NavSidebar");
            if (navGO != null)
                NavSidebar = navGO.GetComponent<NavSidebar>();
            if (NavSidebar == null)
            {
                navGO = new GameObject("NavSidebar");
                navGO.transform.SetParent(transform);
                NavSidebar = navGO.AddComponent<NavSidebar>();
            }

            // Density Controller
            var densGO = new GameObject("DensityController");
            densGO.transform.SetParent(transform);
            DensityController = densGO.AddComponent<DensityController>();

            // Notification Center
            var notifGO = new GameObject("NotificationCenter");
            notifGO.transform.SetParent(transform);
            NotificationCenter = notifGO.AddComponent<NotificationCenter>();

            yield return null;
        }

        private IEnumerator InitializeCamera()
        {
            Debug.Log("[HIVE] Initializing Camera...");
            var camGO = GameObject.Find("AdaptiveCamera");
            if (camGO != null)
                AdaptiveCamera = camGO.GetComponent<AdaptiveCamera>();
            if (AdaptiveCamera == null)
            {
                camGO = new GameObject("AdaptiveCamera");
                camGO.transform.SetParent(transform);
                AdaptiveCamera = camGO.AddComponent<AdaptiveCamera>();
                if (UnityEngine.Camera.main != null)
                    AdaptiveCamera.CameraTransform = UnityEngine.Camera.main.transform;
            }
            yield return null;
        }

        private IEnumerator LoadScenes()
        {
            if (!LoadAdditiveScenes) yield break;

            Debug.Log("[HIVE] Loading additive scenes...");
            foreach (var sceneName in StartupScenes)
            {
                if (SceneUtility.GetBuildIndexByScenePath($"Assets/Scenes/{sceneName}.unity") >= 0)
                {
                    var op = SceneManager.LoadSceneAsync(sceneName, LoadSceneMode.Additive);
                    while (!op.isDone) yield return null;
                    Debug.Log($"[HIVE] Loaded scene: {sceneName}");
                }
                else
                {
                    Debug.LogWarning($"[HIVE] Scene not in build settings: {sceneName}");
                }
            }
        }

        private IEnumerator PostInitialization()
        {
            // Request initial agent status
            if (AgentSpawner != null)
            {
                var req = new AgentStatusRequest
                {
                    CorrelationId = Guid.NewGuid().ToString(),
                    SourceAgent = "HIVE_CONTROLLER"
                };
                MessageBus.Publish(req);
            }

            // Schedule daily briefing
            ScheduleDailyBriefing();

            // Log startup
            Repository?.WriteAudit(new AuditEntry
            {
                EventType = "system_startup",
                Actor = "HIVE_CONTROLLER",
                Target = "hive",
                Action = "initialize",
                Details = new System.Collections.Generic.Dictionary<string, object>
                {
                    { "agent_count", AgentSpawner?.Agents.Count ?? 0 },
                    { "timestamp", DateTime.UtcNow.ToString("o") }
                }
            });

            yield return null;
        }

        private void ScheduleDailyBriefing()
        {
            // TODO: Implement daily briefing at configured time (default 08:00)
            StartCoroutine(DailyBriefingLoop());
        }

        private IEnumerator DailyBriefingLoop()
        {
            while (true)
            {
                var now = DateTime.Now;
                var nextBriefing = new DateTime(now.Year, now.Month, now.Day, 8, 0, 0);
                if (nextBriefing <= now) nextBriefing = nextBriefing.AddDays(1);

                var wait = nextBriefing - now;
                yield return new WaitForSeconds((float)wait.TotalSeconds);

                GenerateDailyBriefing();
            }
        }

        private void GenerateDailyBriefing()
        {
            // TODO: Use LLM to generate briefing from overnight activity
            var briefing = new
            {
                date = DateTime.UtcNow.ToString("yyyy-MM-dd"),
                summary = "Daily briefing generated",
                completedTasks = TaskManager?.GetTasks(state: "Verified").Count ?? 0,
                activeAgents = AgentSpawner?.Agents.Values.Count(a => a.State.Status == "Active") ?? 0,
                pendingApprovals = 0, // TODO
                alerts = 0 // TODO
            };

            NotificationCenter?.ShowNotification(new NotificationCenter.Notification
            {
                Title = "Daily Briefing",
                Message = JsonUtility.ToJson(briefing, true),
                Priority = NotificationCenter.Priority.Important,
                Category = "briefing"
            });
        }

        private void OnApplicationQuit()
        {
            Debug.Log("[HIVE] Shutting down...");
            MessageBus.Shutdown();
            Repository?.Dispose();
        }

        // Public API for external access
        public T GetSystem<T>() where T : Component
        {
            var type = typeof(T).Name;
            switch (type)
            {
                case nameof(Repository): return Repository as T;
                case nameof(AgentSpawner): return AgentSpawner as T;
                case nameof(TaskManager): return TaskManager as T;
                case nameof(MemoryStore): return MemoryStore as T;
                case nameof(CommandInput): return CommandInput as T;
                case nameof(NavSidebar): return NavSidebar as T;
                case nameof(AdaptiveCamera): return AdaptiveCamera as T;
                case nameof(DensityController): return DensityController as T;
                case nameof(NotificationCenter): return NotificationCenter as T;
                case nameof(HIVE.Security.SecurityGate): return SecurityGate as T;
                case nameof(HIVE.Skills.SkillRegistry): return SkillRegistry as T;
                case nameof(HIVE.Voice.VoiceController): return VoiceController as T;
            }
            return null;
        }
    }

    public class HiveReadyMessage : MessageBus.IMessage
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
    }
}