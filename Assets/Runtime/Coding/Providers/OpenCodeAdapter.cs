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
    public class OpenCodeAdapter : ICodingCliAdapter
    {
        public string ProviderName => "OpenCode";
        public bool IsFallback => true;

        private readonly string _executablePath;
        private readonly string _freeModelPolicy;

        public OpenCodeAdapter(string customPath = null, string freeModel = "free")
        {
            _executablePath = ResolveExecutable(customPath);
            _freeModelPolicy = freeModel;
        }

        private static string ResolveExecutable(string customPath)
        {
            if (!string.IsNullOrEmpty(customPath) && File.Exists(customPath))
                return customPath;

            var envPath = Environment.GetEnvironmentVariable("OPENCODE_PATH");
            if (!string.IsNullOrEmpty(envPath) && File.Exists(envPath))
                return envPath;

            var appDataNpm = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "npm", "opencode.cmd");
            if (File.Exists(appDataNpm))
                return appDataNpm;

            return "opencode";
        }

        public Task<bool> IsAvailableAsync(CancellationToken ct = default)
        {
            try
            {
                if (File.Exists(_executablePath) || _executablePath == "opencode")
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
                IsAvailable = File.Exists(_executablePath) || _executablePath == "opencode",
                State = ProviderState.Available,
                RemainingDailyTokens = 5000000,
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
                // Verify model is free under policy
                if (string.IsNullOrEmpty(_freeModelPolicy))
                {
                    result.Success = false;
                    result.Failure = FailureCategory.Authentication;
                    result.ErrorMessage = "OpenCode invocation blocked: no free model policy configured.";
                    return result;
                }

                var psi = new ProcessStartInfo
                {
                    FileName = _executablePath,
                    Arguments = $"run --model {_freeModelPolicy} \"{request.Prompt.Replace("\"", "\\\"")}\"",
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
                        onOutputLine?.Invoke($"[OPENCODE STDERR] {e.Data}");
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
                    result.ErrorMessage = $"OpenCode execution exceeded timeout of {request.TimeoutSeconds}s.";
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

            if (combined.Contains("rate limit") || combined.Contains("429") || combined.Contains("too many requests"))
                return FailureCategory.RateLimit;

            if (combined.Contains("unauthorized") || combined.Contains("auth") || combined.Contains("login"))
                return FailureCategory.Authentication;

            if (combined.Contains("timed out") || combined.Contains("timeout"))
                return FailureCategory.NetworkTimeout;

            return FailureCategory.ProcessCrash;
        }

        private void ExtractTelemetry(CliExecutionResult result)
        {
            result.Telemetry.IsMeasured = false;
            result.Telemetry.InputTokens = Math.Max(10, result.Stdout.Length / 4);
            result.Telemetry.OutputTokens = Math.Max(5, (result.Stdout.Length + result.Stderr.Length) / 8);
            result.Telemetry.EstimatedCostUsd = 0.0m; // Free fallback model
        }
    }
}
