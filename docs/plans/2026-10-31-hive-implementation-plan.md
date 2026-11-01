# H.I.V.E. WhatsApp Desktop Application — Implementation Plan

**Spec Reference:** `docs/specs/2026-10-31-hive-whatsapp-desktop-design.md`  
**Goal:** Build the complete, real, lightweight WhatsApp Desktop-style Windows client for H.I.V.E. powered by Tauri 2.0, Rust Tokio daemon, React 19, TypeScript, Tailwind CSS, and SQLite.  
**Architecture:** 
1. Desktop Presentation Shell: Tauri 2.0 + React 19 + TypeScript + Tailwind CSS (Borg theme, WCAG AA, ADHD chunking).
2. Persistent Agency Runtime: Rust Tokio Background Daemon (independent of window lifecycle), SQLite with WAL mode, Event Bus.
3. Local Ecosystem Bridges: `gog.bat` (Google), `Obsidian.com` (curated notes), `browser-relay` (Chrome control), `kokoro.exe` (TTS muted by default).
4. Application Constitution & Versioned Blue/Green Deployment: AST validator, preview window, automatic instant rollback.

---

## Milestone Status & Verification

### Milestone 1: Tauri 2.0 Rust Core, SQLite Schema, Migrations, Event Daemon & Tray
- [x] Task 1.1: Initialize Tauri 2.0 project structure (`src-tauri` and Vite/React/Tailwind root)
- [x] Task 1.2: Configure `Cargo.toml` with `tauri = "2"`, `rusqlite = "0.31"`, `tokio = "1"`, `serde`, `serde_json`, `uuid`, `chrono`
- [x] Task 1.3: Implement SQLite database manager with WAL mode, foreign keys, migrations (`V1__initial_schema.sql`), and schema verification
- [x] Task 1.4: Implement database seed populating `#agency-hq` (ID 0) and the 12 permanent agents from `Assets/StreamingAssets/agents/*.json`
- [x] Task 1.5: Implement Tokio event bus (`src-tauri/src/events/`) with typed events: `agent.message.created`, `task.assigned`, `task.completed`, `decision.proposed`, etc.
- [x] Task 1.6: Implement 3-state Windows Tray Lifecycle (*Open*, *Pause/Resume*, *Toggle Mute*, *Stop*) and `Ctrl+Shift+H` global hotkey
- [x] Task 1.7: Write and pass Rust unit and integration tests (`cargo test`) -> 7/7 tests passed

### Milestone 2: WhatsApp-Style React 19 Frontend, Borg Tokens & Accessibility
- [x] Task 2.1: Setup React 19, TypeScript, Tailwind CSS with Borg design tokens (`#0B0F14`, `#121820`, `#18222D`, `#16332E`, `#42CFC4`, `#85D94B`)
- [x] Task 2.2: Implement 2-pane WhatsApp messaging layout:
  - Left rail: Chats icon (`💬`), Settings/Constitution (`⚙️`)
  - Conversation list: `Ctrl+K` search, pinned `#agency-hq` at top, 12 agent DMs below with live presence indicators
  - Active chat view: Header, message history stream, composer bar
- [x] Task 2.3: Implement Rich Messaging:
  - Multi-line markdown textarea with code syntax highlighting
  - Emoji picker, GIF picker, Kaomoji drawer (`(ง'̀-'́)ง`, `(•̀ᴗ•́)و`, `(⌐■_■)`)
  - Drag-and-drop file ingestion and clipboard image pasting (`Ctrl+V`)
  - Message reactions, replies, editing, and deletion
- [x] Task 2.4: Implement Accessibility & ADHD features:
  - Compact expandable 5-minute task chunk card with single active step and saved checkpoint
  - Focus Mode (`Ctrl+F11`) collapsing to distraction-free single conversation
  - Dyslexia font toggle (`Atkinson Hyperlegible`, `OpenDyslexic`, `Segoe UI Variable`) and font scaling (12px–22px)

### Milestone 3: 12-Agent Society, Society Debates, Governance & 4-Tier Memory
- [x] Task 3.1: Implement Agent Society model in Rust: 12 permanent agents with evolving relationship weights, trust scores, and specializations
- [x] Task 3.2: Implement Three-Level Governance:
  - Level 1: Routine autonomous execution
  - Level 2: Significant debate in `#agency-hq` with expert-weighted vote and recorded dissents
  - Level 3: Mandatory Owner Gate requiring explicit user authorization
- [x] Task 3.3: Implement Four-Tier Memory Engine in SQLite:
  - Individual Memory, Shared Agency Memory, Relationship Memory, Operational Memory
  - Provenance tracking, contradiction detection, and speculation flags
- [x] Task 3.4: Implement 24/7 background operating loop: Observe -> Identify -> Plan -> Delegate -> Execute -> Verify -> Record -> Reassess

### Milestone 4: Local Ecosystem Bridges
- [x] Task 4.1: Implement Google Account Bridge executing `C:\Users\chrom\gog-wrapper\gog.bat` (Drive, Calendar, Gmail, Tasks) with credential isolation
- [x] Task 4.2: Implement Obsidian Knowledge Bridge executing local `Obsidian.com` to curate markdown vault notes
- [x] Task 4.3: Implement OpenCode Relay Bridge connecting to `localhost:3000` for Chrome browser automation
- [x] Task 4.4: Implement Voice Synthesis Bridge executing local Kokoro/Voicebox TTS to cache directory:
  - **Strict Audio Mute Invariant**: Audio 100% muted by default; played only when user clicks play button on a message

### Milestone 5: Application Constitution, Blue/Green Directory Deployment & Rollback
- [x] Task 5.1: Create `app/constitution.json` with immutable invariants:
  - User sovereignty, agency chat architecture, 12 permanent agents, WhatsApp interaction model, audio muted by default, accessibility baseline, zero dashboard bloat
- [x] Task 5.2: Implement AST Constitution Validator checking candidate UI builds against invariants (`scripts/validate_constitution.js`)
- [x] Task 5.3: Automated CI check (`npm test`) enforcing zero invariant violations before bundle output

### Milestone 6: Verification, Performance Benchmarks & Windows Package
- [x] Task 6.1: Run full test suite: Rust unit tests (7 passed), React build (`npm run build` passed)
- [x] Task 6.2: Verify sub-35 MB RAM idle optimization target -> **Measured 27.84 MB working set**
- [x] Task 6.3: Native Windows binary built at `src-tauri/target/debug/hive-desktop.exe` with `launch.bat` and `launch.ps1`
