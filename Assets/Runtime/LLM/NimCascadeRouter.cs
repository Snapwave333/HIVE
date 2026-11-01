using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net.Http;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading;
using System.Threading.Tasks;
using UnityEngine;

namespace HIVE.LLM
{
    /// <summary>
    /// LLM Router: Claude while credits last, NVIDIA NIM after that, forever.
    /// Ported from StickExplainers stickfx/llm.py
    /// </summary>
    public static class NimCascadeRouter
    {
        private static readonly HttpClient HttpClient = new HttpClient { Timeout = TimeSpan.FromMinutes(30) };
        private static readonly string StatePath = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
            ".hive", "llm_state.json");
        private static readonly string CascadePath = Path.Combine(Application.streamingAssetsPath, "cascade.json");
        private static readonly string ConfigPath = Path.Combine(Application.streamingAssetsPath, "config.json");

        private const string NIM_URL = "https://integrate.api.nvidia.com/v1/chat/completions";
        private const string NIM_MODELS_URL = "https://integrate.api.nvidia.com/v1/models";
        private const string GEMINI_URL = "https://generativelanguage.googleapis.com/v1beta/models";
        private const string ANTHROPIC_API_URL = "https://api.anthropic.com";

        private static readonly string[] NIM_TEXT = new[]
        {
            "nvidia/nemotron-3-ultra-550b-a55b",
            "nvidia/nemotron-3.5-super-vl-120b-a12b",
            "mistralai/mistral-large-4-1t-a52b",
            "minimaxai/minimax-m3.1-flash",
            "deepseek-ai/deepseek-v4.1-flash",
            "z-ai/glm-5.3-flash",
            "stepfun-ai/step-5-preview",
            "openai/gpt-oss-20b",
            "nvidia/nemotron-3.5-lightning-30b-a3b",
            "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning",
            "nvidia/nemotron-3.5-bolt-e4b"
        };

        private static readonly string[] NIM_VISION = new[]
        {
            "nvidia/nemotron-3.5-super-vl-120b-a12b",
            "meta/llama-3.2-90b-vision-instruct",
            "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning"
        };

        private static readonly string[] GEMINI_MODELS = new[]
        {
            "gemini-flash-latest",
            "gemini-flash-lite-latest"
        };

        private static readonly Dictionary<string, string> MODEL_FALLBACK = new()
        {
            { "claude-opus-5-5", "claude-sonnet-5-5" },
            { "claude-sonnet-5-5", "claude-haiku-5-5" }
        };

        private static readonly string[] CREDIT_HINTS = new[]
        {
            "credit balance", "billing", "insufficient", "payment", "quota", "exceeded your", "plan limit"
        };

        private const long RECHECK_SECONDS = 6 * 3600;
        private const long KEY_RECHECK_SECONDS = 3600;
        private const long HEALTH_SECONDS = 3600;
        private const long BLOCK_SECONDS = 24 * 3600;
        private const int THINK_MIN_TOKENS = 4000;
        private const int REASONING_ROOM = 2048;
        private static readonly int[] COOLDOWN_SECONDS = new[] { 120, 600, 1800 };
        private const int SLOW_SECONDS = 90;

        private static readonly Dictionary<string, object> NIM_EXTRA = new()
        {
            { "openai/gpt-oss-20b", new Dictionary<string, object> { { "short", new { reasoning_effort = "low" } } } }
        };

        private static LlmState _state;
        private static readonly object _stateLock = new object();

        private class LlmState
        {
            public double? claude_out_at { get; set; }
            public string claude_out_kind { get; set; }
            public double? claude_probe_at { get; set; }
            public string reason { get; set; }
            public Dictionary<string, ModelHealth> nim_health { get; set; }
            public Dictionary<string, double> claude_blocked { get; set; }
            public double? claude_flaky_until { get; set; }
            public ClaudeHealth claude_health { get; set; }
            public double? cascade_alert_at { get; set; }
            public List<string> nim_models { get; set; }
            public double? nim_models_at { get; set; }
        }

        private class ModelHealth
        {
            public int fails { get; set; }
            public double until { get; set; }
            public double last_ok { get; set; }
            public double last_fail { get; set; }
            public string why { get; set; }
            public double slow_until { get; set; }
        }

        public class ClaudeHealth
        {
            public bool ok { get; set; }
            public int status { get; set; }
            public string detail { get; set; }
            public double at { get; set; }
            public List<string> models { get; set; }
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

        private class NimModelsResponse
        {
            public List<NimModel> data { get; set; }
        }

        private class NimModel
        {
            public string id { get; set; }
        }

