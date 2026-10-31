using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;
using UnityEditor;
using UnityEngine;

namespace HIVE.Editor.ModelLab
{
    /// <summary>
    /// Model Lab: Benchmarks NIM models daily/weekly and writes cascade.json
    /// Ported from StickExplainers model_lab.py
    /// </summary>
    public class ModelLab : EditorWindow
    {
        private const string NIM_MODELS_URL = "https://integrate.api.nvidia.com/v1/models";
        private const string NIM_CHAT_URL = "https://integrate.api.nvidia.com/v1/chat/completions";
        private const string CASCADE_PATH = "Assets/StreamingAssets/cascade.json";
        private const string CONFIG_PATH = "Assets/StreamingAssets/config.json";

        private static readonly HttpClient HttpClient = new HttpClient { Timeout = TimeSpan.FromMinutes(5) };

        private Vector2 _scroll;
        private string _logOutput = "";
        private bool _running;
        private List<string> _availableModels = new();
        private List<string> _selectedModels = new();
        private int _maxConcurrent = 4;
        private int _testTokens = 100;

        [MenuItem("HIVE/Model Lab")]
        public static void ShowWindow()
        {
            GetWindow<ModelLab>("HIVE Model Lab");
        }

        private void OnGUI()
        {
            _scroll = EditorGUILayout.BeginScrollView(_scroll);

            EditorGUILayout.LabelField("HIVE Model Lab", EditorStyles.boldLabel);
            EditorGUILayout.Space();

            EditorGUILayout.LabelField("NVIDIA NIM Model Benchmarking", EditorStyles.miniBoldLabel);
            EditorGUILayout.HelpBox("Fetches available NIM models, runs benchmark prompts, ranks by quality/speed/reliability, and writes cascade.json for the NimCascadeRouter.", MessageType.Info);

            EditorGUILayout.Space();

            _maxConcurrent = EditorGUILayout.IntSlider("Max Concurrent Requests", _maxConcurrent, 1, 16);
            _testTokens = EditorGUILayout.IntSlider("Test Tokens per Model", _testTokens, 50, 500);

            EditorGUILayout.Space();

            using (new EditorGUI.DisabledScope(_running))
            {
                if (GUILayout.Button("Fetch Available NIM Models", GUILayout.Height(30)))
                {
                    _ = FetchModelsAsync();
                }
            }

            if (_availableModels.Count > 0)
            {
                EditorGUILayout.Space();
                EditorGUILayout.LabelField($"Available Models ({_availableModels.Count})", EditorStyles.boldLabel);

                using (new EditorGUI.DisabledScope(_running))
                {
                    if (GUILayout.Button(_running ? "Running Benchmark..." : "Run Benchmark on All Models", GUILayout.Height(30)))
                    {
                        _ = RunBenchmarkAsync();
                    }
                }

                EditorGUILayout.Space();
                EditorGUILayout.LabelField("Select Models for Cascade:", EditorStyles.miniBoldLabel);
                foreach (var model in _availableModels.OrderBy(m => m))
                {
                    var selected = _selectedModels.Contains(model);
                    var newSelected = EditorGUILayout.ToggleLeft(model, selected);
                    if (newSelected && !selected) _selectedModels.Add(model);
                    else if (!newSelected && selected) _selectedModels.Remove(model);
                }
            }

            EditorGUILayout.Space();
            EditorGUILayout.LabelField("Log Output:", EditorStyles.boldLabel);
            var logStyle = new GUIStyle(EditorStyles.textArea) { wordWrap = true, fontSize = 10 };
            EditorGUILayout.TextArea(_logOutput, logStyle, GUILayout.Height(300), GUILayout.ExpandHeight(true));

            EditorGUILayout.EndScrollView();
        }

        private async Task FetchModelsAsync()
        {
            _running = true;
            _logOutput = "";
            Log("Fetching NIM models list...");

            try
            {
                var key = GetEnv("NVIDIA_API_KEY");
                if (string.IsNullOrEmpty(key))
                {
                    Log("ERROR: NVIDIA_API_KEY not set in environment");
                    _running = false;
                    Repaint();
                    return;
                }

                using var request = new HttpRequestMessage(HttpMethod.Get, NIM_MODELS_URL);
                request.Headers.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", key);
                var response = await HttpClient.SendAsync(request);
                var json = await response.Content.ReadAsStringAsync();

                if (!response.IsSuccessStatusCode)
                {
                    Log($"ERROR: {(int)response.StatusCode} - {json}");
                    _running = false;
                    Repaint();
                    return;
                }

                var doc = JsonDocument.Parse(json);
                _availableModels = doc.RootElement.GetProperty("data").EnumerateArray()
                    .Select(m => m.GetProperty("id").GetString())
                    .Where(s => !string.IsNullOrEmpty(s))
                    .OrderBy(s => s)
                    .ToList();

                _selectedModels = _availableModels.ToList();
                Log($"Found {_availableModels.Count} models");
            }
            catch (Exception e)
            {
                Log($"EXCEPTION: {e.Message}");
            }
            finally
            {
                _running = false;
                Repaint();
            }
        }

