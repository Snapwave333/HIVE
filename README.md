# H.I.V.E. — Borg Cube Autonomous AI Agency Headquarters

Immersive 3D cybernetic headquarters for a 12-agent autonomous AI agency. Built in Unity with NVIDIA NIM cascade fallback LLM routing.

## Overview

H.I.V.E. (Hierarchical Intelligence Virtual Environment) is the operational headquarters for a 12-agent autonomous AI agency. It combines:

- **Immersive 3D Environment** — Navigable Borg-inspired cybernetic megastructure
- **12 Persistent Agents** — Each with unique role, workspace, memory, and personality
- **NIM Cascade Router** — Claude → NVIDIA NIM → Gemini fallback (ported from StickExplainers)
- **Unified Command Interface** — Chat, terminal, voice, file attachments, visual controls
- **Customizable Dashboards** — Drag-drop panels, 3 density modes, 8 presets
- **Security & Approvals** — Explicit approval flows, credential vault, audit log, emergency controls

## Quick Start

### Prerequisites

1. **Unity 2022.3 LTS or 2023.2+** (URP, .NET Standard 2.1)
   - Download: https://unity.com/download
   - Install via Unity Hub: Editor + Universal RP + Visual Studio Editor

2. **API Keys** (set as environment variables)
   ```
   ANTHROPIC_API_KEY=your_claude_key
   NVIDIA_API_KEY=your_nim_key
   GEMINI_API_KEY=your_gemini_key  # optional, for fallback tier
   ```

3. **Git** (for version control)

### Installation

```bash
# Clone or navigate to project
cd "C:/Users/chrom/Desktop/1 📂 Projects/H.I.V.E"

# Open in Unity Hub → Add Project → Select this folder
# Unity will import packages and compile scripts
```

### First Run

1. Open `Assets/Scenes/CommandChamber.unity` (create if missing)
2. Enter Play Mode
3. Press `~` (tilde) to open Command Input
4. Type `@PRIME status` — should route through NIM cascade and return response

## Project Structure

```
H.I.V.E/
├── Assets/
│   ├── Runtime/
│   │   ├── Core/           # Logging, MessageBus, Repository
│   │   ├── LLM/            # NimCascadeRouter (Claude→NIM→Gemini)
│   │   ├── Agents/         # AgentSpawner, RelationshipGraph
│   │   ├── Tasks/          # TaskManager, states
│   │   ├── Memory/         # MemoryStore (JSONL + vector index)
│   │   ├── Skills/         # ISkill plugin system
│   │   ├── Persistence/    # SQLite/LiteDB repository
│   │   ├── Security/       # PermissionInspector, CredentialVault, AuditLog
│   │   ├── UI/             # Panels, Widgets, CommandInput, Terminal
│   │   ├── Camera/         # AdaptiveCamera (FPS/TPS/Iso/Overview)
│   │   └── Voice/          # Whisper.cpp + Piper TTS
│   ├── Editor/
│   │   └── ModelLab/       # ModelLab.cs — NIM benchmarking → cascade.json
│   ├── StreamingAssets/
│   │   ├── cascade.json    # Ranked NIM model lists (auto-updated)
│   │   ├── config.json     # Runtime configuration
│   │   └── agents/         # 12 agent definition JSONs
│   ├── Art/
│   │   ├── Architecture/   # Modular corridor/room prefabs
│   │   └── Materials/      # Borg aesthetic materials
│   ├── Scenes/             # 7 HQ areas
│   └── Prefabs/Agents/     # 12 unique workspaces
├── ProjectSettings/
├── docs/
└── AGENTS.md               # Agent roster & schema
```

## Key Systems

### NimCascadeRouter (`Assets/Runtime/LLM/NimCascadeRouter.cs`)

Port of StickExplainers' `stickfx/llm.py`. Priority chain:

1. **Claude** (ANTHROPIC_API_KEY) — primary, credits permitting
2. **NIM Cascade** (NVIDIA_API_KEY) — ranked by Model Lab benchmarks
   - Breaker per model (429→Retry-After, 5xx→2/10/30 min cooldown)
   - Slow successes (>90s) moved to back for 30 min
   - Short calls (<4k tokens): thinking off, +2048 reasoning room
