using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using UnityEngine;

namespace HIVE.Integrations
{
    /// <summary>
    /// Out-of-process bridge to the local Google account ecosystem via the gog CLI.
    /// Provides asynchronous access to Gmail, Google Drive, Google Calendar, Google Tasks, and Profile.
    /// Never blocks the Unity main thread or render loop.
    /// </summary>
    public class GoogleEcosystemBridge
    {
        private static GoogleEcosystemBridge _instance;
        public static GoogleEcosystemBridge Instance => _instance ??= new GoogleEcosystemBridge();

        public string CliPath { get; private set; }
        public bool IsAvailable { get; private set; }
        public string ConnectedAccount { get; private set; }
        public string LastError { get; private set; }
        public DateTime LastChecked { get; private set; }

        public GoogleEcosystemBridge()
        {
            _instance = this;
            ResolveCliPath();
        }

        /// <summary>
        /// Locates the gog CLI executable or wrapper batch script.
        /// </summary>
        public string ResolveCliPath()
        {
            // 1. Explicit environment overrides
            var env = Environment.GetEnvironmentVariable("GOG_CLI_PATH");
            if (!string.IsNullOrEmpty(env) && File.Exists(env))
            {
                CliPath = env;
                IsAvailable = true;
                return CliPath;
            }

            env = Environment.GetEnvironmentVariable("GOG_PATH");
            if (!string.IsNullOrEmpty(env) && File.Exists(env))
            {
                CliPath = env;
                IsAvailable = true;
                return CliPath;
            }

            // 2. gog-wrapper/gog.bat in UserProfile (active environment)
            var userProfile = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
            var wrapperBat = Path.Combine(userProfile, "gog-wrapper", "gog.bat");
            if (File.Exists(wrapperBat))
            {
                CliPath = wrapperBat;
                IsAvailable = true;
                return CliPath;
            }

            // 3. Go binary fallback
            var goBin = Path.Combine(userProfile, "go", "bin", "gog.exe");
            if (File.Exists(goBin))
            {
                CliPath = goBin;
                IsAvailable = true;
                return CliPath;
            }

            // 4. PATH resolution attempt
            var pathEnv = Environment.GetEnvironmentVariable("PATH") ?? "";
            var paths = pathEnv.Split(Path.PathSeparator);
            foreach (var p in paths)
            {
                try
                {
                    var candidateBat = Path.Combine(p.Trim(), "gog.bat");
                    if (File.Exists(candidateBat))
                    {
                        CliPath = candidateBat;
                        IsAvailable = true;
                        return CliPath;
                    }
                    var candidateExe = Path.Combine(p.Trim(), "gog.exe");
                    if (File.Exists(candidateExe))
                    {
                        CliPath = candidateExe;
                        IsAvailable = true;
                        return CliPath;
                    }
                }
                catch { }
            }

            // Fallback default
            CliPath = "gog.bat";
            IsAvailable = false;
            return CliPath;
        }

        /// <summary>
        /// Executes a gog command asynchronously on a background thread without freezing Unity.
        /// </summary>
        public async Task<GoogleCommandResult> ExecuteCommandAsync(string command, string arguments = "", int timeoutMs = 25000)
        {
            if (string.IsNullOrEmpty(CliPath))
            {
                ResolveCliPath();
            }

            return await Task.Run(() =>
            {
                var sw = Stopwatch.StartNew();
                var result = new GoogleCommandResult();

                try
                {
                    bool isBat = CliPath.EndsWith(".bat", StringComparison.OrdinalIgnoreCase) ||
                                 CliPath.EndsWith(".cmd", StringComparison.OrdinalIgnoreCase);

                    var psi = new ProcessStartInfo();
                    if (isBat)
                    {
                        psi.FileName = "cmd.exe";
                        psi.Arguments = $"/c \"\"{CliPath}\" {command} {arguments}\"";
                    }
                    else
                    {
                        psi.FileName = CliPath;
                        psi.Arguments = $"{command} {arguments}".Trim();
                    }

                    psi.UseShellExecute = false;
                    psi.RedirectStandardOutput = true;
                    psi.RedirectStandardError = true;
                    psi.CreateNoWindow = true;
                    psi.StandardOutputEncoding = Encoding.UTF8;
                    psi.StandardErrorEncoding = Encoding.UTF8;

                    using var proc = new Process { StartInfo = psi };
                    var stdoutBuilder = new StringBuilder();
                    var stderrBuilder = new StringBuilder();

                    proc.OutputDataReceived += (_, e) => { if (e.Data != null) stdoutBuilder.AppendLine(e.Data); };
                    proc.ErrorDataReceived += (_, e) => { if (e.Data != null) stderrBuilder.AppendLine(e.Data); };

                    proc.Start();
                    proc.BeginOutputReadLine();
                    proc.BeginErrorReadLine();

                    bool completed = proc.WaitForExit(timeoutMs);
                    sw.Stop();
                    result.ElapsedMs = sw.Elapsed.TotalMilliseconds;

                    if (!completed)
                    {
                        try { proc.Kill(); } catch { }
                        result.Success = false;
                        result.ExitCode = -1;
                        result.Error = $"Command timed out after {timeoutMs}ms";
                        LastError = result.Error;
                        return result;
                    }

                    result.ExitCode = proc.ExitCode;
                    result.Output = stdoutBuilder.ToString().Trim();
                    result.Error = stderrBuilder.ToString().Trim();
                    result.Success = proc.ExitCode == 0;

                    if (!result.Success && string.IsNullOrEmpty(result.Error))
                    {
                        result.Error = $"Process exited with code {proc.ExitCode}";
                    }

                    if (!result.Success)
                    {
                        LastError = result.Error;
                    }

                    return result;
                }
                catch (Exception ex)
                {
                    sw.Stop();
                    result.Success = false;
                    result.ExitCode = -1;
                    result.ElapsedMs = sw.Elapsed.TotalMilliseconds;
                    result.Error = ex.Message;
                    LastError = ex.Message;
                    return result;
                }
            }).ConfigureAwait(false);
        }

