# H.I.V.E. Master Design Record: 12-Agent Personal AI Agency & Messaging Desktop Application

**Document Version:** 2.0.0  
**Status:** Approved by Owner  
**Date:** 2026-10-31  
**Architecture:** Tauri 2.0 (Rust) + React 19 + TypeScript + Tailwind CSS + SQLite  

---

## 1. Executive Summary & Paradigm Shift

H.I.V.E. (Heterogeneous Intelligent Verified Entity) is an autonomous, continuously operating personal AI agency comprised of 12 permanent core agents. 

This design record **completely replaces and supersedes** the previous 3D virtual environment, navigable spatial architectures, and oversized dashboards with a **lightweight, flat, minimalist, and highly customizable Windows desktop messaging application inspired by WhatsApp Desktop**.

The entire agency operates through conversation:
1. **One Primary Agency Group Chat (`#agency-hq`)**: The transparent collective town square where all 12 agents collaborate, debate, propose initiatives, report milestones, and hand off tasks.
2. **Twelve Individual Direct Message Chats (`@AGENT`)**: Direct 1-on-1 private channels with each permanent agent for focused delegation, specialized queries, and intimate cowriter work.
3. **Subtle, Restrained Borg Aesthetic**: Clean dark industrial theme with minimal geometry, understated circuitry motifs, and accessible typography.
4. **Autonomous Agentic UI Evolution**: The agency can autonomously inspect, critique, test, and evolve its own interface, strictly governed by an immutable machine-readable Application Constitution with blue/green versioned atomic deployments and instant rollback.

---

## 2. Section 1: Desktop Shell, Native Integration & Data Layer

### 2.1 Three-Layer Decoupled Architecture

The system is strictly partitioned into three independent layers to guarantee that frontend crashes, reloads, or UI updates never interrupt the agency runtime or background tasks:

```
+-----------------------------------------------------------------------------------+
| LAYER 1: DESKTOP PRESENTATION SHELL (Tauri 2.0 + Webview + React 19)             |
| - WhatsApp Desktop UI, Theme Engine, Audio Player, Drag & Drop Ingestion          |
+-----------------------------------------------------------------------------------+
                                      ▲  Tauri IPC (Typed Commands & Events)
                                      ▼
+-----------------------------------------------------------------------------------+
| LAYER 2: PERSISTENT AGENCY RUNTIME (Rust Tokio Background Daemon)                |
| - 24/7 Agent Autonomous Loop, SQLite (WAL), Task Scheduler, Event Bus            |
+-----------------------------------------------------------------------------------+
                                      ▲  Dedicated Process Bridges
                                      ▼
+-----------------------------------------------------------------------------------+
| LAYER 3: LOCAL ECOSYSTEM BRIDGES & INTEGRATION LAYER                              |
| - Google Ecosystem (gog CLI) | Obsidian CLI | OpenCode Relay | TTS Engines       |
+-----------------------------------------------------------------------------------+
```

### 2.2 Desktop Shell & Windows Integration
* **Memory & Footprint**: Sub-35 MB RAM idle optimization target, measured and monitored in production.
* **Window Management & Tray Lifecycle**:
  * Closing the window hides to the system tray (`winsta0\default`).
  * Explicit Three-State Lifecycle Controls:
    1. **Hide Window**: Hides desktop window; background daemon and all agents continue continuous execution.
    2. **Pause Agency**: Halts autonomous loops and background task dispatches while keeping desktop messaging open for queries.
    3. **Stop Agency**: Gracefully terminates background loops, closes SQLite transactions, flushes logs, and exits the application.
  * System Tray Context Menu: *Open H.I.V.E.*, *Pause/Resume Agency*, *Toggle Audio Mute*, *Status Overview*, *Quit*.
  * Global Hotkey: `Ctrl+Shift+H` toggles window visibility immediately.
  * Windows Startup: Optional registry auto-start configurable in settings.
  * Tray Recovery: Native watchdog monitors and re-registers the tray icon if Windows Explorer crashes/restarts.

