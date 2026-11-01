using System;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;
using UnityEngine.UIElements;
using HIVE.Core;
using HIVE.Agents;
using HIVE.Camera;
using HIVE.UI;

namespace HIVE.Editor
{
    /// <summary>
    /// Automates the creation and setup of all 7 H.I.V.E. operational 3D scenes,
    /// cybernetic Borg megastructure prefabs, materials, and Build Settings.
    /// </summary>
    public static class HiveSceneBuilder
    {
        private const string SCENES_DIR = "Assets/Scenes";
        private const string PREFABS_DIR = "Assets/Prefabs";
        private const string MATERIALS_DIR = "Assets/Art/Materials";

        [MenuItem("HIVE/Build All Scenes and Prefabs")]
        public static void BuildAll()
        {
            Debug.Log("[HiveSceneBuilder] Starting end-to-end build of scenes and prefabs...");

            EnsureDirectories();
            var (cyberMat, glowMat, hexMat) = CreateMaterials();
            var avatarPrefab = CreateAvatarPrefab(cyberMat, glowMat);
            var workspacePrefab = CreateWorkspacePrefab(avatarPrefab, glowMat);

            CreateCommandChamberScene(workspacePrefab, cyberMat, glowMat, hexMat);
            CreateWorkspacesScene(workspacePrefab, cyberMat, glowMat);
            CreateCollabChamberScene(cyberMat, glowMat);
            CreateKnowledgeArchiveScene(cyberMat, glowMat);
            CreateInfrastructureScene(cyberMat, glowMat);
            CreateOwnerControlScene(cyberMat, glowMat);
            CreateAtriumScene(cyberMat, glowMat);

            ConfigureBuildSettings();

            AssetDatabase.SaveAssets();
            AssetDatabase.Refresh();
            Debug.Log("[HiveSceneBuilder] All 7 operational scenes, prefabs, and Build Settings successfully generated!");
        }

        private static void EnsureDirectories()
        {
            if (!Directory.Exists(SCENES_DIR)) Directory.CreateDirectory(SCENES_DIR);
            if (!Directory.Exists(PREFABS_DIR)) Directory.CreateDirectory(PREFABS_DIR);
            if (!Directory.Exists(MATERIALS_DIR)) Directory.CreateDirectory(MATERIALS_DIR);
        }

        private static (Material cyberMat, Material glowMat, Material hexMat) CreateMaterials()
        {
            var shader = Shader.Find("Universal Render Pipeline/Lit") ?? Shader.Find("Standard");

            // Obsidian cybernetic wall/floor material
            var cyberMat = new Material(shader) { name = "Mat_CyberObsidian" };
            cyberMat.color = new Color(0.06f, 0.08f, 0.1f);
            AssetDatabase.CreateAsset(cyberMat, $"{MATERIALS_DIR}/Mat_CyberObsidian.mat");

            // Glowing Borg green hologram/circuit material
            var glowMat = new Material(shader) { name = "Mat_BorgGlow" };
            var borgGreen = new Color(0.52f, 0.85f, 0.29f);
            glowMat.color = borgGreen;
            glowMat.EnableKeyword("_EMISSION");
            glowMat.SetColor("_EmissionColor", borgGreen * 2.5f);
            AssetDatabase.CreateAsset(glowMat, $"{MATERIALS_DIR}/Mat_BorgGlow.mat");

            // Hex accent cyan material
            var hexMat = new Material(shader) { name = "Mat_HexTeal" };
            var teal = new Color(0.26f, 0.81f, 0.77f);
            hexMat.color = teal;
            hexMat.EnableKeyword("_EMISSION");
            hexMat.SetColor("_EmissionColor", teal * 1.8f);
            AssetDatabase.CreateAsset(hexMat, $"{MATERIALS_DIR}/Mat_HexTeal.mat");

            return (cyberMat, glowMat, hexMat);
        }

        private static GameObject CreateAvatarPrefab(Material cyberMat, Material glowMat)
        {
            var root = new GameObject("AvatarPrefab");
            var body = GameObject.CreatePrimitive(PrimitiveType.Cylinder);
            body.name = "Body";
            body.transform.SetParent(root.transform);
            body.transform.localPosition = new Vector3(0, 0.9f, 0);
            body.transform.localScale = new Vector3(0.6f, 0.8f, 0.6f);
            body.GetComponent<Renderer>().sharedMaterial = cyberMat;

            var head = GameObject.CreatePrimitive(PrimitiveType.Sphere);
            head.name = "Head";
            head.transform.SetParent(root.transform);
            head.transform.localPosition = new Vector3(0, 1.8f, 0);
            head.transform.localScale = new Vector3(0.45f, 0.45f, 0.45f);
            head.GetComponent<Renderer>().sharedMaterial = glowMat;

            var prefabPath = $"{PREFABS_DIR}/AvatarPrefab.prefab";
            var prefab = PrefabUtility.SaveAsPrefabAsset(root, prefabPath);
            GameObject.DestroyImmediate(root);
            return prefab;
        }