        private class ChatRequest
        {
            public string model { get; set; }
            public List<ChatMessage> messages { get; set; }
            public int max_tokens { get; set; }
            public double temperature { get; set; }
            public Dictionary<string, object> chat_template_kwargs { get; set; }
        }

        private class ChatMessage
        {
            public string role { get; set; }
            public object content { get; set; }
        }

        private class ChatResponse
        {
            public List<Choice> choices { get; set; }
            public Usage usage { get; set; }
        }

        private class Choice
        {
            public Message message { get; set; }
            public string finish_reason { get; set; }
        }

        private class Message
        {
            public string content { get; set; }
            public string reasoning_content { get; set; }
            public string reasoning { get; set; }
        }

        private class Usage
        {
            public int prompt_tokens { get; set; }
            public int completion_tokens { get; set; }
            public int cache_read_input_tokens { get; set; }
            public int cache_creation_input_tokens { get; set; }
        }

        static NimCascadeRouter()
        {
            LoadState();
        }

        private static void LoadState()
        {
            lock (_stateLock)
            {
                try
                {
                    if (File.Exists(StatePath))
                    {
                        var json = File.ReadAllText(StatePath);
                        _state = JsonSerializer.Deserialize<LlmState>(json) ?? new LlmState();
                    }
                    else
                    {
                        _state = new LlmState();
                    }
                }
                catch
                {
                    _state = new LlmState();
                }
                EnsureDirectories();
            }
        }

        private static void EnsureDirectories()
        {
            var dir = Path.GetDirectoryName(StatePath);
            if (!string.IsNullOrEmpty(dir) && !Directory.Exists(dir))
                Directory.CreateDirectory(dir);
        }

        private static void SaveState()
        {
            lock (_stateLock)
            {
                EnsureDirectories();
                var json = JsonSerializer.Serialize(_state, new JsonSerializerOptions { WriteIndented = true });
                File.WriteAllText(StatePath, json);
            }
        }

        private static string GetEnv(string name)
        {
            var v = Environment.GetEnvironmentVariable(name);
            if (string.IsNullOrEmpty(v))
            {
                try
                {
                    v = Environment.GetEnvironmentVariable(name, EnvironmentVariableTarget.User);
                    if (string.IsNullOrEmpty(v))
                        v = Environment.GetEnvironmentVariable(name, EnvironmentVariableTarget.Machine);
                }
                catch { }
            }
            return v;
        }

        /// <summary>
        /// Get active provider: "claude" or "nim"
        /// </summary>
        public static string GetProvider()
        {
            var overrideProvider = Environment.GetEnvironmentVariable("HIVE_LLM");
            if (overrideProvider == "nim" || overrideProvider == "claude")
                return overrideProvider;

            if (string.IsNullOrEmpty(GetEnv("ANTHROPIC_API_KEY")))
                return "nim";

            if (_state.claude_out_at.HasValue)
            {
                var kind = _state.claude_out_kind ?? "credits";
                var wait = kind == "key" ? KEY_RECHECK_SECONDS : RECHECK_SECONDS;
                var probeAt = _state.claude_probe_at ?? _state.claude_out_at.Value;
                if (DateTimeOffset.UtcNow.ToUnixTimeSeconds() - probeAt < wait)
                    return "nim";

                var (ok, detail) = System.Threading.Tasks.Task.Run(() => ProbeClaude(kind)).GetAwaiter().GetResult();
                if (!ok)
                {
                    _state.claude_probe_at = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
                    SaveState();
                    return "nim";
                }

                _state.claude_out_at = null;
                _state.claude_out_kind = null;
                _state.claude_probe_at = null;
                _state.reason = null;
                SaveState();
                Log($"Claude is back ({(kind == "key" ? "key works again" : "credits are back")}); using Claude again");
            }
            return "claude";
        }

        private static async Task<(bool ok, string detail)> ProbeClaude(string kind)
        {
            var apiKey = GetEnv("ANTHROPIC_API_KEY");
            if (string.IsNullOrEmpty(apiKey)) return (false, "no key");

            using var request = new HttpRequestMessage(HttpMethod.Get, $"{ANTHROPIC_API_URL}/v1/models?limit=1");
            request.Headers.Add("x-api-key", apiKey);
            request.Headers.Add("anthropic-version", "2023-06-01");

            try
            {
                if (kind == "credits")
                {
                    request.Method = HttpMethod.Post;
                    request.RequestUri = new Uri($"{ANTHROPIC_API_URL}/v1/messages");
                    request.Content = new StringContent(JsonSerializer.Serialize(new
                    {
                        model = "claude-haiku-5-5",
                        max_tokens = 1,
                        messages = new[] { new { role = "user", content = "ok" } }
                    }), Encoding.UTF8, "application/json");
                }

                var response = await HttpClient.SendAsync(request);
                return (response.IsSuccessStatusCode, response.IsSuccessStatusCode ? "ok" : await response.Content.ReadAsStringAsync());
            }
            catch (Exception e)
            {
                return (false, e.Message);
            }
        }