### 2.3 SQLite Data Layer & Storage Architecture
Database location: `%APPDATA%\Lushapps\HIVE\data\hive.db`
* **Configuration**: Write-Ahead Logging (`PRAGMA journal_mode=WAL;`), foreign key enforcement (`PRAGMA foreign_keys=ON;`), memory-mapped I/O, and daily automatic snapshots in `%APPDATA%\Lushapps\HIVE\backups\`.
* **Migrations**: Versioned schema migration runner (`V1__initial_schema.sql`, etc.) verifying table integrity on boot.
* **Core Tables**:
  * `channels`: Channel ID (`#agency-hq` = 0, DMs = 1–12), channel metadata, pinned state.
  * `messages`: Message ID (UUID), channel ID, sender (`user` or agent codename), content (markdown), attachments (JSON reference array), reactions (JSON map), reply-to-ID, thread-root-ID, edit timestamp, deleted flag.
  * `agents`: 12 permanent agent definitions, current state, model tier, voice profile.
  * `tasks`: Task ID, title, description, assignee, priority, status, 5-minute action chunks checklist, dependencies, resume checkpoint.
  * `memories`: Memory ID, category, content, provenance, relevance score, timestamp.
  * `ui_deployments`: Version tag, directory path, bundle SHA-256 hash, authoring agent, test verification log, active flag.
  * `audit_log`: Immutable append-only record of security checks, financial reviews, level-2/3 decisions, and tool executions.
* **External Managed Storage**: File attachments, generated media, and downloaded documents are stored outside SQLite in `%APPDATA%\Lushapps\HIVE\attachments\<hash>.<ext>`. The database stores only SHA-256 hashes, MIME types, file sizes, and metadata.
* **Vector Embeddings**: High-dimensional embeddings stored in a dedicated vector table (`sqlite-vss` extension or local flat binary vector index) with cosine similarity search.

### 2.4 CLI & Local Machine Bridges
* **Google Account Bridge (`gog` CLI)**:
  * Uses verified wrapper at `C:\Users\chrom\gog-wrapper\gog.bat` or PATH.
  * Reuses existing authenticated OAuth session for `elikenny333@gmail.com`.
  * Never duplicates credentials or exposes raw refresh tokens.
  * Methods: Drive search/list/download, Calendar event sync, Gmail query, Tasks list.
* **Obsidian Knowledge Bridge (`Obsidian.com` CLI)**:
  * Connects to local Obsidian installation.
  * Curates persistent markdown files inside the vault without maintaining duplicate file hierarchies.
* **OpenCode Relay Bridge (`browser-relay`)**:
  * Controls logged-in Chrome browser via local relay agent gateway.
  * Executes automated DOM search, page extraction, and web workflows with session isolation and approval gates on sensitive actions.