3. **Gemini** (GEMINI_API_KEY) — only if `config.json: llm.gemini_fallback=true`

State persisted to `~/.hive/llm_state.json`.

### Model Lab (`Assets/Editor/ModelLab/ModelLab.cs`)

Editor window (Menu: **HIVE → Model Lab**):
- Fetches live NIM model list
- Benchmarks selected models (quality/reliability/speed)
- Writes `Assets/StreamingAssets/cascade.json` with ranked lists
- Run weekly to keep cascade current

### Agent Runtime

Each agent defined in `Assets/StreamingAssets/agents/{codename}.json` with:
- Role, personality, skills[], tools[], permissions{}
- Workspace theme, avatar config, memory index
- Spawned by `AgentSpawner` on startup

### 3D Headquarters (7 Areas)

| Scene | Purpose |
|-------|---------|
| `CommandChamber.unity` | Central hub — agency overview, agent activity, owner command |
| `Workspace_[Agent].unity` | 12 unique workspaces — role-appropriate geometry |
| `CollabChamber.unity` | Multi-agent collaboration, shared holographic tables |
| `KnowledgeArchive.unity` | Searchable memory nodes, project docs, decisions |
| `Infrastructure.unity` | Visual runtime: model status, queues, breakers |
| `OwnerControl.unity` | Approvals, permissions, emergency controls, audit |
| `Atrium.unity` | Transition space connecting all areas |

## Development

### Running Tests

```bash
# In Unity: Window → General → Test Runner
# Or CLI:
unity -batchmode -runTests -projectPath . -testResults results.xml
```

### Building

```bash
# Windows Standalone (64-bit)
unity -batchmode -buildWindows64Player "Build/HIVE.exe" -projectPath .
```

### Adding a New Agent

1. Create `Assets/StreamingAssets/agents/NEWCODE.json` (see AGENTS.md schema)
2. Create workspace prefab: `Assets/Prefabs/Agents/Workspace_NEWCODE.prefab`
3. Add to `AgentSpawner` registry
4. Run Model Lab to ensure model tier available

## Configuration

### config.json (`Assets/StreamingAssets/config.json`)

```json
{
  "llm": { "gemini_fallback": false },
  "hive": {
    "agent_count": 12,
    "density_mode": "comfortable",
    "animation_level": "full",
    "focus_mode_enabled": true,
    "daily_briefing_time": "08:00"
  }
}
```

### Environment Variables

| Variable | Required | Purpose |
|----------|----------|---------|
| `ANTHROPIC_API_KEY` | Yes* | Claude access (primary) |
| `NVIDIA_API_KEY` | Yes | NIM cascade (free tier) |
| `GEMINI_API_KEY` | No | Gemini fallback tier |
| `HIVE_LLM` | No | Override: "claude" or "nim" |
| `HIVE_LLM_TEST` | No | Enable test mode (no Telegram alerts) |

*Required unless running in `free_only` mode.

## Density Modes

- **Dense** — Maximum panels, compact spacing, all metrics visible
- **Comfortable** — Balanced, default
- **Focus** — ADHD-friendly: hides non-critical UI, reduces animation, highlights single next action

Toggle with `Ctrl+Shift+D` or Command Input: `density focus`

## Command Interface

Access via `~` (tilde) or Command Input panel:

```
@PRIME analyze logs           # Direct agent command
/project "Website Redesign"   # Context: project
task #42 status              # Context: task
/terminal                    # Switch to terminal mode
/voice                       # Push-to-talk
attach logs/error.log        # File attachment
```

## Roadmap

See [ROADMAP.md](docs/ROADMAP.md) for 20-week phased implementation plan.

## License

Proprietary — Internal use only.

## Credits

- NIM Cascade Router ported from [StickExplainers](https://github.com/your/stickexplainers) `stickfx/llm.py`
- Borg aesthetic inspired by *Star Trek* (original IP-safe implementation)
- Unity URP, DOTS, Addressables, Input System