        /// <summary>
        /// Validates authentication and retrieves the current account email.
        /// </summary>
        public async Task<GoogleAuthStatus> CheckAuthStatusAsync()
        {
            var res = await ExecuteCommandAsync("auth", "status --json").ConfigureAwait(false);
            var status = new GoogleAuthStatus
            {
                RawJson = res.Output
            };

            if (res.Success && !string.IsNullOrEmpty(res.Output))
            {
                status.Authenticated = true;
                var emailMatch = Regex.Match(res.Output, "\"email\"\\s*:\\s*\"([^\"]+)\"");
                if (emailMatch.Success)
                {
                    status.Email = emailMatch.Groups[1].Value;
                    ConnectedAccount = status.Email;
                }

                var clientMatch = Regex.Match(res.Output, "\"client\"\\s*:\\s*\"([^\"]+)\"");
                if (clientMatch.Success) status.Client = clientMatch.Groups[1].Value;

                var authPrefMatch = Regex.Match(res.Output, "\"auth_preferred\"\\s*:\\s*\"([^\"]+)\"");
                if (authPrefMatch.Success) status.AuthType = authPrefMatch.Groups[1].Value;

                var keyringMatch = Regex.Match(res.Output, "\"backend\"\\s*:\\s*\"([^\"]+)\"");
                if (keyringMatch.Success) status.KeyringBackend = keyringMatch.Groups[1].Value;
            }
            else
            {
                status.Authenticated = false;
                status.Error = res.Error;
            }

            LastChecked = DateTime.UtcNow;
            return status;
        }

        /// <summary>
        /// Retrieves the authenticated user's profile information.
        /// </summary>
        public async Task<GoogleUserProfile> GetProfileAsync()
        {
            var res = await ExecuteCommandAsync("me", "--json").ConfigureAwait(false);
            var profile = new GoogleUserProfile { RawJson = res.Output };

            if (res.Success && !string.IsNullOrEmpty(res.Output))
            {
                profile.Success = true;
                var emailMatch = Regex.Match(res.Output, "\"value\"\\s*:\\s*\"([^\"]+)\"");
                if (emailMatch.Success) profile.Email = emailMatch.Groups[1].Value;

                var resNameMatch = Regex.Match(res.Output, "\"resourceName\"\\s*:\\s*\"([^\"]+)\"");
                if (resNameMatch.Success) profile.ResourceName = resNameMatch.Groups[1].Value;

                if (!string.IsNullOrEmpty(profile.Email))
                {
                    ConnectedAccount = profile.Email;
                }
            }
            else
            {
                profile.Success = false;
                profile.Error = res.Error;
            }

            return profile;
        }

        /// <summary>
        /// Searches Google Drive files matching query keywords.
        /// </summary>
        public async Task<List<GoogleDriveItem>> SearchDriveAsync(string query, int limit = 10)
        {
            var safeQuery = (query ?? "").Replace("\"", "\\\"");
            var args = $"search \"{safeQuery}\" --max {limit} --json --results-only";
            var res = await ExecuteCommandAsync("drive", args).ConfigureAwait(false);

            if (!res.Success || string.IsNullOrEmpty(res.Output))
            {
                return new List<GoogleDriveItem>();
            }

            return ParseJsonArray<GoogleDriveItem>(res.Output);
        }