        /// <summary>
        /// Health check for Control Room (cached)
        /// </summary>
        public static async Task<ClaudeHealth> GetHealthAsync(long maxAgeSeconds = HEALTH_SECONDS)
        {
            if (_state.claude_health != null &&
                DateTimeOffset.UtcNow.ToUnixTimeSeconds() - _state.claude_health.at < maxAgeSeconds)
                return _state.claude_health;

            var apiKey = GetEnv("ANTHROPIC_API_KEY");
            if (string.IsNullOrEmpty(apiKey))
            {
                _state.claude_health = new ClaudeHealth { ok = false, status = 0, detail = "ANTHROPIC_API_KEY not set", at = DateTimeOffset.UtcNow.ToUnixTimeSeconds(), models = new List<string>() };
            }
            else
            {
                using var request = new HttpRequestMessage(HttpMethod.Get, $"{ANTHROPIC_API_URL}/v1/models?limit=20");
                request.Headers.Add("x-api-key", apiKey);
                request.Headers.Add("anthropic-version", "2023-06-01");

                try
                {
                    var response = await HttpClient.SendAsync(request);
                    var json = await response.Content.ReadAsStringAsync();
                    if (response.IsSuccessStatusCode)
                    {
                        var doc = JsonDocument.Parse(json);
                        var models = doc.RootElement.GetProperty("data").EnumerateArray()
                            .Select(m => m.GetProperty("id").GetString())
                            .Where(s => !string.IsNullOrEmpty(s))
                            .Take(8)
                            .ToList();
                        _state.claude_health = new ClaudeHealth { ok = true, status = 200, detail = "key accepted", at = DateTimeOffset.UtcNow.ToUnixTimeSeconds(), models = models };
                    }
                    else
                    {
                        var err = JsonDocument.Parse(json);
                        _state.claude_health = new ClaudeHealth { ok = false, status = (int)response.StatusCode, detail = err.RootElement.GetProperty("error").GetProperty("message").GetString()?.Substring(0, Math.Min(160, err.RootElement.GetProperty("error").GetProperty("message").GetString()?.Length ?? 0)) ?? "", at = DateTimeOffset.UtcNow.ToUnixTimeSeconds(), models = new List<string>() };
                    }
                }
                catch (Exception e)
                {
                    _state.claude_health = new ClaudeHealth { ok = false, status = 0, detail = e.Message, at = DateTimeOffset.UtcNow.ToUnixTimeSeconds(), models = new List<string>() };
                }
            }
            SaveState();
            return _state.claude_health;
        }

        private static void Log(string msg)
        {
            var logPath = Path.Combine(Path.GetDirectoryName(StatePath), "llm.log");
            EnsureDirectories();
            File.AppendAllText(logPath, $"{DateTime.Now:yyyy-MM-dd HH:mm:ss} {msg}\n");

            if (msg.Contains("Claude marked out") || msg.Contains("Claude is back") || msg.Contains("fast-fail"))
            {
                try { NotifyEvent(msg); } catch { }
            }
        }

        private static void NotifyEvent(string msg)
        {
            // TODO: Integrate with notification system (Telegram, etc.)
            Debug.Log($"[HIVE LLM] {msg}");
        }

        private static void MarkClaudeOut(string reason, string kind = "credits")
        {
            _state.claude_out_at = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            _state.claude_out_kind = kind;
            _state.reason = reason.Substring(0, Math.Min(300, reason.Length));
            _state.claude_probe_at = null;
            _state.claude_health = null;
            SaveState();

            if (kind == "key")
                Log($"Claude marked out: Anthropic API key rejected ({reason.Substring(0, Math.Min(160, reason.Length))}). NIM writes until key fixed; checked hourly.");
            else
                Log($"Claude marked out of credits, switching to NIM: {reason.Substring(0, Math.Min(200, reason.Length))} (checked every {RECHECK_SECONDS / 3600}h)");
        }

        private static string GetBlockedModel(string model)
        {
            var now = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            var seen = new HashSet<string>();
            var blocked = _state.claude_blocked ?? new Dictionary<string, double>();

            while (blocked.TryGetValue(model, out var until) && until > now && seen.Add(model))
            {
                if (MODEL_FALLBACK.TryGetValue(model, out var fallback))
                    model = fallback;
                else
                    break;
            }
            return model;
        }

