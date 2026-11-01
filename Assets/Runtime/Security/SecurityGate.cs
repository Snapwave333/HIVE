using System;
using System.Collections.Generic;
using UnityEngine;
using HIVE.Core;
using HIVE.Persistence;
using HIVE.Agents;

namespace HIVE.Security
{
    /// <summary>
    /// Security & Audit subsystem: enforces agent permission gates, emergency lockout, and audit trails.
    /// </summary>
    public class SecurityGate : MonoBehaviour
    {
        public static SecurityGate Instance { get; private set; }

        public bool EmergencyLockdownActive { get; private set; }
        private Repository _repository;

        private void Awake()
        {
            if (Instance == null) Instance = this;
        }

        public void Initialize(Repository repository)
        {
            _repository = repository;
            Debug.Log("[SecurityGate] Initialized with RBAC security policies active.");
        }

        public bool CheckPermission(AgentDefinition agent, string scope, string targetAction)
        {
            if (EmergencyLockdownActive)
            {
                Debug.LogWarning($"[SecurityGate] DENIED action '{targetAction}' for {agent.Codename}: Emergency Lockdown active.");
                return false;
            }

            if (agent.Codename == "PRIME") return true;

            var permissions = agent.Permissions;
            if (permissions == null) return false;

            List<string> allowedScopes = scope.ToLowerInvariant() switch
            {
                "read" => permissions.Read,
                "write" => permissions.Write,
                "execute" => permissions.Execute,
                "external_comms" => permissions.ExternalComms,
                "financial" => permissions.Financial,
                _ => null
            };

            var isAllowed = allowedScopes != null && (allowedScopes.Contains("all") || allowedScopes.Contains(targetAction));

            _repository?.WriteAudit(new AuditEntry
            {
                EventType = isAllowed ? "security_grant" : "security_denial",
                Actor = agent.Codename,
                Target = scope,
                Action = targetAction,
                Details = new Dictionary<string, object>
                {
                    { "granted", isAllowed },
                    { "timestamp", DateTime.UtcNow.ToString("o") }
                }
            });

            return isAllowed;
        }

        public void EngageEmergencyLockdown(string reason = "Owner manual override")
        {
            EmergencyLockdownActive = true;
            Debug.LogError($"[SecurityGate] EMERGENCY LOCKDOWN ENGAGED: {reason}");
            _repository?.WriteAudit(new AuditEntry
            {
                EventType = "emergency_lockdown",
                Actor = "OWNER",
                Target = "all_agents",
                Action = "lockdown",
                Details = new Dictionary<string, object> { { "reason", reason } }
            });
            MessageBus.Publish(new SecurityAlertMessage
            {
                Alert = $"EMERGENCY LOCKDOWN: {reason}",
                SourceAgent = "SENTINEL"
            });
        }

        public void DisengageEmergencyLockdown()
        {
            EmergencyLockdownActive = false;
            Debug.Log("[SecurityGate] Emergency lockdown lifted.");
            _repository?.WriteAudit(new AuditEntry
            {
                EventType = "lockdown_lifted",
                Actor = "OWNER",
                Target = "all_agents",
                Action = "unlock",
                Details = new Dictionary<string, object>()
            });
        }
    }

    public class SecurityAlertMessage : MessageBus.IMessage
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string Alert { get; set; }
    }
}
