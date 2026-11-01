namespace HIVE.Coding
{
    public static class CostEstimator
    {
        public static decimal CalculateCost(string provider, string model, long inputTokens, long outputTokens, long cachedTokens)
        {
            if (provider == "OpenCode")
            {
                // Configured free model policy guarantees zero monetary cost
                return 0.0m;
            }

            // Claude Code / Anthropic standard rates
            // Sonnet: $3.00/M input, $15.00/M output, $0.30/M cached input
            decimal inputRatePerMillion = 3.00m;
            decimal outputRatePerMillion = 15.00m;
            decimal cachedRatePerMillion = 0.30m;

            if (model != null && model.Contains("haiku"))
            {
                inputRatePerMillion = 0.25m;
                outputRatePerMillion = 1.25m;
                cachedRatePerMillion = 0.03m;
            }

            decimal cost = (inputTokens * (inputRatePerMillion / 1_000_000m))
                         + (outputTokens * (outputRatePerMillion / 1_000_000m))
                         + (cachedTokens * (cachedRatePerMillion / 1_000_000m));

            return System.Math.Round(cost, 6);
        }
    }
}