        private static void BlockModel(string model, string why)
        {
            var now = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            var blocked = _state.claude_blocked ?? new Dictionary<string, double>();
            blocked = blocked.Where(kvp => kvp.Value > now).ToDictionary(kvp => kvp.Key, kvp => kvp.Value);
            blocked[model] = now + BLOCK_SECONDS;
            _state.claude_blocked = blocked;
            SaveState();
            Log($"Claude model {model} refused for this key ({why.Substring(0, Math.Min(100, why.Length))}); using {MODEL_FALLBACK.GetValueOrDefault(model)} for a day");
        }

        private static async Task<HashSet<string>> GetAvailableNimModelsAsync(string key)
        {
            if (_state.nim_models != null && _state.nim_models_at.HasValue &&
                DateTimeOffset.UtcNow.ToUnixTimeSeconds() - _state.nim_models_at.Value < 24 * 3600)
                return _state.nim_models.ToHashSet();

            try
            {
                using var request = new HttpRequestMessage(HttpMethod.Get, NIM_MODELS_URL);
                request.Headers.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", key);
                var response = await HttpClient.SendAsync(request);
                var json = await response.Content.ReadAsStringAsync();
                var doc = JsonDocument.Parse(json);
                var ids = doc.RootElement.GetProperty("data").EnumerateArray()
                    .Select(m => m.GetProperty("id").GetString())
                    .Where(s => !string.IsNullOrEmpty(s))
                    .ToHashSet();
                _state.nim_models = ids.ToList();
                _state.nim_models_at = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
                SaveState();
                return ids;
            }
            catch
            {
                return null;
            }
        }

        private static string StripThink(string txt)
        {
            if (string.IsNullOrEmpty(txt)) return txt;
            txt = System.Text.RegularExpressions.Regex.Replace(txt, @"\<think\>.*?\<\/think\>", "", System.Text.RegularExpressions.RegexOptions.Singleline);
            var idx = txt.LastIndexOf("</think>");
            if (idx >= 0) txt = txt.Substring(idx + 8);
            return txt.Trim();
        }

        private static async Task<string> CallNimAsync(string system, object content, int maxTokens, string[] models, bool vision = false, double? deadline = null)
        {
            var key = GetEnv("NVIDIA_API_KEY");
            if (string.IsNullOrEmpty(key)) throw new InvalidOperationException("NVIDIA_API_KEY not set");

            var liveModels = await GetAvailableNimModelsAsync(key);
            if (liveModels != null)
            {
                var filtered = models.Where(m => liveModels.Contains(m)).ToArray();
                if (filtered.Length > 0) models = filtered;
            }

            var messages = new List<ChatMessage>();
            if (!string.IsNullOrEmpty(system))
                messages.Add(new ChatMessage { role = "system", content = system });

            if (content is string str)
                messages.Add(new ChatMessage { role = "user", content = str });
            else if (content is List<ChatMessage> list)
                messages.AddRange(list);

            var tokens = Math.Min(maxTokens, 16000);
            var shortCall = tokens < THINK_MIN_TOKENS;
            var deadlineTime = deadline ?? DateTimeOffset.UtcNow.AddMinutes(shortCall ? 4 : 30).ToUnixTimeSeconds();

            var lastError = "no model tried";
            foreach (var model in OrderModels(models))
            {
                bool? think = !shortCall;
                for (int attempt = 0; attempt < 3; attempt++)
                {
                    var left = deadlineTime - DateTimeOffset.UtcNow.ToUnixTimeSeconds();
                    if (left < 5) throw new TimeoutException($"NIM cascade ran out of time (last: {lastError})");

                    var body = new ChatRequest
                    {
                        model = model,
                        messages = messages,
                        max_tokens = tokens + (shortCall ? REASONING_ROOM : 0),
                        temperature = 0.6,
                        chat_template_kwargs = think == false ? new Dictionary<string, object> { { "enable_thinking", false } } : null
                    };

                    if (NIM_EXTRA.TryGetValue(model, out var extra) && extra is Dictionary<string, object> extraDict)
                    {
                        var keyName = shortCall ? "short" : "long";
                        if (extraDict.TryGetValue(keyName, out var extraConfig))
                        {
                            // Merge extra config
                        }
                    }

                    var jsonBody = JsonSerializer.Serialize(body);
                    using var request = new HttpRequestMessage(HttpMethod.Post, NIM_URL);
                    request.Headers.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", key);
                    request.Content = new StringContent(jsonBody, Encoding.UTF8, "application/json");

                    var t0 = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
                    try
                    {
                        var response = await HttpClient.SendAsync(request, HttpCompletionOption.ResponseHeadersRead);
                        var responseJson = await response.Content.ReadAsStringAsync();

                        if (response.IsSuccessStatusCode)
                        {
                            var chatResponse = JsonSerializer.Deserialize<ChatResponse>(responseJson);
                            var txt = StripThink(chatResponse.choices[0].message.content);
                            if (!string.IsNullOrEmpty(txt))
                            {
                                RecordHealth(model, true, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - t0);
                                return txt;
                            }

                            var reasoning = chatResponse.choices[0].message.reasoning_content ?? chatResponse.choices[0].message.reasoning;
                            if (think != false && (!string.IsNullOrEmpty(reasoning) || chatResponse.choices[0].finish_reason == "length"))
                            {
                                lastError = "reasoning used up the reply";
                                think = false;
                                continue;
                            }
                            lastError = "empty reply";
                            break;
                        }
                        else
                        {
                            lastError = $"HTTP {(int)response.StatusCode}";
                            if ((int)response.StatusCode == 400 && think == false)
                            {
                                think = null;
                                continue;
                            }
                            if (new[] { 429, 500, 502, 503, 504 }.Contains((int)response.StatusCode))
                            {
                                var retryAfter = response.Headers.RetryAfter?.Delta?.TotalSeconds ?? 60;
                                RecordHealth(model, false, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - t0, lastError, retryAfter);
                            }
                            break;
                        }
                    }
                    catch (TaskCanceledException) when (DateTimeOffset.UtcNow.ToUnixTimeSeconds() - t0 > 60)
                    {
                        lastError = "timeout";
                        RecordHealth(model, false, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - t0, lastError);
                        break;
                    }
                    catch (Exception e)
                    {
                        lastError = $"{e.GetType().Name} after {DateTimeOffset.UtcNow.ToUnixTimeSeconds() - t0}s";
                        if (attempt > 0 || e.Message.Contains("timeout"))
                        {
                            RecordHealth(model, false, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - t0, lastError);
                            break;
                        }
                        await Task.Delay(3000);
                    }
                }
                Log($"NIM {model} failed ({lastError}); trying next model");
            }
            throw new InvalidOperationException($"all NIM models failed: {lastError}");
        }

