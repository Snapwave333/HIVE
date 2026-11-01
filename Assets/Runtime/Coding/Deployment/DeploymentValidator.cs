using System;
using System.Collections.Generic;
using System.IO;
using System.Text.RegularExpressions;
using System.Threading;
using System.Threading.Tasks;

namespace HIVE.Coding
{
    public class DeploymentValidator
    {
        private static readonly Regex SecretPattern = new Regex(
            @"(sk-ant-[a-zA-Z0-9_\-]{20,}|nvapi-[a-zA-Z0-9_\-]{20,}|ghp_[a-zA-Z0-9]{20,}|-----BEGIN\s+(?:RSA\s+)?PRIVATE\s+KEY-----)",
            RegexOptions.Compiled | RegexOptions.IgnoreCase);

        /// <summary>
        /// SENTINEL static diff security check.
        /// Scans diffs for leaked secrets, .env additions, and permission escalations.
        /// </summary>
        public Task<bool> ValidateSecurityDiffAsync(string diffText, CancellationToken ct = default)
        {
            if (string.IsNullOrEmpty(diffText))
                return Task.FromResult(true);

            // Secret pattern scan
            if (SecretPattern.IsMatch(diffText))
                return Task.FromResult(false);

            // Disallow adding or modifying raw .env files
            if (Regex.IsMatch(diffText, @"^\+\+\+\s+b/\.env(?:\.local)?$", RegexOptions.Multiline))
                return Task.FromResult(false);

            // Disallow adding wildcard write permissions for non-PRIME agents
            if (diffText.Contains("\"write\": [\"all\"]") && !diffText.Contains("PRIME.json"))
                return Task.FromResult(false);

            return Task.FromResult(true);
        }

        /// <summary>
        /// GAUGE test inventory matcher.
        /// Ensures candidate test suite inventory includes all baseline tests without silent skipping.
        /// </summary>
        public bool ValidateTestInventory(IReadOnlyList<string> baselineTests, IReadOnlyList<string> candidateTests)
        {
            if (baselineTests == null || baselineTests.Count == 0)
                return true;

            if (candidateTests == null || candidateTests.Count < baselineTests.Count)
                return false;

            var candidateSet = new HashSet<string>(candidateTests);
            foreach (var baselineTest in baselineTests)
            {
                if (!candidateSet.Contains(baselineTest))
                    return false; // Test was dropped or skipped!
            }

            return true;
        }
    }
}
