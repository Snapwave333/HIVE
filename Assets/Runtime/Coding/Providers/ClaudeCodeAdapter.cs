using System;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading;
using System.Threading.Tasks;
using UnityEngine;

namespace HIVE.Coding
{
    public class ClaudeCodeAdapter : ICodingCliAdapter
    {
        public string ProviderName => "ClaudeCode";
        public bool IsFallback => false;

        private readonly string _executablePath;

        public ClaudeCodeAdapter(string customPath = null)
        {
            _executablePath = ResolveExecutable(customPath);
        }

        private static string ResolveExecutable(string customPath)
        {
            if (!string.IsNullOrEmpty(customPath) && File.Exists(customPath))
                return customPath;

            var envPath = Environment.GetEnvironmentVariable("CLAUDE_PATH");
            if (!string.IsNullOrEmpty(envPath) && File.Exists(envPath))
                return envPath;

            var userLocal = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".local", "bin", "claude.exe");
            if (File.Exists(userLocal))
                return userLocal;

            return "claude.exe";
        }

        public Task<bool> IsAvailableAsync(CancellationToken ct = default)
        {
            try
            {
                if (File.Exists(_executablePath) || _executablePath == "claude.exe")
                    return Task.FromResult(true);
                return Task.FromResult(false);
            }
            catch
            {
                return Task.FromResult(false);
            }
        }

        public Task<ProviderCapacity> CheckCapacityAsync(CancellationToken ct = default)
        {
            var capacity = new ProviderCapacity
            {
                ProviderName = ProviderName,
                IsAvailable = File.Exists(_executablePath) || _executablePath == "claude.exe",
                State = ProviderState.Available,
                RemainingDailyTokens = 1000000,
                RateLimitResetSeconds = 0
            };
            return Task.FromResult(capacity);
        }

        public async Task<CliExecutionResult> ExecuteAsync(CliExecutionRequest request, Action<string> onOutputLine, CancellationToken ct = default)
        {
            var result = new CliExecutionResult();
            var stdoutBuilder = new StringBuilder();
            var stderrBuilder = new StringBuilder();

            try
            {
                var psi = new ProcessStartInfo
                {
                    FileName = _executablePath,
                    Arguments = $"--print \"{request.Prompt.Replace("\"", "\\\"")}\"",
                    WorkingDirectory = !string.IsNullOrEmpty(request.WorktreePath) && Directory.Exists(request.WorktreePath)
                        ? request.WorktreePath
                        : Directory.GetCurrentDirectory(),
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    UseShellExecute = false,
                    CreateNoWindow = true,
                    StandardOutputEncoding = Encoding.UTF8,
                    StandardErrorEncoding = Encoding.UTF8
                };

                using var process = new Process { StartInfo = psi, EnableRaisingEvents = true };

                process.OutputDataReceived += (s, e) =>
                {
                    if (e.Data != null)
                    {
                        stdoutBuilder.AppendLine(e.Data);
                        onOutputLine?.Invoke(e.Data);
                    }
                };

                process.ErrorDataReceived += (s, e) =>
                {
                    if (e.Data != null)
                    {
                        stderrBuilder.AppendLine(e.Data);
                        onOutputLine?.Invoke($"[STDERR] {e.Data}");
                    }
                };

                process.Start();
                process.BeginOutputReadLine();
                process.BeginErrorReadLine();

                var timeoutMs = request.TimeoutSeconds * 1000;
                var processTask = Task.Run(() => process.WaitForExit(timeoutMs));

                using var reg = ct.Register(() =>
                {
                    try { if (!process.HasExited) process.Kill(); } catch { }
                });

                bool completedInTime = await processTask.ConfigureAwait(false);

                if (!completedInTime)
                {
                    try { if (!process.HasExited) process.Kill(); } catch { }
                    result.Success = false;
                    result.Failure = FailureCategory.NetworkTimeout;
                    result.ErrorMessage = $"Execution exceeded timeout of {request.TimeoutSeconds}s.";
                    return result;
                }

                result.ExitCode = process.ExitCode;
                result.Stdout = stdoutBuilder.ToString();
                result.Stderr = stderrBuilder.ToString();
                result.Failure = ClassifyExit(process.ExitCode, result.Stderr, result.Stdout);
                result.Success = result.Failure == FailureCategory.None;

                ExtractTelemetry(result);
                return result;
            }
            catch (Exception ex)
            {
                result.Success = false;
                result.Failure = FailureCategory.ProcessCrash;
                result.ErrorMessage = ex.Message;
                return result;
            }
        }

        public FailureCategory ClassifyExit(int exitCode, string stderr, string stdout)
        {
            if (exitCode == 0) return FailureCategory.None;

            var combined = (stderr + " " + stdout).ToLowerInvariant();

            if (combined.Contains("rate limit") || combined.Contains("429") || combined.Contains("too many requests") || combined.Contains("quota"))
                return FailureCategory.RateLimit;

            if (combined.Contains("unauthorized") || combined.Contains("authentication") || combined.Contains("auth fail") || combined.Contains("api key missing"))
                return FailureCategory.Authentication;

            if (combined.Contains("timed out") || combined.Contains("timeout"))
                return FailureCategory.NetworkTimeout;

            return FailureCategory.ProcessCrash;
        }

        private void ExtractTelemetry(CliExecutionResult result)
        {
            // Attempt to parse token output from Claude Code output
            var match = Regex.Match(result.Stdout, @"(\d+)\s+input\s+tokens.*?(\d+)\s+output\s+tokens", RegexOptions.IgnoreCase);
            if (match.Success)
            {
                result.Telemetry.IsMeasured = true;
                long.TryParse(match.Groups[1].Value, out result.Telemetry.InputTokens);
                long.TryParse(match.Groups[2].Value, out result.Telemetry.OutputTokens);
            }
            else
            {
                // Fallback to heuristic Level 4
                result.Telemetry.IsMeasured = false;
                result.Telemetry.InputTokens = Math.Max(10, result.Stdout.Length / 4);
                result.Telemetry.OutputTokens = Math.Max(5, (result.Stdout.Length + result.Stderr.Length) / 8);
            }

            // Estimate cost: ~$3/M input, ~$15/M output for Sonnet tier
            result.Telemetry.EstimatedCostUsd = (result.Telemetry.InputTokens * 0.000003m) + (result.Telemetry.OutputTokens * 0.000015m);
        }
    }
}