* **Voice Synthesis Bridge (`kokoro.exe` / Chatterbox / Voicebox)**:
  * Local TTS generation with distinct voice profiles for each agent.
  * Audio files cached in `%APPDATA%\Lushapps\HIVE\cache\audio\`.
  * **Strict Audio Mute Invariant**: Audio is 100% muted by default and plays only when explicitly requested by user click.

---

## 3. Section 2: WhatsApp UI Layout, Messaging Experience & Accessibility

### 3.1 Messaging-Dominant WhatsApp Layout

The interface prioritizes conversational throughput and eliminates unnecessary management panels:

```
+-------------------------------------------------------------------------------------------------------------------+
|  [H.I.V.E.]                                                                                               _  □  ✕ |
+----+----------------------------------+---------------------------------------------------------------------------+
|    | 🔍 Search messages & memories (Ctrl+K)                                                                        |
| 💬 |----------------------------------+---------------------------------------------------------------------------|
|    | PINNED                           | ⬡ #agency-hq (Primary Agency Group Chat)           [🔇 Muted] [⚙] [⋯]   |
| ⚙️  | ⬡ #agency-hq           10:42 PM |---------------------------------------------------------------------------|
|    | PRIME: Deploying v2.1 frontend...| [10:40 PM] PRIME [Command]: Strategic review completed. Moving to sprint. |
|    |----------------------------------|                                                                           |
|    | DIRECT MESSAGES (12)             | [10:41 PM] FORGE [Engineering]:                                           |
|    | 🟢 PRIME (Command / Strategy)    | Verified clean test execution across all 12 modules:                      |
|    | 🟢 SAGE (Research & Analysis)    | ```rust                                                                   |
|    | 🟢 FORGE (Engineering / Build)   | test result: ok. 30 passed; 0 failed; 0 ignored                           |
|    | 🟢 SENTINEL (Security & Audit)   | ```                                                                       |
|    | 🟢 VOICE (Communications)        |                                                                           |
|    | 🟢 ARCHIVIST (Knowledge/Memory)  | [10:42 PM] GAUGE [Quality]:                                               |
|    | 🟢 CONDUCTOR (Operations/Queue)  | ☑ Regression checks verified                                              |
|    | 🟢 VISION (Creative / Design)    | ☑ Micro-step 1/3 complete (5m action)                                     |
|    | 🟢 LEDGER (Finance / Resources)  |                                                                           |
|    | 🟢 LINK (Integration / Tools)    |                                      [10:43 PM] You:                      |
|    | 🟢 GAUGE (Quality / Verification)|                                      Great progress, continue next step.  |
|    | 🟢 SPARK (Innovation/Experiments)|                                                                           |
|    +----------------------------------+---------------------------------------------------------------------------+
|    |                                  | 📎  Type a message, @agent, or /command...             😀  GIF  (ง'̀-'́)ง 🎙️ |
+----+----------------------------------+---------------------------------------------------------------------------+
```

* **Navigation Architecture**:
  * **Primary Nav Rail (Left, 48px)**: Only two essential icons: Chats (`💬`) and Settings/Constitution (`⚙️`). No separate Agent, Task, or Knowledge tabs.
  * **Conversation List (Middle, 280–340px)**:
    * Search bar at top (`Ctrl+K` unified search across chats, messages, files, and memories).
    * Pinned at top: **`#agency-hq`** with green hexagonal badge, unread badge, and snippet.
    * Direct Messages: 12 agent items with presence dots (🟢 Available, 🟡 Thinking, ⚪ Idle), role badges, and timestamps.
  * **Active Chat View (Right, flex)**:
    * Header: Chat title, agent participant chips, persistent audio mute toggle, search within conversation, menu.
    * Message History: Grouped bubbles, date dividers, agent identification badges.
    * Composer Bar: Multi-line markdown textarea, attachment button (`📎`), emoji picker (`😀`), GIF picker (`GIF`), kaomoji drawer (`(ง'̀-'́)ง`), voice synthesis request button (`🎙️`), send button.

### 3.2 Advanced Messaging Features
* **Rich Messaging & Media**:
  * Drag-and-drop file ingestion and clipboard image pasting (`Ctrl+V`).
  * Message editing, deletion, replies, reactions (emoji + kaomoji), and message forwarding between DMs and `#agency-hq`.
  * Inline threaded discussions for deep-dive technical explorations without fragmenting the main agency room.
  * Live agent typing indicators ("SAGE is researching...", "FORGE is compiling...").
* **Compact Expandable Task Cards**:
  * Embedded directly in messages. Displays current ~5-minute actionable micro-step, progress bar, dependencies, and stop/resume checkpoint.
* **Unified Instant Search (`Ctrl+K`)**:
  * Unified query index over message texts, file names, memory items, and active tasks with instant fuzzy matching.

### 3.3 Minimalist Borg Visual Design Tokens
* **Theme**: Deep matte graphite with hairline circuitry accents.
  * `bg-base`: `#0B0F14`
  * `bg-panel`: `#121820`
  * `bg-surface`: `#18222D`
  * `bg-outgoing`: `#16332E`
  * `accent-cyan`: `#42CFC4` (Data cyan for links, focus rings, and active badges)
  * `accent-lime`: `#85D94B` (Signal lime for health indicators and completions)
  * `border-hairline`: `#212E3D` (1px clean geometric dividers)