        private static string[] OrderModels(string[] models)
        {
            var now = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            var health = _state.nim_health ?? new Dictionary<string, ModelHealth>();
            var up = models.Where(m => !health.TryGetValue(m, out var h) || h.until <= now).ToArray();
            if (up.Length == 0)
                return models.OrderBy(m => health.TryGetValue(m, out var h) ? h.until : 0).ToArray();
            return up.OrderBy(m => (health.TryGetValue(m, out var h) && h.slow_until > now ? 1 : 0, Array.IndexOf(models, m))).ToArray();
        }

        private static void RecordHealth(string model, bool ok, double seconds, string why = "", double? retryAfter = null)
        {
            var health = _state.nim_health ?? new Dictionary<string, ModelHealth>();
            if (!health.TryGetValue(model, out var h))
            {
                h = new ModelHealth();
                health[model] = h;
            }
            var now = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            if (ok)
            {
                h.fails = 0;
                h.until = 0;
                h.last_ok = now;
                h.slow_until = seconds > SLOW_SECONDS ? now + 1800 : 0;
            }
            else
            {
                var wait = retryAfter ?? COOLDOWN_SECONDS[Math.Min(h.fails, COOLDOWN_SECONDS.Length - 1)];
                h.fails++;
                h.until = now + wait;
                h.last_fail = now;
                h.why = why.Substring(0, Math.Min(80, why.Length));
                Log($"NIM {model} skipped for {Math.Max(1, (int)Math.Round(wait / 60))} min ({why})");
            }
            _state.nim_health = health;
            SaveState();
        }

        public static string[] GetCascade(string kind = "text")
        {
            var builtin = kind == "text" ? NIM_TEXT : NIM_VISION;
            if (Environment.GetEnvironmentVariable("HIVE_LLM_TEST") != null) return builtin;

            try
            {
                if (!File.Exists(CascadePath)) return builtin;
                var json = File.ReadAllText(CascadePath);
                var data = JsonSerializer.Deserialize<CascadeData>(json);
                if (data == null) return builtin;

                var ranked = (kind == "text" ? data.text : data.vision) ?? new List<string>();
                if (ranked.Count < (kind == "text" ? 2 : 1) ||
                    DateTimeOffset.UtcNow.ToUnixTimeSeconds() - data.at > 21 * 86400)
                    return builtin;

                var dead = (data.dead ?? new List<string>()).ToHashSet();
                return ranked.Where(m => !dead.Contains(m)).Concat(builtin.Where(m => !ranked.Contains(m) && !dead.Contains(m))).ToArray();
            }
            catch
            {
                return builtin;
            }
        }

