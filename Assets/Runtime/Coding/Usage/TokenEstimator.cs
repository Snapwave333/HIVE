using System;

namespace HIVE.Coding
{
    public static class TokenEstimator
    {
        /// <summary>
        /// Estimates token consumption when measured telemetry is unavailable.
        /// Adheres strictly to the Level 1-4 telemetry hierarchy.
        /// </summary>
        public static UsageTelemetry EstimateFromText(string inputText, string outputText, string provider, bool exactAvailable = false)
        {
            var telemetry = new UsageTelemetry();

            if (exactAvailable)
            {
                telemetry.IsMeasured = true;
                return telemetry;
            }

            telemetry.IsMeasured = false;
            // Level 4 fallback: character heuristic (~4 chars/token)
            telemetry.InputTokens = Math.Max(1, (inputText?.Length ?? 0) / 4);
            telemetry.OutputTokens = Math.Max(1, (outputText?.Length ?? 0) / 4);
            telemetry.CachedTokens = 0;

            telemetry.EstimatedCostUsd = CostEstimator.CalculateCost(provider, "default", telemetry.InputTokens, telemetry.OutputTokens, telemetry.CachedTokens);
            return telemetry;
        }
    }
}