        private async Task RunBenchmarkAsync()
        {
            if (_selectedModels.Count == 0)
            {
                Log("No models selected");
                return;
            }

            _running = true;
            _logOutput = "";
            Log($"Starting benchmark on {_selectedModels.Count} models...");

            var results = new Dictionary<string, ModelBenchmark>();
            var semaphore = new System.Threading.SemaphoreSlim(_maxConcurrent);

            var tasks = _selectedModels.Select(async model =>
            {
                await semaphore.WaitAsync();
                try
                {
                    var bench = await BenchmarkModel(model);
                    lock (results)
                    {
                        results[model] = bench;
                    }
                    Log($"✓ {model}: q={bench.Quality:F3}, rel={bench.Reliability:F3}, speed={bench.SpeedTokensPerSec:F1} tok/s");
                }
                catch (Exception e)
                {
                    Log($"✗ {model}: {e.Message}");
                    lock (results)
                    {
                        results[model] = new ModelBenchmark { Quality = 0, Reliability = 0, SpeedTokensPerSec = 0, VisionQuality = 0, Error = e.Message };
                    }
                }
                finally
                {
                    semaphore.Release();
                }
            });

            await Task.WhenAll(tasks);

            WriteCascade(results);
            Log("Benchmark complete. cascade.json updated.");
            _running = false;
            Repaint();
        }

        private async Task<ModelBenchmark> BenchmarkModel(string model)
        {
            var key = GetEnv("NVIDIA_API_KEY");
            if (string.IsNullOrEmpty(key)) throw new InvalidOperationException("NVIDIA_API_KEY not set");

            // Test prompts for quality assessment
            var prompts = new[]
            {
                "Explain quantum entanglement in one paragraph.",
                "Write a Python function to find the longest palindromic substring.",
                "What are the trade-offs between consistency and availability in distributed systems?"
            };

            var times = new List<double>();
            var successes = 0;
            var qualityScores = new List<double>();

            foreach (var prompt in prompts)
            {
                var body = new
                {
                    model = model,
                    messages = new[] { new { role = "user", content = prompt } },
                    max_tokens = _testTokens,
                    temperature = 0.6,
                    chat_template_kwargs = new { enable_thinking = false }
                };

                var jsonBody = JsonSerializer.Serialize(body);
                using var request = new HttpRequestMessage(HttpMethod.Post, NIM_CHAT_URL);
                request.Headers.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", key);
                request.Content = new StringContent(jsonBody, System.Text.Encoding.UTF8, "application/json");

                var t0 = DateTime.UtcNow;
                try
                {
                    var response = await HttpClient.SendAsync(request);
                    var elapsed = (DateTime.UtcNow - t0).TotalSeconds;
                    var responseJson = await response.Content.ReadAsStringAsync();

                    if (response.IsSuccessStatusCode)
                    {
                        var chatDoc = JsonDocument.Parse(responseJson);
                        var content = chatDoc.RootElement.GetProperty("choices")[0].GetProperty("message").GetProperty("content").GetString() ?? "";
                        if (!string.IsNullOrWhiteSpace(content))
                        {
                            successes++;
                            times.Add(elapsed);
                            // Simple quality heuristic: length + structure
                            var quality = Math.Min(1.0, content.Length / 200.0);
                            if (content.Contains("```") || content.Contains("\n\n")) quality += 0.1;
                            qualityScores.Add(Math.Min(1.0, quality));
                        }
                    }
                    else if ((int)response.StatusCode == 400)
                    {
                        // Retry without chat_template_kwargs
                        body = new
                        {
                            model = model,
                            messages = new[] { new { role = "user", content = prompt } },
                            max_tokens = _testTokens,
                            temperature = 0.6
                        };
                        jsonBody = JsonSerializer.Serialize(body);
                        request.Content = new StringContent(jsonBody, System.Text.Encoding.UTF8, "application/json");
                        response = await HttpClient.SendAsync(request);
                        responseJson = await response.Content.ReadAsStringAsync();

                        if (response.IsSuccessStatusCode)
                        {
                            var chatDoc = JsonDocument.Parse(responseJson);
                            var content = chatDoc.RootElement.GetProperty("choices")[0].GetProperty("message").GetProperty("content").GetString() ?? "";
                            if (!string.IsNullOrWhiteSpace(content))
                            {
                                successes++;
                                times.Add((DateTime.UtcNow - t0).TotalSeconds);
                                var quality = Math.Min(1.0, content.Length / 200.0);
                                qualityScores.Add(Math.Min(1.0, quality));
                            }
                        }
                    }
                }
                catch { }
            }

            var avgSpeed = times.Count > 0 ? _testTokens / times.Average() : 0;
            var reliability = (double)successes / prompts.Length;
            var avgQuality = qualityScores.Count > 0 ? qualityScores.Average() : 0;
            var visionQuality = model.Contains("vision") || model.Contains("vl") ? 1.0 : 0.0;

            return new ModelBenchmark
            {
                Quality = avgQuality,
                Reliability = reliability,
                SpeedTokensPerSec = avgSpeed,
                VisionQuality = visionQuality
            };
        }