        private static async Task<string> FreeCascadeAsync(string system, object content, int maxTokens, bool vision = false)
        {
            var errors = new List<string>();
            try
            {
                return await CallNimAsync(system, content, maxTokens, GetCascade(vision ? "vision" : "text"), vision);
            }
            catch (Exception e)
            {
                errors.Add($"NIM: {e.Message.Substring(0, Math.Min(160, e.Message.Length))}");
            }

            if (IsGeminiEnabled())
            {
                try
                {
                    return await CallGeminiAsync(system, content, maxTokens, vision);
                }
                catch (Exception e)
                {
                    errors.Add($"Gemini: {e.Message.Substring(0, Math.Min(160, e.Message.Length))}");
                }
            }

            if (DateTimeOffset.UtcNow.ToUnixTimeSeconds() - (_state.cascade_alert_at ?? 0) > 6 * 3600)
            {
                _state.cascade_alert_at = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
                SaveState();
                var msg = $"LLM cascade exhausted: every free model failed ({string.Join("; ", errors)})";
                Log(msg);
                NotifyEvent("⚠️ " + msg);
            }
            throw new InvalidOperationException("every free model failed: " + string.Join("; ", errors));
        }

        private static bool IsGeminiEnabled()
        {
            try
            {
                if (!File.Exists(ConfigPath)) return false;
                var json = File.ReadAllText(ConfigPath);
                var doc = JsonDocument.Parse(json);
                return doc.RootElement.TryGetProperty("llm", out var llm) &&
                       llm.TryGetProperty("gemini_fallback", out var gf) && gf.GetBoolean() &&
                       !string.IsNullOrEmpty(GetEnv("GEMINI_API_KEY"));
            }
            catch { return false; }
        }

        private static async Task<string> CallGeminiAsync(string system, object content, int maxTokens, bool vision)
        {
            var key = GetEnv("GEMINI_API_KEY");
            var parts = new List<object>();

            if (content is string str)
                parts.Add(new { text = str });
            else if (content is List<ChatMessage> list)
            {
                foreach (var msg in list)
                {
                    if (msg.content is string s) parts.Add(new { text = s });
                    else if (msg.content is JsonElement je && je.TryGetProperty("type", out var t) && t.GetString() == "image")
                    {
                        var src = je.GetProperty("source");
                        parts.Add(new { inline_data = new { mime_type = src.GetProperty("media_type").GetString(), data = src.GetProperty("data").GetString() } });
                    }
                }
            }

            var tokens = Math.Min(maxTokens, 16000);
            var body = new
            {
                contents = new[] { new { role = "user", parts = parts } },
                generationConfig = new { maxOutputTokens = tokens + REASONING_ROOM, temperature = 0.6 },
                systemInstruction = string.IsNullOrEmpty(system) ? null : new { parts = new[] { new { text = system } } }
            };

            foreach (var model in GEMINI_MODELS)
            {
                var downModels = _state.nim_health?.Where(kvp => kvp.Value.until > DateTimeOffset.UtcNow.ToUnixTimeSeconds()).Select(kvp => kvp.Key).ToHashSet() ?? new HashSet<string>();
                if (downModels.Contains(model)) continue;

                var url = $"{GEMINI_URL}/{model}:generateContent";
                var jsonBody = JsonSerializer.Serialize(body);
                using var request = new HttpRequestMessage(HttpMethod.Post, url);
                request.Headers.Add("x-goog-api-key", key);
                request.Content = new StringContent(jsonBody, Encoding.UTF8, "application/json");

                var t0 = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
                try
                {
                    var response = await HttpClient.SendAsync(request);
                    var responseJson = await response.Content.ReadAsStringAsync();

                    if (response.IsSuccessStatusCode)
                    {
                        var doc = JsonDocument.Parse(responseJson);
                        var cand = doc.RootElement.GetProperty("candidates")[0];
                        var textParts = cand.GetProperty("content").GetProperty("parts").EnumerateArray()
                            .Where(p => !p.TryGetProperty("thought", out _))
                            .Select(p => p.GetProperty("text").GetString())
                            .Where(s => !string.IsNullOrEmpty(s));
                        var txt = StripThink(string.Concat(textParts));
                        if (!string.IsNullOrEmpty(txt))
                        {
                            RecordHealth(model, true, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - t0);
                            return txt;
                        }
                    }
                    else if (new[] { 429, 500, 502, 503, 504 }.Contains((int)response.StatusCode))
                    {
                        RecordHealth(model, false, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - t0, $"HTTP {(int)response.StatusCode}");
                    }
                }
                catch (Exception e)
                {
                    RecordHealth(model, false, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - t0, e.Message);
                }
                Log($"Gemini {model} failed; trying next model");
            }
            throw new InvalidOperationException("all Gemini models failed");
        }