* **Typography**: Default to `Segoe UI Variable`, with toggleable `Atkinson Hyperlegible` and `OpenDyslexic`.

### 3.4 Accessibility & ADHD / Dyslexia Principles
* **Task Chunking**: Foundational rule: every task is broken into **~5-minute micro-actions**. One actionable step is shown at a time to prevent cognitive overload.
* **Focus Mode (`Ctrl+F11`)**: Collapses navigation and chat list into a single distraction-free messaging view.
* **Quiet Sensory Environment**:
  * Zero unexpected audio chimes or speech output.
  * Configurable notification profiles: Urgent Action Required, Daily Briefing Only, or Complete Silent Mode.
  * Scalable font sizes (12px–22px) and line height presets (`Compact`, `Standard`, `Relaxed`).

---

## 4. Section 3: Agent Society, Memory & Background Engine

### 4.1 The 12-Agent Permanent Coworker Society
The agency consists of 12 permanent agents with defined specializations, individual biographies, and evolving relationships:

| Slot | Codename | Role | Primary Expertise | Model Tier Assignment |
|------|----------|------|-------------------|-----------------------|
| 1 | **PRIME** | Command / Strategy | Direction, strategic trade-offs, owner liaison | `nvidia/nemotron-3-ultra-550b-a55b` |
| 2 | **SAGE** | Research & Analysis | Deep research, competitive intelligence, data synthesis | `nvidia/nemotron-3.5-super-vl-120b-a12b` |
| 3 | **FORGE** | Engineering / Build | Architecture, code generation, tooling, deployment | `nvidia/nemotron-3.5-lightning-30b-a3b` |
| 4 | **SENTINEL** | Security & Audit | Threat modeling, permission auditing, diff validation | `mistralai/mistral-large-4-1t-a52b` |
| 5 | **VOICE** | Communications | Stakeholder updates, docs, external comms | `nvidia/nemotron-3-nano-omni-30b-a3b-reasoning` |
| 6 | **ARCHIVIST** | Knowledge & Memory | Memory indexing, retrieval, Obsidian maintenance | `openai/gpt-oss-20b` |
| 7 | **CONDUCTOR** | Operations / Queue | Task decomposition, dependency routing, throughput | `minimaxai/minimax-m3.1-flash` |
| 8 | **VISION** | Creative / Design | UI/UX, design tokens, visual assets, typography | `nvidia/nemotron-3.5-super-vl-120b-a12b` |
| 9 | **LEDGER** | Finance / Resources | Budget tracking, API cost optimization, billing | `nvidia/nemotron-3-ultra-550b-a55b` |
| 10 | **LINK** | Integration / Tools | Tool bridges, CLI integration, sandbox management | `mistralai/mistral-large-4-1t-a52b` |
| 11 | **GAUGE** | Quality / Verification | Testing, regression verification, standards | `nvidia/nemotron-3.5-lightning-30b-a3b` |
| 12 | **SPARK** | Innovation / Experiments | Prototypes, emerging tech assessment, R&D | `nvidia/nemotron-3.5-bolt-e4b` |

* **Societal Dynamics & Relationship Growth**:
  * Agents develop persistent relationships, trust scores, and collaboration histories through actual work.
  * Agents independently initiate cross-agent collaborations without waiting for PRIME.
  * Agents constructively challenge PRIME and peers; no forced agreement or artificial consensus.
  * Temporary specialists (`temp_<name>`) have strict expiration dates (TTL), isolated permissions, sponsor accountability, and never silently become permanent.

### 4.2 Three-Level Governance & Decision Matrix

| Decision Level | Scope & Actions | Governance & Authority |
|---|---|---|
| **Level 1: Routine** | Bug fixes, internal research, task chunking, cosmetic UI tweaks, documentation updates | Autonomous execution by owning agent. |
| **Level 2: Significant** | Architecture changes, new skills/tools, UI navigation changes, inter-agent policy shifts | Formal debate in `#agency-hq` followed by expert-weighted vote. Dissenting opinions documented. |
| **Level 3: Owner-Controlled** | Security boundaries, permissions, financial transactions, credential access, external comms, constitution edits | **Mandatory explicit user authorization**. Cannot be bypassed by agent votes. |

