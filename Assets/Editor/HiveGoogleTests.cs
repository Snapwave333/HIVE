using System;
using System.IO;
using System.Threading.Tasks;
using UnityEngine;
using HIVE.Integrations;
using HIVE.Skills;

namespace HIVE.Editor
{
    /// <summary>
    /// Integration tests for the Google Ecosystem Bridge and gog CLI connectivity.
    /// </summary>
    public static class HiveGoogleTests
    {
        private static T RunSync<T>(Func<Task<T>> func, int timeoutMs = 20000)
        {
            var task = Task.Run(func);
            if (task.Wait(timeoutMs))
            {
                return task.Result;
            }
            throw new TimeoutException($"Async operation timed out after {timeoutMs}ms");
        }

        public static void RunGoogleTests(Action<string, bool, string> report)
        {
            var bridge = GoogleEcosystemBridge.Instance;

            // TEST 1: CLI Resolution
            var resolvedPath = bridge.ResolveCliPath();
            bool cliExists = !string.IsNullOrEmpty(resolvedPath) && File.Exists(resolvedPath);
            report("Google CLI Path Resolution", cliExists, $"Resolved: {resolvedPath}");

            if (!cliExists)
            {
                report("Google CLI Authentication Status", false, "CLI not found on system");
                report("Google Profile Verification", false, "Skipped due to missing CLI");
                report("Google Drive Access", false, "Skipped due to missing CLI");
                report("Google Calendar Access", false, "Skipped due to missing CLI");
                report("Google Skills Integration", false, "Skipped due to missing CLI");
                return;
            }

            // TEST 2: Authentication Status Verification
            try
            {
                var auth = RunSync(() => bridge.CheckAuthStatusAsync());
                bool authOk = auth != null && auth.Authenticated && auth.Email.IndexOf("@", StringComparison.Ordinal) > 0;
                report("Google CLI Authentication Status", authOk, authOk ? $"Connected: {auth.Email} (Client: {auth.Client})" : $"Auth Error: {auth?.Error}");
            }
            catch (Exception ex)
            {
                report("Google CLI Authentication Status", false, $"Exception: {ex.Message}");
            }

            // TEST 3: User Profile Verification
            try
            {
                var profile = RunSync(() => bridge.GetProfileAsync());
                bool profileOk = profile != null && profile.Success && !string.IsNullOrEmpty(profile.Email);
                report("Google Profile Verification", profileOk, profileOk ? $"Profile verified for {profile.Email}" : $"Profile Error: {profile?.Error}");
            }
            catch (Exception ex)
            {
                report("Google Profile Verification", false, $"Exception: {ex.Message}");
            }

            // TEST 4: Google Drive Query Access
            try
            {
                var files = RunSync(() => bridge.ListDriveFilesAsync(5));
                bool driveOk = files != null && files.Count > 0;
                report("Google Drive Access", driveOk, driveOk ? $"Retrieved {files.Count} Drive files. First: {files[0].name}" : "No files returned");
            }
            catch (Exception ex)
            {
                report("Google Drive Access", false, $"Exception: {ex.Message}");
            }

            // TEST 5: Google Calendar Query Access
            try
            {
                var events = RunSync(() => bridge.ListCalendarEventsAsync(5));
                bool calOk = events != null && events.Count > 0;
                report("Google Calendar Access", calOk, calOk ? $"Retrieved {events.Count} Calendar events. First: {events[0].summary}" : "No events returned");
            }
            catch (Exception ex)
            {
                report("Google Calendar Access", false, $"Exception: {ex.Message}");
            }

            // TEST 6: Skill Registry Execution of Google Skill
            try
            {
                var skillGO = new GameObject("TestGoogleSkillRegistry");
                var registry = skillGO.AddComponent<SkillRegistry>();
                registry.Initialize(null);
                var skillRes = RunSync(() => registry.ExecuteSkillAsync("google_auth_status", "LINK"));
                bool skillOk = skillRes != null && skillRes.Success && skillRes.Output.Contains("@");
                report("Google Skills Integration", skillOk, skillRes?.Output);
                GameObject.DestroyImmediate(skillGO);
            }
            catch (Exception ex)
            {
                report("Google Skills Integration", false, $"Exception: {ex.Message}");
            }
        }
    }
}
