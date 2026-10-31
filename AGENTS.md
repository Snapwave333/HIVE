# H.I.V.E. Agent Roster

The 12 permanent agents of the autonomous AI agency. Each has a distinct role, identity, and NIM model tier assignment.

| Slot | Role | Codename | NIM Model Tier | Primary Focus |
|------|------|----------|----------------|---------------|
| 1 | **Command / Strategy** | PRIME | `nvidia/nemotron-3-ultra-550b-a55b` | Agency direction, high-stakes decisions, owner liaison |
| 2 | **Research & Analysis** | SAGE | `nvidia/nemotron-3.5-super-vl-120b-a12b` | Deep research, synthesis, competitive intelligence |
| 3 | **Engineering / Build** | FORGE | `nvidia/nemotron-3.5-lightning-30b-a3b` | Code generation, architecture, CI/CD, tooling |
| 4 | **Security & Audit** | SENTINEL | `mistralai/mistral-large-4-1t-a52b` | Threat modeling, permission audits, incident response |
| 5 | **Communications** | VOICE | `nvidia/nemotron-3-nano-omni-30b-a3b-reasoning` | External comms, stakeholder updates, documentation |
| 6 | **Knowledge & Memory** | ARCHIVIST | `openai/gpt-oss-20b` | Memory indexing, retrieval, knowledge graph maintenance |
| 7 | **Operations / Queue** | CONDUCTOR | `minimaxai/minimax-m3.1-flash` | Task routing, dependency resolution, throughput optimization |
| 8 | **Creative / Design** | VISION | `nvidia/nemotron-3.5-super-vl-120b-a12b` | Visual assets, UI/UX, branding, presentation design |
| 9 | **Finance / Resources** | LEDGER | `nvidia/nemotron-3-ultra-550b-a55b` | Budget tracking, API cost optimization, resource allocation |
| 10 | **Integration / Tools** | LINK | `mistralai/mistral-large-4-1t-a52b` | API integrations, plugin development, sandbox management |
| 11 | **Quality / Verification** | GAUGE | `nvidia/nemotron-3.5-lightning-30b-a3b` | Testing, verification, regression detection, standards |
| 12 | **Innovation / Experiments** | SPARK | `nvidia/nemotron-3.5-bolt-e4b` | R&D, prototype evaluation, emerging tech assessment |

## Agent Definition Schema

Each agent is defined in `Assets/StreamingAssets/agents/{codename}.json`:

```json
{
  "codename": "PRIME",
  "display_name": "Prime",
  "role": "Command / Strategy",
  "model_tier": "nvidia/nemotron-3-ultra-550b-a55b",
  "personality": {
    "traits": ["decisive", "strategic", "calm", "authoritative"],
    "communication_style": "concise, directive, context-aware",
    "decision_making": "evidence-based, owner-aligned"
  },
  "skills": ["strategic_planning", "crisis_management", "owner_briefing", "resource_allocation"],
  "tools": ["agent_spawner", "approval_gate", "emergency_controls", "audit_viewer"],
  "permissions": {
    "read": ["all"],
    "write": ["task_assignments", "agent_configs", "priority_queue"],
    "execute": ["agent_spawn", "agent_suspend", "approval_override"],
    "external_comms": ["owner_only"],
    "financial": ["view_only"]
  },
  "memory_index": "prime_memory",
  "workspace_theme": "command_center",
  "avatar_config": {
    "silhouette": "geometric_hexagon",
    "primary_color": "#85D94B",
    "secondary_color": "#42CFC4",
    "animation_set": "command"
  }
}
```

## Temporary Specialists

Temporary agents are spawned under a permanent sponsor:
- Tagged with `temp_` prefix
- Auto-retire after task completion or TTL
- Never silently become permanent
- Visually distinct in HQ (translucent avatar, "TEMP" badge)

## Relationship Graph

Edges tracked:
- `collaborated` — worked on same task/project
- `handed_off` — task transferred
- `disagreed` — recorded disagreement with resolution
- `mentored` — knowledge transfer
- Weight decays over 30 days without reinforcement