        private static GameObject CreateWorkspacePrefab(GameObject avatarPrefab, Material glowMat)
        {
            var root = new GameObject("WorkspacePrefab");
            var workspaceComp = root.AddComponent<AgentWorkspace>();

            var uiDoc = root.AddComponent<UIDocument>();
            var template = AssetDatabase.LoadAssetAtPath<VisualTreeAsset>("Assets/Runtime/UI/Templates/WorkspaceView.uxml");
            workspaceComp.WorkspaceTemplate = template;
            workspaceComp.UIDocument = uiDoc;
            workspaceComp.AvatarPrefab = avatarPrefab;
            workspaceComp.HologramMaterial = glowMat;

            // Desk terminal geometry
            var desk = GameObject.CreatePrimitive(PrimitiveType.Cube);
            desk.name = "TerminalDesk";
            desk.transform.SetParent(root.transform);
            desk.transform.localPosition = new Vector3(0, 0.4f, 0.6f);
            desk.transform.localScale = new Vector3(1.6f, 0.8f, 0.8f);

            var screen = GameObject.CreatePrimitive(PrimitiveType.Quad);
            screen.name = "HoloScreen";
            screen.transform.SetParent(root.transform);
            screen.transform.localPosition = new Vector3(0, 1.2f, 0.9f);
            screen.transform.localScale = new Vector3(1.2f, 0.7f, 1f);
            screen.GetComponent<Renderer>().sharedMaterial = glowMat;

            // Anchors
            var holoAnchor = new GameObject("HologramAnchor").transform;
            holoAnchor.SetParent(root.transform);
            holoAnchor.localPosition = new Vector3(0, 1.5f, 0);
            workspaceComp.HologramAnchor = holoAnchor;

            var taskAnchor = new GameObject("TaskDisplayAnchor").transform;
            taskAnchor.SetParent(root.transform);
            taskAnchor.localPosition = new Vector3(0.8f, 1.2f, 0);
            workspaceComp.TaskDisplayAnchor = taskAnchor;

            var netAnchor = new GameObject("NetworkConnectionsAnchor").transform;
            netAnchor.SetParent(root.transform);
            netAnchor.localPosition = new Vector3(-0.8f, 1.2f, 0);
            workspaceComp.NetworkConnectionsAnchor = netAnchor;

            var prefabPath = $"{PREFABS_DIR}/AgentWorkspacePrefab.prefab";
            var prefab = PrefabUtility.SaveAsPrefabAsset(root, prefabPath);
            GameObject.DestroyImmediate(root);
            return prefab;
        }

        private static void CreateCommandChamberScene(GameObject wsPrefab, Material cyberMat, Material glowMat, Material hexMat)
        {
            var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);

            // Lighting & Environment
            var lightGO = new GameObject("Key Cybernetic Light");
            var light = lightGO.AddComponent<UnityEngine.Light>();
            light.type = LightType.Directional;
            light.color = new Color(0.8f, 0.95f, 1.0f);
            light.intensity = 1.1f;
            lightGO.transform.rotation = Quaternion.Euler(50f, -30f, 0f);

            // Hexagonal Command Floor
            var floor = GameObject.CreatePrimitive(PrimitiveType.Cylinder);
            floor.name = "CommandFloor";
            floor.transform.position = Vector3.zero;
            floor.transform.localScale = new Vector3(30f, 0.2f, 30f);
            floor.GetComponent<Renderer>().sharedMaterial = cyberMat;

            // Central Holographic Operations Pedestal
            var holoTable = GameObject.CreatePrimitive(PrimitiveType.Cylinder);
            holoTable.name = "HolographicTable";
            holoTable.transform.position = new Vector3(0, 0.5f, 0);
            holoTable.transform.localScale = new Vector3(6f, 0.5f, 6f);
            holoTable.GetComponent<Renderer>().sharedMaterial = hexMat;

            var holoBeam = GameObject.CreatePrimitive(PrimitiveType.Cylinder);
            holoBeam.name = "HologramBeam";
            holoBeam.transform.position = new Vector3(0, 2.5f, 0);
            holoBeam.transform.localScale = new Vector3(5.5f, 1.8f, 5.5f);
            holoBeam.GetComponent<Renderer>().sharedMaterial = glowMat;

