import React, { useState, useEffect } from "react";
import {
  Constitution,
  BridgeStatus,
  RelationshipEdge,
  TempSpecialist,
  UiDeployment,
  AuditEntry,
  MemoryItem,
} from "../types";
import {
  Shield,
  Type,
  Play,
  Pause,
  CheckCircle,
  GitBranch,
  RotateCcw,
  Eye,
  Database,
  EyeOff,
  Power,
  Trash2,
} from "lucide-react";
import { api } from "../services/api";

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  constitution?: Constitution | null;
  bridgeStatus?: BridgeStatus | null;
  agencyState: string;
  onSetAgencyState: (state: string) => void;
  currentFont: string;
  onSelectFont: (font: string) => void;
  fontSize: number;
  onSetFontSize: (size: number) => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({
  isOpen,
  onClose,
  constitution,
  bridgeStatus,
  agencyState,
  onSetAgencyState,
  currentFont,
  onSelectFont,
  fontSize,
  onSetFontSize,
}) => {
  const [tab, setTab] = useState<
    "constitution" | "deployments" | "society" | "accessibility" | "bridges" | "lifecycle"
  >("constitution");
  const [deployments, setDeployments] = useState<UiDeployment[]>([]);
  const [relationships, setRelationships] = useState<RelationshipEdge[]>([]);
  const [specialists, setSpecialists] = useState<TempSpecialist[]>([]);
  const [memories, setMemories] = useState<MemoryItem[]>([]);
  const [auditLogs, setAuditLogs] = useState<AuditEntry[]>([]);
  const [statusBanner, setStatusBanner] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      api.getUiDeployments().then(setDeployments).catch(() => {});
      api.getRelationships().then(setRelationships).catch(() => {});
      api.getTempSpecialists().then(setSpecialists).catch(() => {});
      api.getMemories().then(setMemories).catch(() => {});
      api.getAuditLog(25).then(setAuditLogs).catch(() => {});
    }
  }, [isOpen, tab]);

  if (!isOpen) return null;

  const handleDeployNewVersion = async () => {
    try {
      const nextVer = `1.0.${deployments.length + 1}`;
      await api.deployUiVersion(
        nextVer,
        "VISION",
        "Autonomous UI refinement verified against Constitution v1.0.0",
        "autonomous"
      );
      const updated = await api.getUiDeployments();
      setDeployments(updated);
      setStatusBanner(`Deployed v${nextVer} with verified SHA-256 bundle & backup.`);
    } catch (e) {
      setStatusBanner(`Deployment blocked: ${String(e)}`);
    }
  };

  const handleRollback = async () => {
    try {
      const target = await api.rollbackUiVersion("Manual user rollback from Settings");
      const updated = await api.getUiDeployments();
      setDeployments(updated);
      setStatusBanner(`Rolled back atomically to v${target}.`);
    } catch (e) {
      setStatusBanner(`Rollback notice: ${String(e)}`);
    }
  };

  const handleBackupNow = async () => {
    try {
      const path = await api.createDbBackup();
      setStatusBanner(`Online SQLite backup created: ${path}`);
    } catch (e) {
      setStatusBanner(`Backup error: ${String(e)}`);
    }
  };

  const handleDeleteMemory = async (id: string) => {
    await api.deleteMemory(id);
    const updated = await api.getMemories();
    setMemories(updated);
  };

  return (
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-4 select-none">
      <div className="bg-hive-panel border border-hive-border rounded-xl w-full max-w-3xl max-h-[88vh] flex flex-col shadow-2xl overflow-hidden font-segoe">
        {/* Modal Header */}
        <div className="p-4 border-b border-hive-border flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <Shield className="w-5 h-5 text-hive-cyan" />
            <h2 className="text-sm font-semibold text-hive-brightText uppercase tracking-wider">
              Agency Settings, Governance & Constitution
            </h2>
          </div>
          <button
            onClick={onClose}
            className="text-hive-dimText hover:text-hive-brightText text-sm px-2 py-1 rounded hover:bg-hive-surface"
          >
            ✕
          </button>
        </div>

        {/* Sub-navigation Tabs */}
        <div className="flex border-b border-hive-border px-4 bg-hive-base/50 overflow-x-auto">
          {[
            { id: "constitution", label: "Constitution" },
            { id: "deployments", label: "UI Evolution & Rollback" },
            { id: "society", label: "Society & Memory" },
            { id: "accessibility", label: "Accessibility" },
            { id: "bridges", label: "CLI Bridges" },
            { id: "lifecycle", label: "Daemon Lifecycle" },
          ].map((t) => (
            <button
              key={t.id}
              onClick={() => {
                setTab(t.id as any);
                setStatusBanner(null);
              }}
              className={`py-2.5 px-3 text-xs font-medium border-b-2 whitespace-nowrap transition-colors ${
                tab === t.id
                  ? "border-hive-cyan text-hive-cyan"
                  : "border-transparent text-hive-dimText hover:text-hive-brightText"
              }`}
            >
              {t.label}
            </button>
          ))}
        </div>

        {statusBanner && (
          <div className="px-4 py-2 bg-hive-surface border-b border-hive-cyan/40 text-xs text-hive-cyan flex items-center justify-between">
            <span>{statusBanner}</span>
            <button onClick={() => setStatusBanner(null)} className="text-hive-dimText">
              ✕
            </button>
          </div>
        )}

        {/* Tab Content */}
        <div className="p-5 flex-1 overflow-y-auto space-y-4">
          {tab === "constitution" && (
            <div className="space-y-3">
              <div className="p-3 bg-hive-surface rounded-lg border border-hive-border">
                <span className="text-xs font-semibold text-hive-lime flex items-center gap-1.5">
                  <CheckCircle className="w-3.5 h-3.5" /> Immutable Constitution v1.0.0
                </span>
                <p className="text-xs text-hive-dimText mt-1">
                  These invariants are machine-verified on every autonomous commit and Blue/Green deployment. Agents cannot alter or bypass them.
                </p>
              </div>

              <div className="space-y-2">
                {constitution?.invariants &&
                  Object.entries(constitution.invariants).map(([key, inv]) => (
                    <div
                      key={key}
                      className="p-3 bg-hive-surface/60 rounded border border-hive-border text-xs flex flex-col space-y-1"
                    >
                      <div className="flex items-center justify-between">
                        <span className="font-mono text-hive-cyan font-semibold">{key}</span>
                        <span className="text-[10px] bg-hive-lime/10 text-hive-lime px-1.5 py-0.5 rounded font-mono border border-hive-lime/30">
                          IMMUTABLE
                        </span>
                      </div>
                      <p className="text-hive-brightText leading-relaxed">{inv.rule}</p>
                    </div>
                  ))}
              </div>
            </div>
          )}

          {tab === "deployments" && (
            <div className="space-y-4 text-xs">
              <div className="p-3 bg-hive-surface rounded-lg border border-hive-border flex items-center justify-between">
                <div>
                  <div className="font-semibold text-hive-brightText flex items-center gap-1.5">
                    <GitBranch className="w-4 h-4 text-hive-lime" /> Blue/Green UI Version Manager
                  </div>
                  <p className="text-hive-dimText mt-0.5">
                    Every UI bundle is verified against Constitution v1.0.0, SHA-256 hashed, and backed up before atomic promotion.
                  </p>
                </div>
                <div className="flex items-center space-x-2">
                  <button
                    onClick={() => api.openPreviewWindow("preview")}
                    className="px-2.5 py-1.5 rounded bg-hive-base border border-hive-cyan/40 text-hive-cyan hover:bg-hive-cyan/10 flex items-center gap-1"
                  >
                    <Eye className="w-3.5 h-3.5" /> Preview Window
                  </button>
                  <button
                    onClick={handleDeployNewVersion}
                    className="px-2.5 py-1.5 rounded bg-hive-lime text-hive-base font-semibold hover:bg-hive-lime/90"
                  >
                    + Verify & Snapshot
                  </button>
                  <button
                    onClick={handleRollback}
                    className="px-2.5 py-1.5 rounded bg-red-500/20 border border-red-500/40 text-red-300 hover:bg-red-500/30 flex items-center gap-1"
                  >
                    <RotateCcw className="w-3.5 h-3.5" /> Rollback
                  </button>
                </div>
              </div>

              <div className="space-y-2">
                {deployments.map((d) => (
                  <div
                    key={d.version}
                    className="p-3 rounded bg-hive-surface/60 border border-hive-border flex items-center justify-between"
                  >
                    <div>
                      <div className="flex items-center space-x-2">
                        <span className="font-mono font-bold text-hive-cyan">v{d.version}</span>
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-hive-base border border-hive-border text-hive-dimText font-mono">
                          @{d.author_agent}
                        </span>
                        <span
                          className={`text-[10px] px-1.5 py-0.5 rounded font-mono ${
                            d.status === "active"
                              ? "bg-hive-lime/15 text-hive-lime border border-hive-lime/30"
                              : "bg-hive-base text-hive-dimText"
                          }`}
                        >
                          {d.status.toUpperCase()}
                        </span>
                      </div>
                      <div className="text-hive-brightText mt-1">{d.changelog}</div>
                      <div className="text-[10px] text-hive-dimText font-mono mt-0.5">
                        SHA-256: {d.bundle_hash.slice(0, 24)}...
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {tab === "society" && (
            <div className="space-y-4 text-xs">
              {/* Relationship Graph Edges */}
              <div className="space-y-2">
                <div className="font-semibold text-hive-brightText uppercase tracking-wider text-[11px]">
                  Inter-Agent Relationship Graph ({relationships.length} active edges)
                </div>
                <div className="grid grid-cols-2 gap-2">
                  {relationships.map((r, i) => (
                    <div
                      key={i}
                      className="p-2.5 rounded bg-hive-surface border border-hive-border flex flex-col space-y-1"
                    >
                      <div className="flex items-center justify-between">
                        <span className="font-mono text-hive-cyan font-semibold">
                          @{r.source_agent} → @{r.target_agent}
                        </span>
                        <span className="text-[10px] px-1.5 py-0.2 rounded bg-hive-base border border-hive-lime/30 text-hive-lime font-mono">
                          {r.edge_type} ({r.weight.toFixed(2)})
                        </span>
                      </div>
                      {r.notes && <p className="text-[11px] text-hive-dimText">{r.notes}</p>}
                    </div>
                  ))}
                </div>
              </div>

              {/* Temporary Specialists */}
              <div className="space-y-2 pt-2 border-t border-hive-border">
                <div className="font-semibold text-hive-brightText uppercase tracking-wider text-[11px]">
                  Temporary Specialists (temp_ prefix, auto-retiring TTL)
                </div>
                {specialists.length === 0 ? (
                  <div className="text-hive-dimText italic">
                    No active temporary specialists. Use `/spawn_temp name role SPONSOR` in chat.
                  </div>
                ) : (
                  <div className="space-y-1.5">
                    {specialists.map((s) => (
                      <div
                        key={s.id}
                        className="p-2 rounded bg-hive-surface border border-hive-cyan/30 flex items-center justify-between"
                      >
                        <div>
                          <span className="font-mono text-hive-lime font-semibold">
                            {s.codename}
                          </span>{" "}
                          <span className="text-hive-dimText">({s.role})</span>
                          <div className="text-[10px] text-hive-dimText font-mono">
                            Sponsor: @{s.sponsor_agent} • Expires: {s.expires_at}
                          </div>
                        </div>
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-hive-cyan/10 text-hive-cyan font-mono">
                          TEMP ({s.status})
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {/* 4-Tier Memory Inspector with User Delete Control */}
              <div className="space-y-2 pt-2 border-t border-hive-border">
                <div className="font-semibold text-hive-brightText uppercase tracking-wider text-[11px]">
                  4-Tier Memory Ecosystem (Provenance & Confidence)
                </div>
                <div className="space-y-1.5 max-h-44 overflow-y-auto">
                  {memories.map((m) => (
                    <div
                      key={m.id}
                      className="p-2.5 rounded bg-hive-surface/60 border border-hive-border flex items-start justify-between gap-2"
                    >
                      <div>
                        <div className="flex items-center space-x-2">
                          <span className="text-[10px] uppercase font-mono px-1.5 py-0.2 rounded bg-hive-base text-hive-cyan border border-hive-cyan/30">
                            {m.category}
                          </span>
                          <span className="text-[10px] text-hive-lime font-mono">
                            Confidence: {(m.confidence * 100).toFixed(0)}%
                          </span>
                        </div>
                        <p className="text-hive-brightText mt-1">{m.content}</p>
                        <div className="text-[10px] text-hive-dimText font-mono mt-0.5">
                          Provenance: {m.provenance}
                        </div>
                      </div>
                      <button
                        onClick={() => handleDeleteMemory(m.id)}
                        className="text-hive-dimText hover:text-red-400 p-1"
                        title="Delete memory record"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {tab === "accessibility" && (
            <div className="space-y-4 text-xs">
              <div className="space-y-2">
                <label className="font-semibold text-hive-brightText flex items-center gap-1.5">
                  <Type className="w-4 h-4 text-hive-cyan" /> Typography Preset
                </label>
                <div className="grid grid-cols-3 gap-2">
                  {[
                    { id: "segoe", label: "Segoe UI Variable", desc: "Default Windows 11" },
                    { id: "atkinson", label: "Atkinson Hyperlegible", desc: "Braille Institute" },
                    { id: "dyslexic", label: "OpenDyslexic", desc: "Weighted Baseline" },
                  ].map((f) => (
                    <button
                      key={f.id}
                      onClick={() => onSelectFont(f.id)}
                      className={`p-2.5 rounded border text-left transition-all ${
                        currentFont === f.id
                          ? "bg-hive-surface border-hive-cyan text-hive-brightText"
                          : "bg-hive-surface/40 border-hive-border text-hive-dimText hover:border-hive-border/80"
                      }`}
                    >
                      <div className="font-medium text-hive-brightText">{f.label}</div>
                      <div className="text-[10px] text-hive-dimText mt-0.5">{f.desc}</div>
                    </button>
                  ))}
                </div>
              </div>

              <div className="space-y-2 pt-2 border-t border-hive-border">
                <div className="flex items-center justify-between">
                  <label className="font-semibold text-hive-brightText">
                    Font Scaling: {fontSize}px
                  </label>
                  <span className="text-hive-dimText font-mono text-[11px]">(12px – 20px)</span>
                </div>
                <input
                  type="range"
                  min="12"
                  max="20"
                  step="1"
                  value={fontSize}
                  onChange={(e) => onSetFontSize(Number(e.target.value))}
                  className="w-full accent-hive-cyan bg-hive-surface h-1.5 rounded-lg appearance-none cursor-pointer"
                />
              </div>

              <div className="p-3 bg-hive-surface rounded-lg border border-hive-border space-y-1">
                <span className="font-semibold text-hive-brightText">
                  Sensory Sovereignty & Keyboard Shortcuts
                </span>
                <p className="text-hive-dimText leading-relaxed">
                  • Audio speech generation is 100% muted by default.
                  <br />• Global Window Toggle: <kbd className="font-mono text-hive-cyan">Ctrl+Shift+H</kbd>
                  <br />• Unified Search: <kbd className="font-mono text-hive-cyan">Ctrl+K</kbd>
                  <br />• Focus Mode: <kbd className="font-mono text-hive-cyan">Ctrl+F11</kbd>
                </p>
              </div>
            </div>
          )}

          {tab === "bridges" && (
            <div className="space-y-3 text-xs">
              <div className="p-3 rounded bg-hive-surface border border-hive-border flex items-center justify-between">
                <div>
                  <div className="font-semibold text-hive-brightText">Google Account Bridge (gog)</div>
                  <div className="text-hive-dimText font-mono text-[11px] mt-0.5">
                    Account: {bridgeStatus?.google_account || "elikenny333@gmail.com"}
                  </div>
                </div>
                <span className="px-2 py-0.5 bg-hive-lime/10 text-hive-lime border border-hive-lime/30 rounded font-mono text-[10px]">
                  CONNECTED
                </span>
              </div>

              <div className="p-3 rounded bg-hive-surface border border-hive-border flex items-center justify-between">
                <div>
                  <div className="font-semibold text-hive-brightText">Obsidian Vault Bridge</div>
                  <div className="text-hive-dimText font-mono text-[11px] mt-0.5">
                    Direct markdown note read/write/search inside local vault (`/obsidian`)
                  </div>
                </div>
                <span className="px-2 py-0.5 bg-hive-cyan/10 text-hive-cyan border border-hive-cyan/30 rounded font-mono text-[10px]">
                  ACTIVE
                </span>
              </div>

              <div className="p-3 rounded bg-hive-surface border border-hive-border flex items-center justify-between">
                <div>
                  <div className="font-semibold text-hive-brightText">OpenCode Chrome Relay</div>
                  <div className="text-hive-dimText font-mono text-[11px] mt-0.5">
                    Live browser inspection & navigation via `browser-relay` (`/browser`)
                  </div>
                </div>
                <span className="px-2 py-0.5 bg-hive-cyan/10 text-hive-cyan border border-hive-cyan/30 rounded font-mono text-[10px]">
                  ACTIVE
                </span>
              </div>

              <div className="p-3 rounded bg-hive-surface border border-hive-border flex items-center justify-between">
                <div>
                  <div className="font-semibold text-hive-brightText">Local Neural Speech & WAV Synthesizer</div>
                  <div className="text-hive-dimText font-mono text-[11px] mt-0.5">
                    On-demand WAV synthesis with base64 audio playback
                  </div>
                </div>
                <span className="px-2 py-0.5 bg-hive-lime/10 text-hive-lime border border-hive-lime/30 rounded font-mono text-[10px]">
                  READY (MUTED)
                </span>
              </div>
            </div>
          )}

          {tab === "lifecycle" && (
            <div className="space-y-4 text-xs">
              <div className="p-3 rounded bg-hive-surface border border-hive-border space-y-2">
                <div className="font-semibold text-hive-brightText">
                  Current Agency Daemon State:{" "}
                  <span className="text-hive-lime uppercase">{agencyState}</span>
                </div>
                <p className="text-hive-dimText leading-relaxed">
                  The persistent Rust Tokio daemon runs 24/7 independently of the desktop window. Choose between hiding the window to tray, pausing background routines, or stopping the agency completely.
                </p>
              </div>

              {/* Three-State Quit / Lifecycle Controls */}
              <div className="grid grid-cols-3 gap-2">
                <button
                  onClick={() => api.hideMainWindow()}
                  className="p-3 rounded border bg-hive-surface border-hive-border text-hive-brightText hover:border-hive-cyan flex items-center justify-center gap-1.5 font-medium"
                >
                  <EyeOff className="w-4 h-4 text-hive-cyan" /> Hide Window (Tray)
                </button>

                {agencyState === "running" ? (
                  <button
                    onClick={() => onSetAgencyState("paused")}
                    className="p-3 rounded border bg-hive-surface border-yellow-500/40 text-yellow-300 hover:bg-yellow-500/10 flex items-center justify-center gap-1.5 font-medium"
                  >
                    <Pause className="w-4 h-4" /> Pause Agency
                  </button>
                ) : (
                  <button
                    onClick={() => onSetAgencyState("running")}
                    className="p-3 rounded border bg-hive-lime text-hive-base border-hive-lime flex items-center justify-center gap-1.5 font-semibold"
                  >
                    <Play className="w-4 h-4" /> Resume Agency
                  </button>
                )}

                <button
                  onClick={() => api.stopAgencyAndQuit()}
                  className="p-3 rounded border bg-red-500/15 border-red-500/40 text-red-300 hover:bg-red-500/25 flex items-center justify-center gap-1.5 font-medium"
                >
                  <Power className="w-4 h-4" /> Stop Agency & Quit
                </button>
              </div>

              {/* Database Online Backup & Audit Log */}
              <div className="pt-2 border-t border-hive-border flex items-center justify-between">
                <div>
                  <div className="font-semibold text-hive-brightText">SQLite WAL Online Snapshot</div>
                  <div className="text-hive-dimText text-[11px]">
                    Automatic daily snapshots + on-demand `VACUUM INTO` backup
                  </div>
                </div>
                <button
                  onClick={handleBackupNow}
                  className="px-3 py-1.5 rounded bg-hive-surface border border-hive-cyan/40 text-hive-cyan hover:bg-hive-cyan/10 flex items-center gap-1.5"
                >
                  <Database className="w-3.5 h-3.5" /> Backup Now
                </button>
              </div>

              <div className="space-y-1.5 pt-2 border-t border-hive-border">
                <div className="font-semibold text-hive-brightText uppercase tracking-wider text-[11px]">
                  Recent Security & Governance Audit Log
                </div>
                <div className="max-h-36 overflow-y-auto space-y-1 font-mono text-[10px]">
                  {auditLogs.map((a) => (
                    <div
                      key={a.id}
                      className="px-2 py-1 rounded bg-hive-base border border-hive-border flex items-center justify-between"
                    >
                      <span className="text-hive-cyan">
                        [{a.actor}] {a.action}
                      </span>
                      <span className="text-hive-dimText truncate max-w-[260px]">
                        {a.details || a.target}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