        /// <summary>
        /// Lists recent files in Google Drive.
        /// </summary>
        public async Task<List<GoogleDriveItem>> ListDriveFilesAsync(int limit = 10)
        {
            var args = $"ls --max {limit} --json --results-only";
            var res = await ExecuteCommandAsync("drive", args).ConfigureAwait(false);

            if (!res.Success || string.IsNullOrEmpty(res.Output))
            {
                return new List<GoogleDriveItem>();
            }

            return ParseJsonArray<GoogleDriveItem>(res.Output);
        }

        /// <summary>
        /// Lists upcoming Google Calendar events.
        /// </summary>
        public async Task<List<GoogleCalendarEvent>> ListCalendarEventsAsync(int limit = 10)
        {
            var args = $"events --max {limit} --json --results-only";
            var res = await ExecuteCommandAsync("calendar", args).ConfigureAwait(false);

            if (!res.Success || string.IsNullOrEmpty(res.Output))
            {
                return new List<GoogleCalendarEvent>();
            }

            return ParseJsonArray<GoogleCalendarEvent>(res.Output);
        }

        /// <summary>
        /// Searches Gmail messages matching a search query (e.g. "newer_than:7d", "is:unread").
        /// </summary>
        public async Task<List<GoogleGmailMessage>> ListGmailMessagesAsync(string query = "newer_than:7d", int limit = 10)
        {
            var safeQuery = string.IsNullOrEmpty(query) ? "newer_than:7d" : query.Replace("\"", "\\\"");
            var args = $"messages list \"{safeQuery}\" --max {limit} --json --results-only";
            var res = await ExecuteCommandAsync("gmail", args).ConfigureAwait(false);

            if (!res.Success || string.IsNullOrEmpty(res.Output))
            {
                return new List<GoogleGmailMessage>();
            }

            return ParseJsonArray<GoogleGmailMessage>(res.Output);
        }

        /// <summary>
        /// Lists task lists from Google Tasks.
        /// </summary>
        public async Task<List<GoogleTaskItem>> ListTasksAsync()
        {
            var args = "lists --json --results-only";
            var res = await ExecuteCommandAsync("tasks", args).ConfigureAwait(false);

            if (!res.Success || string.IsNullOrEmpty(res.Output))
            {
                return new List<GoogleTaskItem>();
            }

            return ParseJsonArray<GoogleTaskItem>(res.Output);
        }

        /// <summary>
        /// Parses a top-level JSON array using Unity's JsonUtility with an envelope wrapper.
        /// </summary>
        public static List<T> ParseJsonArray<T>(string json)
        {
            var list = new List<T>();
            if (string.IsNullOrWhiteSpace(json)) return list;

            var trimmed = json.Trim();
            if (trimmed.StartsWith("["))
            {
                try
                {
                    string wrapped = "{\"items\":" + trimmed + "}";
                    var container = JsonUtility.FromJson<JsonArrayContainer<T>>(wrapped);
                    if (container?.items != null)
                    {
                        return container.items;
                    }
                }
                catch (Exception ex)
                {
                    UnityEngine.Debug.LogWarning($"[GoogleEcosystemBridge] JsonUtility array parsing note: {ex.Message}");
                }
            }
            else if (trimmed.StartsWith("{"))
            {
                try
                {
                    var single = JsonUtility.FromJson<T>(trimmed);
                    if (single != null) list.Add(single);
                }
                catch { }
            }

            return list;
        }

        [Serializable]
        private class JsonArrayContainer<T>
        {
            public List<T> items;
        }
    }

    [Serializable]
    public class GoogleCommandResult
    {
        public bool Success;
        public int ExitCode;
        public string Output;
        public string Error;
        public double ElapsedMs;
    }

    [Serializable]
    public class GoogleAuthStatus
    {
        public bool Authenticated;
        public string Email;
        public string Client;
        public string AuthType;
        public string KeyringBackend;
        public string Error;
        public string RawJson;
    }

    [Serializable]
    public class GoogleUserProfile
    {
        public bool Success;
        public string Email;
        public string ResourceName;
        public string Error;
        public string RawJson;
    }

    [Serializable]
    public class GoogleDriveItem
    {
        public string id;
        public string name;
        public string mimeType;
        public string webViewLink;
        public string modifiedTime;
        public string size;
    }

    [Serializable]
    public class GoogleCalendarEvent
    {
        public string id;
        public string summary;
        public string description;
        public string location;
        public string status;
        public string htmlLink;
        public string startDayOfWeek;
        public string endDayOfWeek;
    }

    [Serializable]
    public class GoogleGmailMessage
    {
        public string id;
        public string threadId;
        public string from;
        public string subject;
        public string date;
        public string[] labels;
    }

    [Serializable]
    public class GoogleTaskItem
    {
        public string id;
        public string title;
        public string updated;
        public string selfLink;
    }
}