            // Master Throne
            var throne = GameObject.CreatePrimitive(PrimitiveType.Cube);
            throne.name = "OwnerThrone";
            throne.transform.position = new Vector3(0, 1.0f, -8f);
            throne.transform.localScale = new Vector3(2.5f, 2.0f, 2.0f);
            throne.GetComponent<Renderer>().sharedMaterial = cyberMat;

            // Camera Setup
            var camGO = new GameObject("Main Camera");
            camGO.tag = "MainCamera";
            var cam = camGO.AddComponent<UnityEngine.Camera>();
            camGO.AddComponent<AudioListener>();
            camGO.transform.position = new Vector3(0, 8f, -16f);
            camGO.transform.rotation = Quaternion.Euler(28f, 0, 0);

            // HiveController & Subsystems
            var hiveGO = new GameObject("HiveController");
            var hive = hiveGO.AddComponent<HiveController>();
            hive.AgentWorkspacePrefab = wsPrefab;
            var wsContainer = new GameObject("WorkspaceContainer").transform;
            wsContainer.SetParent(hiveGO.transform);
            hive.WorkspaceContainer = wsContainer;

            // AdaptiveCamera
            var adCam = camGO.AddComponent<AdaptiveCamera>();
            adCam.CameraTransform = camGO.transform;

            // UI Canvas & Documents
            var uiGO = new GameObject("UI");
            uiGO.transform.SetParent(hiveGO.transform);

            var cmdGO = new GameObject("CommandInput");
            cmdGO.transform.SetParent(uiGO.transform);
            var cmdUIDoc = cmdGO.AddComponent<UIDocument>();
            cmdUIDoc.visualTreeAsset = AssetDatabase.LoadAssetAtPath<VisualTreeAsset>("Assets/Runtime/UI/Templates/CommandInputView.uxml");
            var cmdInput = cmdGO.AddComponent<CommandInput>();
            cmdInput.UIDocument = cmdUIDoc;
            cmdInput.CommandInputTemplate = cmdUIDoc.visualTreeAsset;
            cmdInput.MessageTemplate = AssetDatabase.LoadAssetAtPath<VisualTreeAsset>("Assets/Runtime/UI/Templates/MessageItemView.uxml");

            var navGO = new GameObject("NavSidebar");
            navGO.transform.SetParent(uiGO.transform);
            var navUIDoc = navGO.AddComponent<UIDocument>();
            navUIDoc.visualTreeAsset = AssetDatabase.LoadAssetAtPath<VisualTreeAsset>("Assets/Runtime/UI/Templates/NavSidebarView.uxml");
            var navSide = navGO.AddComponent<NavSidebar>();
            navSide.UIDocument = navUIDoc;
            navSide.SidebarTemplate = navUIDoc.visualTreeAsset;

            var notifGO = new GameObject("NotificationCenter");
            notifGO.transform.SetParent(uiGO.transform);
            var notifUIDoc = notifGO.AddComponent<UIDocument>();
            notifUIDoc.visualTreeAsset = AssetDatabase.LoadAssetAtPath<VisualTreeAsset>("Assets/Runtime/UI/Templates/NotificationView.uxml");
            notifGO.AddComponent<NotificationCenter>();

            EditorSceneManager.SaveScene(scene, $"{SCENES_DIR}/CommandChamber.unity");
        }

        private static void CreateWorkspacesScene(GameObject wsPrefab, Material cyberMat, Material glowMat)
        {
            var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            var floor = GameObject.CreatePrimitive(PrimitiveType.Cylinder);
            floor.name = "WorkspacesFloor";
            floor.transform.position = Vector3.zero;
            floor.transform.localScale = new Vector3(45f, 0.2f, 45f);
            floor.GetComponent<Renderer>().sharedMaterial = cyberMat;

            // Spawn 12 perimeter workspace desks
            float radius = 15f;
            for (int i = 0; i < 12; i++)
            {
                float angle = i * (360f / 12f) * Mathf.Deg2Rad;
                var pos = new Vector3(Mathf.Sin(angle) * radius, 0, Mathf.Cos(angle) * radius);
                var rot = Quaternion.LookRotation(-pos, Vector3.up);
                var pod = GameObject.Instantiate(wsPrefab, pos, rot);
                pod.name = $"WorkspaceNode_{i + 1}";
            }

            EditorSceneManager.SaveScene(scene, $"{SCENES_DIR}/Workspaces.unity");
        }

