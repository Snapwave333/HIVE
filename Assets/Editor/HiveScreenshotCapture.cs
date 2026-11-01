using System;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using HIVE.Camera;

namespace HIVE.Editor
{
    /// <summary>
    /// Headless and Editor screenshot utility for H.I.V.E. operational 3D scenes.
    /// Supports batchmode CLI invocations, custom resolutions (e.g. 1080p, 4K),
    /// multi-camera modes, and automated asset generation.
    /// </summary>
    public static class HiveScreenshotCapture
    {
        public static readonly string[] OperationalScenes = new[]
        {
            "CommandChamber",
            "Workspaces",
            "CollabChamber",
            "KnowledgeArchive",
            "Infrastructure",
            "OwnerControl",
            "Atrium"
        };

        [MenuItem("HIVE/Capture Screenshots/All Scenes (1080p)")]
        public static void CaptureAll1080p()
        {
            CaptureAll(1920, 1080, "Screenshots");
        }

        [MenuItem("HIVE/Capture Screenshots/Command Chamber (4K)")]
        public static void CaptureCommandChamber4K()
        {
            CaptureScene("CommandChamber", 3840, 2160, "Screenshots");
        }

        /// <summary>
        /// Command line entry point for headless batchmode capture.
        /// Invoked via: Unity.exe -batchmode -quit -projectPath . -executeMethod HIVE.Editor.HiveScreenshotCapture.CaptureCLI -scene all -res 1920x1080 -out Screenshots
        /// </summary>
        public static void CaptureCLI()
        {
            Debug.Log("[HiveScreenshotCapture] Initializing Headless Screenshot CLI...");
            var args = Environment.GetCommandLineArgs();

            string sceneArg = GetArg(args, "-scene", "all");
            string resArg = GetArg(args, "-res", "1920x1080");
            string outDir = GetArg(args, "-out", "Screenshots");
            string modeArg = GetArg(args, "-cameraMode", null);

            var (width, height) = ParseResolution(resArg);
            AdaptiveCamera.CameraMode? cameraMode = null;
            if (!string.IsNullOrEmpty(modeArg) && Enum.TryParse<AdaptiveCamera.CameraMode>(modeArg, true, out var parsedMode))
            {
                cameraMode = parsedMode;
            }

            try
            {
                if (sceneArg.Equals("all", StringComparison.OrdinalIgnoreCase))
                {
                    CaptureAll(width, height, outDir, cameraMode);
                }
                else
                {
                    CaptureScene(sceneArg, width, height, outDir, cameraMode);
                }

                Debug.Log("[HiveScreenshotCapture] Headless capture completed successfully.");
                EditorApplication.Exit(0);
            }
            catch (Exception ex)
            {
                Debug.LogError($"[HiveScreenshotCapture] Capture failed: {ex}");
                EditorApplication.Exit(1);
            }
        }

        public static void CaptureAll(int width, int height, string outDir, AdaptiveCamera.CameraMode? cameraMode = null)
        {
            EnsureDirectory(outDir);
            Debug.Log($"[HiveScreenshotCapture] Capturing all {OperationalScenes.Length} operational scenes at {width}x{height} to '{outDir}'...");

            foreach (var scene in OperationalScenes)
            {
                CaptureScene(scene, width, height, outDir, cameraMode);
            }
        }

        public static string CaptureScene(string sceneName, int width, int height, string outDir, AdaptiveCamera.CameraMode? cameraMode = null)
        {
            EnsureDirectory(outDir);
            var scenePath = $"Assets/Scenes/{sceneName}.unity";
            if (!File.Exists(scenePath))
            {
                throw new FileNotFoundException($"Scene asset not found at {scenePath}");
            }

            var scene = EditorSceneManager.OpenScene(scenePath, OpenSceneMode.Single);

            // Locate or construct camera
            var cam = UnityEngine.Camera.main;
            if (cam == null)
            {
                cam = UnityEngine.Object.FindFirstObjectByType<UnityEngine.Camera>();
            }

            if (cam == null)
            {
                var camGO = new GameObject("Fallback_Capture_Camera");
                cam = camGO.AddComponent<UnityEngine.Camera>();
                cam.transform.position = new Vector3(0, 10, -20);
                cam.transform.rotation = Quaternion.Euler(25, 0, 0);
            }

            // Apply camera mode if requested and component is available
            var adCam = cam.GetComponent<AdaptiveCamera>();
            if (adCam != null && cameraMode.HasValue)
            {
                adCam.SetMode(cameraMode.Value, true);
            }

            // Create target render texture with depth buffer
            var rt = new RenderTexture(width, height, 24, RenderTextureFormat.ARGB32)
            {
                antiAliasing = 4
            };

            var prevActive = RenderTexture.active;
            var prevTarget = cam.targetTexture;

            cam.targetTexture = rt;
            cam.Render();

            RenderTexture.active = rt;
            var tex = new Texture2D(width, height, TextureFormat.RGB24, false);
            tex.ReadPixels(new Rect(0, 0, width, height), 0, 0);
            tex.Apply();

            cam.targetTexture = prevTarget;
            RenderTexture.active = prevActive;
            UnityEngine.Object.DestroyImmediate(rt);

            byte[] pngBytes = ImageConversion.EncodeToPNG(tex);
            UnityEngine.Object.DestroyImmediate(tex);

            var suffix = cameraMode.HasValue ? $"_{cameraMode.Value.ToString().ToLowerInvariant()}" : "";
            var fileName = $"{sceneName}{suffix}.png";
            var fullPath = Path.Combine(outDir, fileName);

            File.WriteAllBytes(fullPath, pngBytes);
            Debug.Log($"[HiveScreenshotCapture] [OK] Captured {sceneName} -> {fullPath} ({width}x{height}, {pngBytes.Length / 1024} KB)");

            return fullPath;
        }

        private static (int width, int height) ParseResolution(string res)
        {
            if (string.IsNullOrWhiteSpace(res)) return (1920, 1080);
            var parts = res.ToLowerInvariant().Split('x');
            if (parts.Length == 2 && int.TryParse(parts[0], out var w) && int.TryParse(parts[1], out var h))
            {
                return (Math.Max(320, w), Math.Max(240, h));
            }
            return (1920, 1080);
        }

        private static string GetArg(string[] args, string flag, string defaultValue)
        {
            for (int i = 0; i < args.Length - 1; i++)
            {
                if (args[i].Equals(flag, StringComparison.OrdinalIgnoreCase))
                {
                    return args[i + 1];
                }
            }
            return defaultValue;
        }

        private static void EnsureDirectory(string path)
        {
            if (!Directory.Exists(path)) Directory.CreateDirectory(path);
        }
    }
}