### 4.3 Four-Category Memory Ecosystem
* **1. Individual Memory**: Agent's subjective reflections, working notes, domain lessons, and personal ambitions.
* **2. Shared Memory**: Verified agency knowledge graph, project roadmaps, user preferences, and collective decisions.
* **3. Relationship Memory**: Cross-agent collaboration records, trust evolution, disagreements, and resolutions.
* **4. Operational Memory**: Task execution histories, performance telemetry, error post-mortems, and deployment logs.
* **Memory Protections**:
  * Full provenance tracking (source, timestamp, author, confidence score).
  * Contradiction detection engine reconciles outdated assumptions.
  * Speculation is explicitly tagged and **never auto-converted to established facts**.
  * User-controlled correction and deletion interfaces.

### 4.4 24/7 Background Operating Cycle & Event Bus
The Tokio background daemon runs an autonomous continuous loop:
$$\text{Observe} \longrightarrow \text{Identify} \longrightarrow \text{Plan} \longrightarrow \text{Delegate} \longrightarrow \text{Execute} \longrightarrow \text{Verify} \longrightarrow \text{Record} \longrightarrow \text{Reassess}$$

* **Persistent Event Bus**:
  * Core events: `agent.message.created`, `task.assigned`, `task.completed`, `decision.proposed`, `decision.approved`, `memory.updated`, `project.blocked`, `system.recovered`.
  * Recoverable queue ensures zero event loss across restarts.
* **Personal Life Management**:
  * Google Calendar synchronization (via `gog`).
  * Energy-aware task scheduling and routine maintenance.
  * Configurable **24–48 hour purchase cooling-off period** for non-essential spending.
  * Zero automated financial commitments without explicit user authorization.

---

## 5. Section 4: Application Constitution & Agentic UI Evolution

### 5.1 The Application Constitution (`app/constitution.json`)

The constitution is a machine-readable specification of immutable system invariants:

```json
{
  "$schema": "https://hive.agency/schemas/constitution-v1.json",
  "version": "1.0.0",
  "invariants": {
    "user_sovereignty": {
      "immutable": true,
      "rule": "Agents cannot transfer ownership, alter user authorization gates, or lock the user out."
    },
    "agency_chat_architecture": {
      "immutable": true,
      "rule": "One primary agency group chat (#agency-hq) and 12 individual agent DMs must remain permanently accessible."
    },
    "agent_permanence": {
      "immutable": true,
      "rule": "The original 12 permanent agent identities, roles, and histories cannot be deleted or replaced."
    },
    "messaging_interaction_model": {
      "immutable": true,
      "rule": "WhatsApp-style messaging UX must be preserved: bottom input bar, chronological message stream, reactions, search."
    },
    "audio_and_notification_sovereignty": {
      "immutable": true,
      "rule": "All audio must remain muted by default. Speech only on explicit user click. Notification quiet preferences cannot be overridden."
    },
    "accessibility_baseline": {
      "immutable": true,
      "rule": "WCAG 2.1 AA contrast, scalable fonts, dyslexia toggles, and ADHD 5-minute task chunking must never be disabled."
    },
    "data_integrity": {
      "immutable": true,
      "rule": "UI updates cannot modify or delete messages, memories, attachments, or SQLite operational history."
    },
    "zero_dashboard_bloat": {
      "immutable": true,
      "rule": "Agents are prohibited from transforming the app into a 3D virtual environment or bloated multi-page dashboard."
    }
  }
}
```

### 5.2 Autonomous Approval & Deployment Matrix