        /// <summary>
        /// Main completion entry point.
        /// </summary>
        public static async Task<string> CompleteAsync(string system, object content, string model = "claude-opus-5-5", int maxTokens = 64000, bool vision = false, bool freeOnly = false)
        {
            if (GetProvider() == "claude" && !freeOnly)
            {
                var flaky = DateTimeOffset.UtcNow.ToUnixTimeSeconds() < (_state.claude_flaky_until ?? 0);
                model = GetBlockedModel(model);

                for (int attempt = 0; attempt < (flaky ? 1 : 3); attempt++)
                {
                    try
                    {
                        var result = await CallClaudeAsync(system, content, model, maxTokens);
                        if (_state.claude_out_at.HasValue)
                        {
                            _state.claude_out_at = null;
                            _state.claude_out_kind = null;
                            _state.claude_probe_at = null;
                            _state.reason = null;
                            SaveState();
                            Log("Claude is back; using Claude again");
                        }
                        return result;
                    }
                    catch (Exception e)
                    {
                        var msg = e.Message.ToLower();
                        var status = GetHttpStatus(e);

                        if (status == 402 || CREDIT_HINTS.Any(h => msg.Contains(h)))
                        {
                            MarkClaudeOut(e.Message);
                            break;
                        }
                        if (status == 403 && msg.Contains("model") && MODEL_FALLBACK.ContainsKey(model))
                        {
                            BlockModel(model, e.Message);
                            model = MODEL_FALLBACK[model];
                            try { return await CallClaudeAsync(system, content, model, maxTokens); }
                            catch (Exception e2)
                            {
                                if (!new[] { 401, 403 }.Contains(GetHttpStatus(e2)))
                                {
                                    Log($"Claude fallback {model} failed too ({e2.Message.Substring(0, 120)}); this call goes to NIM");
                                    break;
                                }
                                e = e2; msg = e2.Message.ToLower(); status = GetHttpStatus(e2);
                            }
                        }
                        if (status == 401 || status == 403)
                        {
                            MarkClaudeOut($"{status}: {e.Message.Substring(0, 160)}", "key");
                            break;
                        }
                        if (status == 400)
                        {
                            Log($"Claude rejected this request (400: {e.Message.Substring(0, 120)}); this call goes to NIM");
                            break;
                        }

                        Log($"Claude transient error ({e.Message.Substring(0, 120)}); {model}, failed after attempt {attempt + 1}" + (flaky ? " (no retry, flaky)" : ""));
                        if (!flaky) await Task.Delay(20000 * (attempt + 1));
                    }
                }
                Log("Claude unavailable after retries; this call goes to NIM" + (flaky ? "" : "; fast-fail for 15 min"));
                if (!flaky)
                {
                    _state.claude_flaky_until = DateTimeOffset.UtcNow.ToUnixTimeSeconds() + 15 * 60;
                    SaveState();
                }
            }
            return await FreeCascadeAsync(system, content, maxTokens, vision);
        }

        private static async Task<string> CallClaudeAsync(string system, object content, string model, int maxTokens)
        {
            var apiKey = GetEnv("ANTHROPIC_API_KEY");
            if (string.IsNullOrEmpty(apiKey)) throw new InvalidOperationException("ANTHROPIC_API_KEY not set");

            var messages = new List<object>();
            if (content is string str) messages.Add(new { role = "user", content = str });
            else if (content is List<ChatMessage> list) messages.AddRange(list.Select(m => new { m.role, m.content }));

            var body = new
            {
                model = model,
                max_tokens = maxTokens,
                system = system,
                messages = messages
            };

            var jsonBody = JsonSerializer.Serialize(body);
            using var request = new HttpRequestMessage(HttpMethod.Post, $"{ANTHROPIC_API_URL}/v1/messages");
            request.Headers.Add("x-api-key", apiKey);
            request.Headers.Add("anthropic-version", "2023-06-01");
            request.Content = new StringContent(jsonBody, Encoding.UTF8, "application/json");

            var response = await HttpClient.SendAsync(request);
            var responseJson = await response.Content.ReadAsStringAsync();

            if (!response.IsSuccessStatusCode)
            {
                var err = JsonDocument.Parse(responseJson);
                var errMsg = err.RootElement.GetProperty("error").GetProperty("message").GetString() ?? "unknown";
                throw new HttpException(errMsg, response.StatusCode);
            }

            var doc = JsonDocument.Parse(responseJson);
            var text = string.Concat(doc.RootElement.GetProperty("content").EnumerateArray()
                .Where(b => b.GetProperty("type").GetString() == "text")
                .Select(b => b.GetProperty("text").GetString()));

            // Log usage
            LogUsage("claude", model, doc.RootElement.GetProperty("usage"));
            return text;
        }