        private void WriteCascade(Dictionary<string, ModelBenchmark> results)
        {
            var textModels = results
                .Where(kvp => kvp.Value.Quality > 0 && !kvp.Key.Contains("vision") && !kvp.Key.Contains("vl"))
                .OrderByDescending(kvp => kvp.Value.Quality * 0.5 + kvp.Value.Reliability * 0.3 + Math.Min(1.0, kvp.Value.SpeedTokensPerSec / 50.0) * 0.2)
                .Select(kvp => kvp.Key)
                .ToList();

            var visionModels = results
                .Where(kvp => kvp.Value.Quality > 0 && (kvp.Key.Contains("vision") || kvp.Key.Contains("vl")))
                .OrderByDescending(kvp => kvp.Value.VisionQuality * 0.5 + kvp.Value.Quality * 0.3 + kvp.Value.Reliability * 0.2)
                .Select(kvp => kvp.Key)
                .ToList();

            // Load existing dead list
            var existingDead = new List<string>();
            if (File.Exists(CASCADE_PATH))
            {
                try
                {
                    var existing = JsonSerializer.Deserialize<CascadeData>(File.ReadAllText(CASCADE_PATH));
                    if (existing?.dead != null) existingDead = existing.dead;
                }
                catch { }
            }

            var cascade = new CascadeData
            {
                at = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
                text = textModels,
                vision = visionModels,
                dead = existingDead,
                scores = results.Where(kvp => kvp.Value.Quality > 0)
                    .ToDictionary(
                        kvp => kvp.Key,
                        kvp => new ModelScore
                        {
                            q = kvp.Value.Quality,
                            rel = kvp.Value.Reliability,
                            s = kvp.Value.SpeedTokensPerSec,
                            vq = kvp.Value.VisionQuality
                        })
            };

            var json = JsonSerializer.Serialize(cascade, new JsonSerializerOptions { WriteIndented = true });
            File.WriteAllText(CASCADE_PATH, json);
            AssetDatabase.Refresh();
            Log($"Written cascade: {textModels.Count} text, {visionModels.Count} vision models");
        }

        private string GetEnv(string name)
        {
            var v = Environment.GetEnvironmentVariable(name);
            if (string.IsNullOrEmpty(v) && OperatingSystem.IsWindows())
            {
                try
                {
                    using var key = Microsoft.Win32.Registry.CurrentUser.OpenSubKey("Environment");
                    v = key?.GetValue(name) as string;
                }
                catch { }
            }
            return v;
        }

        private void Log(string msg)
        {
            _logOutput += $"[{DateTime.Now:HH:mm:ss}] {msg}\n";
            Debug.Log($"[ModelLab] {msg}");
        }

        private class ModelBenchmark
        {
            public double Quality { get; set; }
            public double Reliability { get; set; }
            public double SpeedTokensPerSec { get; set; }
            public double VisionQuality { get; set; }
            public string Error { get; set; }
        }

        private class CascadeData
        {
            public double at { get; set; }
            public List<string> text { get; set; }
            public List<string> vision { get; set; }
            public List<string> dead { get; set; }
            public Dictionary<string, ModelScore> scores { get; set; }
        }

        private class ModelScore
        {
            public double q { get; set; }
            public double rel { get; set; }
            public double s { get; set; }
            public double vq { get; set; }
        }
    }
}