        private static void CreateCollabChamberScene(Material cyberMat, Material glowMat)
        {
            var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            var floor = GameObject.CreatePrimitive(PrimitiveType.Cylinder);
            floor.name = "CollabFloor";
            floor.transform.localScale = new Vector3(25f, 0.2f, 25f);
            floor.GetComponent<Renderer>().sharedMaterial = cyberMat;

            var projectionRing = GameObject.CreatePrimitive(PrimitiveType.Cylinder);
            projectionRing.name = "SynthesisRing";
            projectionRing.transform.position = new Vector3(0, 1.5f, 0);
            projectionRing.transform.localScale = new Vector3(8f, 0.1f, 8f);
            projectionRing.GetComponent<Renderer>().sharedMaterial = glowMat;

            EditorSceneManager.SaveScene(scene, $"{SCENES_DIR}/CollabChamber.unity");
        }

        private static void CreateKnowledgeArchiveScene(Material cyberMat, Material glowMat)
        {
            var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            var floor = GameObject.CreatePrimitive(PrimitiveType.Plane);
            floor.name = "ArchiveFloor";
            floor.transform.localScale = new Vector3(4f, 1f, 4f);
            floor.GetComponent<Renderer>().sharedMaterial = cyberMat;

            // 6 Glowing Data Stacks
            for (int i = 0; i < 6; i++)
            {
                var stack = GameObject.CreatePrimitive(PrimitiveType.Cube);
                stack.name = $"DataStack_{i + 1}";
                stack.transform.position = new Vector3(-10f + i * 4f, 3f, 5f);
                stack.transform.localScale = new Vector3(1.5f, 6f, 1.5f);
                stack.GetComponent<Renderer>().sharedMaterial = glowMat;
            }

            EditorSceneManager.SaveScene(scene, $"{SCENES_DIR}/KnowledgeArchive.unity");
        }

        private static void CreateInfrastructureScene(Material cyberMat, Material glowMat)
        {
            var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            var floor = GameObject.CreatePrimitive(PrimitiveType.Plane);
            floor.name = "InfraFloor";
            floor.transform.localScale = new Vector3(3f, 1f, 3f);
            floor.GetComponent<Renderer>().sharedMaterial = cyberMat;

            var serverRack = GameObject.CreatePrimitive(PrimitiveType.Cube);
            serverRack.name = "NIM_Cascade_Racks";
            serverRack.transform.position = new Vector3(0, 2f, 3f);
            serverRack.transform.localScale = new Vector3(8f, 4f, 2f);
            serverRack.GetComponent<Renderer>().sharedMaterial = glowMat;

            EditorSceneManager.SaveScene(scene, $"{SCENES_DIR}/Infrastructure.unity");
        }

        private static void CreateOwnerControlScene(Material cyberMat, Material glowMat)
        {
            var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            var floor = GameObject.CreatePrimitive(PrimitiveType.Cylinder);
            floor.name = "ControlFloor";
            floor.transform.localScale = new Vector3(15f, 0.2f, 15f);
            floor.GetComponent<Renderer>().sharedMaterial = cyberMat;

            var console = GameObject.CreatePrimitive(PrimitiveType.Cube);
            console.name = "EmergencyLockoutConsole";
            console.transform.position = new Vector3(0, 1f, 2f);
            console.transform.localScale = new Vector3(3f, 1.2f, 1.5f);
            console.GetComponent<Renderer>().sharedMaterial = glowMat;

            EditorSceneManager.SaveScene(scene, $"{SCENES_DIR}/OwnerControl.unity");
        }

        private static void CreateAtriumScene(Material cyberMat, Material glowMat)
        {
            var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            var floor = GameObject.CreatePrimitive(PrimitiveType.Cylinder);
            floor.name = "AtriumFloor";
            floor.transform.localScale = new Vector3(50f, 0.2f, 50f);
            floor.GetComponent<Renderer>().sharedMaterial = cyberMat;

            // Spire
            var spire = GameObject.CreatePrimitive(PrimitiveType.Cylinder);
            spire.name = "BorgCentralSpire";
            spire.transform.position = new Vector3(0, 10f, 0);
            spire.transform.localScale = new Vector3(2f, 20f, 2f);
            spire.GetComponent<Renderer>().sharedMaterial = glowMat;

            EditorSceneManager.SaveScene(scene, $"{SCENES_DIR}/Atrium.unity");
        }

        private static void ConfigureBuildSettings()
        {
            var sceneNames = new[]
            {
                "CommandChamber",
                "Workspaces",
                "CollabChamber",
                "KnowledgeArchive",
                "Infrastructure",
                "OwnerControl",
                "Atrium"
            };

            var editorBuildSettingsScenes = sceneNames.Select(s => new EditorBuildSettingsScene($"{SCENES_DIR}/{s}.unity", true)).ToArray();
            EditorBuildSettings.scenes = editorBuildSettingsScenes;
            Debug.Log($"[HiveSceneBuilder] Configured {editorBuildSettingsScenes.Length} scenes in EditorBuildSettings.");
        }
    }
}