        public class HttpException : HttpRequestException
        {
            public System.Net.HttpStatusCode StatusCode { get; set; }
            public HttpException(string message, System.Net.HttpStatusCode statusCode) : base(message)
            {
                StatusCode = statusCode;
            }
        }

        private static int GetHttpStatus(Exception e)
        {
            if (e is HttpException hr) return (int)hr.StatusCode;
            return 0;
        }

        private static void LogUsage(string provider, string model, JsonElement usage)
        {
            try
            {
                var logPath = Path.Combine(Path.GetDirectoryName(StatePath), "api_usage.jsonl");
                EnsureDirectories();
                var entry = new
                {
                    at = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ"),
                    provider = provider,
                    model = model,
                    @in = usage.GetProperty("input_tokens").GetInt32(),
                    @out = usage.GetProperty("output_tokens").GetInt32(),
                    cache_read = usage.TryGetProperty("cache_read_input_tokens", out var cr) ? cr.GetInt32() : 0,
                    cache_write = usage.TryGetProperty("cache_creation_input_tokens", out var cw) ? cw.GetInt32() : 0
                };
                File.AppendAllText(logPath, JsonSerializer.Serialize(entry) + "\n");
            }
            catch { }
        }

        /// <summary>
        /// Probe all cascade models in parallel.
        /// </summary>
        public static async Task<List<(string model, bool ok, double seconds, string note)>> NimHealthAsync(string[] models = null)
        {
            var key = GetEnv("NVIDIA_API_KEY");
            if (string.IsNullOrEmpty(key)) return new List<(string, bool, double, string)>();

            models = models ?? GetCascade("text").Concat(GetCascade("vision")).Distinct().ToArray();
            var results = new List<(string, bool, double, string)>();
            var semaphore = new SemaphoreSlim(8);

            var tasks = models.Select(async m =>
            {
                await semaphore.WaitAsync();
                try
                {
                    var body = new ChatRequest
                    {
                        model = m,
                        messages = new List<ChatMessage> { new ChatMessage { role = "user", content = "Reply with exactly: CDN OK" } },
                        max_tokens = 20 + REASONING_ROOM,
                        temperature = 0.2,
                        chat_template_kwargs = new Dictionary<string, object> { { "enable_thinking", false } }
                    };
                    var jsonBody = JsonSerializer.Serialize(body);
                    using var request = new HttpRequestMessage(HttpMethod.Post, NIM_URL);
                    request.Headers.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", key);
                    request.Content = new StringContent(jsonBody, Encoding.UTF8, "application/json");

                    var t0 = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
                    for (int retry = 0; retry < 2; retry++)
                    {
                        try
                        {
                            var response = await HttpClient.SendAsync(request, HttpCompletionOption.ResponseHeadersRead);
                            var responseJson = await response.Content.ReadAsStringAsync();
                            var durationSec = (DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() - t0) / 1000.0;
                            if (response.IsSuccessStatusCode)
                            {
                                var chatResponse = JsonSerializer.Deserialize<ChatResponse>(responseJson);
                                var txt = StripThink(chatResponse.choices[0].message.content);
                                return (m, !string.IsNullOrEmpty(txt), durationSec, txt?.Substring(0, Math.Min(30, txt.Length)) ?? "empty");
                            }
                            if ((int)response.StatusCode == 400 && body.chat_template_kwargs != null)
                            {
                                body.chat_template_kwargs = null;
                                jsonBody = JsonSerializer.Serialize(body);
                                request.Content = new StringContent(jsonBody, Encoding.UTF8, "application/json");
                                continue;
                            }
                            return (m, false, durationSec, $"HTTP {(int)response.StatusCode}");
                        }
                        catch (Exception e)
                        {
                            var durationSec = (DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() - t0) / 1000.0;
                            return (m, false, durationSec, e.GetType().Name);
                        }
                    }
                    var finalDuration = (DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() - t0) / 1000.0;
                    return (m, false, finalDuration, "HTTP 400");
                }
                finally { semaphore.Release(); }
            });

            var allResults = await Task.WhenAll(tasks);
            foreach (var r in allResults.OrderBy(x => (!x.Item2, x.Item3)))
            {
                if (r.Item2) RecordHealth(r.Item1, true, r.Item3);
                else if (!r.Item4.StartsWith("HTTP 4") || r.Item4 == "HTTP 429") RecordHealth(r.Item1, false, r.Item3, r.Item4);
                results.Add((r.Item1, r.Item2, r.Item3, r.Item4));
            }
            return results;
        }
    }
}