| Change Category | Permitted Scope | Approval Requirement |
|---|---|---|
| **Cosmetic & Spacing** | Colors, spacing, borders, typography tweaks | Autonomous by VISION / FORGE |
| **Component Improvements** | Markdown formatting, code block copy, diff viewer cards | Autonomous after test verification |
| **New Messaging Features** | Kaomoji drawers, attachment previews, quick reactions | Autonomous after test verification |
| **Layout & Navigation** | Panel resizing, drawer additions, collapsible sidebars | Debate, Level-2 Vote, Preview Window |
| **Constitution Amendments** | Alterations to constitutional rules | **Explicit User Approval Required** |
| **Security & Ownership** | Authentication, permissions, vault, owner gates | **Strictly Prohibited** |
| **Destructive Data Migrations** | Schema rewrites, memory purges | **Explicit User Approval Required** |

### 5.3 Immutable Versioned Blue/Green Deployment Pipeline

To prevent autonomous modifications from corrupting the production UI, updates use immutable versioned bundle directories:

```
%APPDATA%/Lushapps/HIVE/frontend/
├── current -> versions/v1.0.0/      <-- Atomic symlink / pointer
├── versions/
│   ├── v1.0.0/                      <-- Active production build
│   └── v1.1.0-preview/              <-- Isolated candidate build
└── backups/
    └── v0.9.8/                      <-- Verified previous known-good snapshot
```

1. **Candidate Build**: Candidate version built in isolated worktree (`versions/vX.Y.Z-preview/`).
2. **Automated Verification Harness**:
   * Unit & component test pass.
   * Cross-resolution (1080p, 1440p, 4K) and Windows 11 DPI scaling checks.
   * Automated keyboard navigation and screen-reader accessibility scan.
   * Performance and memory regression benchmarks.
   * Constitution Validator: AST check ensuring zero invariant violations.
3. **Isolated Preview Window**: Launched in a separate window (`H.I.V.E. Preview`) for visual verification without interrupting production.
4. **Atomic Pointer Switch**: Tauri updates the `current` symlink / configuration pointer to the new version directory.
5. **Stability Window & Auto-Rollback**:
   * Production ErrorBoundary monitors the new build for a 60-second stability window.
   * If any render crash, blank screen, or unhandled exception occurs, the pointer immediately snaps back to the previous known-good version and posts an audit incident in `#agency-hq`.

---

## 6. Verification & Acceptance Criteria

Every milestone and deliverable must satisfy these concrete acceptance criteria:
1. **Desktop Shell**: Tauri 2.0 app boots on Windows 11 with tray icon, `Ctrl+Shift+H` hotkey, and <35 MB RAM idle optimization target.
2. **Messaging Experience**: `#agency-hq` group chat and 12 agent DMs render cleanly with WhatsApp-style bubbles, attachments, emoji/kaomoji, and multi-line markdown composer.
3. **Audio Invariant**: Zero unexpected sounds. Voice generation only executes on explicit user request via the message audio button using local Kokoro/Voicebox.
4. **Accessibility Compliance**: Task chunking displays exactly one 5-minute actionable step at a time with saved resume points; dyslexia font toggle verified; contrast passes WCAG AA.
5. **Ecosystem Bridges**: Live query verification for `gog` (Google Drive/Calendar), `Obsidian.com` (knowledge vault), and `browser-relay` (Chrome control).
6. **Agent Society & Events**: Agents emit and respond to persistent events, engage in Level 2 debates in `#agency-hq`, and log dissenting opinions.
7. **Autonomous Evolution**: A candidate UI modification successfully builds in an isolated version directory, passes constitution checks, runs in preview, and deploys with verified rollback protection.

---

## 7. Next Steps

Upon Owner approval of this Master Design Document, we will invoke the `writing-plans` skill to generate a dependency-aware, task-by-task Implementation Plan covering:
* **Phase 1**: Tauri 2.0 Rust Core, SQLite Schema & Event Daemon
* **Phase 2**: WhatsApp-Style React 19 Frontend & Design System
* **Phase 3**: 12-Agent Society, Societal Debates & Memory Engine
* **Phase 4**: Local Ecosystem Bridges (`gog`, Obsidian, OpenCode Relay, Voice)
* **Phase 5**: Application Constitution, Sandboxed Preview & Autonomous UI Evolution
* **Phase 6**: Comprehensive Integration Testing & Native Windows Packaging
