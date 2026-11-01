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
import { api } from "../services/api";
import { M3Icon } from "./M3Icon";

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
  isMuted?: boolean;
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
  isMuted = true,
}) => {
  const [tab, setTab] = useState<
    "constitution" | "deployments" | "society" | "accessibility" | "bridges" | "lifecycle"
  >("constitution");
  const [deployments, setDeployments] = useState<UiDeployment[]>([]);
  const [relationships, setRelationships] = useState<RelationshipEdge[]>([]);
  const [specialists, setSpecialists] = useState<TempSpecialist[]>([]);
  const [memories, setMemories] = useState<MemoryItem[]>([]);
  const [auditLogs, setAuditLogs] = useState<AuditEntry[]>([]);
  const [statusBanner, setBanner] = useState<{ text: string; error: boolean } | null>(null);
  const setStatusBanner = (text: string | null, error = false) => setBanner(text === null ? null : { text, error });
  const [liveBridges, setLiveBridges] = useState<BridgeStatus | null>(bridgeStatus ?? null);
  const [bridgesLoading, setBridgesLoading] = useState<boolean>(false);
  const [deploying, setDeploying] = useState<boolean>(false);
  const [apiKeyInput, setApiKeyInput] = useState<string>("");
  const [savingKey, setSavingKey] = useState<boolean>(false);
  const [showKeyForm, setShowKeyForm] = useState<boolean>(false);

  const handleSaveKey = async () => {
    if (!apiKeyInput.trim()) return;
    setSavingKey(true);
    try {
      const msg = await api.setNimApiKey(apiKeyInput.trim());
      setApiKeyInput("");
      setShowKeyForm(false);
      setStatusBanner(`${msg}. Agents can talk now — type /activate in #agency-hq if they never checked in.`);
      await refreshBridges();
    } catch (e) {
      setStatusBanner(`Key not saved: ${String(e)}`, true);
    } finally {
      setSavingKey(false);
    }
  };

  const refreshBridges = async () => {
    setBridgesLoading(true);
    try {
      setLiveBridges(await api.getBridgeStatus());
    } catch (e) {
      setStatusBanner(`Bridge check failed: ${String(e)}`, true);
    } finally {
      setBridgesLoading(false);
    }
  };

  const [notifyDms, setNotifyDms] = useState<boolean>(false);

  useEffect(() => {
    if (isOpen) {
      api
        .getSetting("desktop_notifications")
        .then((v) => setNotifyDms(v === "dms"))
        .catch(() => {});
    }
  }, [isOpen]);

  const handleToggleNotify = async () => {
    const next = !notifyDms;
    setNotifyDms(next);
    try {
      await api.setSetting("desktop_notifications", next ? "dms" : "off");
    } catch (e) {
      setNotifyDms(!next);
      setStatusBanner(`Couldn't save notification setting: ${String(e)}`, true);
    }
  };

  useEffect(() => {
    if (bridgeStatus) setLiveBridges(bridgeStatus);
  }, [bridgeStatus]);

  useEffect(() => {
    if (isOpen && tab === "bridges") refreshBridges();
  }, [isOpen, tab]);

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
    setDeploying(true);
    try {
      const stamp = new Date().toISOString().replace(/[-:]/g, "").replace(/\..+$/, "").replace("T", "-");
      const nextVer = `snapshot-${stamp}`;
      const dep = await api.deployUiVersion(
        nextVer,
        "OWNER",
        "Manual snapshot of the running UI build from Settings",
        false
      );
      const updated = await api.getUiDeployments();
      setDeployments(updated);
      setStatusBanner(`Snapshotted ${dep.version} (sha256 ${dep.sha256_hash.slice(0, 12)}…). Preview it or keep it as a rollback point.`);
    } catch (e) {
      setStatusBanner(`Snapshot blocked: ${String(e)}`, true);
    } finally {
      setDeploying(false);
    }
  };

  const handleRollback = async () => {
    try {
      const target = await api.rollbackUiVersion("Manual user rollback from Settings");
      const updated = await api.getUiDeployments();
      setDeployments(updated);
      setStatusBanner(`Rolled back to ${target.version}. Reloading the UI...`);
    } catch (e) {
      setStatusBanner(`Rollback notice: ${String(e)}`, true);
    }
  };

  const handleBackupNow = async () => {
    try {
      const path = await api.createDbBackup();
      setStatusBanner(`Online SQLite backup created: ${path}`);
    } catch (e) {
      setStatusBanner(`Backup error: ${String(e)}`, true);
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
            <M3Icon name="shield" filled size={20} className="text-hive-cyan" />
            <h2 className="text-sm font-semibold text-hive-brightText uppercase tracking-wider">
              Agency Settings, Governance & Constitution
            </h2>
          </div>
          <button
            onClick={onClose}
            className="text-hive-dimText hover:text-hive-brightText text-sm px-2 py-1 rounded hover:bg-hive-surface"
          >
            <M3Icon name="close" size={18} />
          </button>
        </div>

        {/* Sub-navigation Tabs */}
        <div className="flex border-b border-hive-border px-4 bg-hive-base/50 overflow-x-auto">
          {[
            { id: "constitution", label: "Constitution", icon: "policy" },
            { id: "deployments", label: "Snapshots", icon: "account_tree" },
            { id: "society", label: "Society", icon: "groups" },
            { id: "accessibility", label: "Accessibility", icon: "accessibility_new" },
            { id: "bridges", label: "Bridges", icon: "hub" },
            { id: "lifecycle", label: "Lifecycle", icon: "settings_power" },
          ].map((t) => (
            <button
              key={t.id}
              onClick={() => {
                setTab(t.id as any);
                setStatusBanner(null);
              }}
              className={`py-2.5 px-3 text-xs font-medium border-b-2 whitespace-nowrap transition-colors flex items-center gap-1.5 ${
                tab === t.id
                  ? "border-hive-cyan text-hive-cyan"
                  : "border-transparent text-hive-dimText hover:text-hive-brightText"
              }`}
            >
              <M3Icon name={t.icon} size={15} filled={tab === t.id} />
              <span>{t.label}</span>
            </button>
          ))}
        </div>

        {statusBanner && (
          <div
            role={statusBanner.error ? "alert" : "status"}
            className={`px-4 py-2 bg-hive-surface border-b text-xs flex items-center justify-between gap-3 ${
              statusBanner.error ? "border-red-500/40 text-red-400" : "border-hive-cyan/40 text-hive-cyan"
            }`}
          >
            <span className="flex items-center gap-1.5">
              <M3Icon name={statusBanner.error ? "error" : "check_circle"} size={14} />
              {statusBanner.text}
            </span>
            <button onClick={() => setStatusBanner(null)} className="text-hive-dimText">
              <M3Icon name="close" size={14} />
            </button>
          </div>
        )}

        {/* Tab Content */}
        <div className="p-5 flex-1 overflow-y-auto space-y-4">
          {tab === "constitution" && (
            <div className="space-y-3">
              <div className="p-3 bg-hive-surface rounded-lg border border-hive-border">
                <span className="text-xs font-semibold text-hive-lime flex items-center gap-1.5">
                  <M3Icon name="verified" filled size={16} /> Immutable Constitution v1.0.0
                </span>
                <p className="text-xs text-hive-dimText mt-1">
                  These rules are compiled into the app. Every UI snapshot's code is scanned against them before it can be activated, and `npm test` checks them too.
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
                    <M3Icon name="account_tree" size={16} className="text-hive-lime" /> Blue/Green UI Version Manager
                  </div>
                  <p className="text-hive-dimText mt-0.5">
                    Snapshots copy the running UI bundle to disk, scan its code against the Constitution, and SHA-256 hash every file. The active snapshot is what loads; rollback re-verifies the hash before switching.
                  </p>
                </div>
                <div className="flex items-center space-x-2">
                  <button
                    onClick={handleDeployNewVersion}
                    disabled={deploying}
                    className="px-2.5 py-1.5 rounded bg-hive-lime text-hive-base font-semibold hover:bg-hive-lime/90 disabled:opacity-50"
                  >
                    {deploying ? "Snapshotting..." : "+ Verify & Snapshot"}
                  </button>
                  <button
                    onClick={handleRollback}
                    className="px-2.5 py-1.5 rounded bg-hive-base border border-hive-border text-hive-dimText hover:text-hive-brightText flex items-center gap-1"
                  >
                    <M3Icon name="history" size={15} /> Rollback
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
                        <span className="font-mono font-bold text-hive-cyan">{d.version}</span>
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-hive-base border border-hive-border text-hive-dimText font-mono">
                          @{d.author_agent}
                        </span>
                        <span
                          className={`text-[10px] px-1.5 py-0.5 rounded font-mono ${
                            d.is_active
                              ? "bg-hive-lime/15 text-hive-lime border border-hive-lime/30"
                              : "bg-hive-base text-hive-dimText"
                          }`}
                        >
                          {d.is_active ? "ACTIVE" : "STANDBY"}
                        </span>
                      </div>
                      <div className="text-hive-brightText mt-1">{d.verification_log || d.directory_path}</div>
                      <div className="text-[10px] text-hive-dimText font-mono mt-0.5">
                        SHA-256: {d.sha256_hash.slice(0, 24)}...
                      </div>
                    </div>
                    {d.directory_path.endsWith("bundle") && (
                      <button
                        onClick={() =>
                          api.openPreviewWindow(d.version).catch((e) => setStatusBanner(`Preview failed: ${String(e)}`, true))
                        }
                        className="shrink-0 ml-3 px-2.5 py-1.5 rounded bg-hive-base border border-hive-cyan/40 text-hive-cyan hover:bg-hive-cyan/10 flex items-center gap-1"
                      >
                        <M3Icon name="visibility" size={15} /> Preview
                      </button>
                    )}
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
                    No active temporary specialists. Use `/spawn_temp name` in chat.
                  </div>
                ) : (
                  <div className="space-y-1.5">
                    {specialists.map((s) => (
                      <div
                        key={s.codename}
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
                        className="text-hive-dimText hover:text-hive-lime p-1"
                        title="Delete memory record"
                      >
                        <M3Icon name="delete" size={15} />
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
                  <M3Icon name="text_fields" size={16} className="text-hive-cyan" /> Typography Preset
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

              <div className="p-3 bg-hive-surface rounded-lg border border-hive-border flex items-center justify-between gap-4">
                <div>
                  <div className="font-semibold text-hive-brightText">Silent pop-ups for agent DMs</div>
                  <p className="text-hive-dimText mt-0.5">
                    When H.I.V.E. is hidden or in the background, show a silent desktop notification for direct messages from agents. Never for #agency-hq chatter, at most one per agent every 20 seconds, and no sound. Off by default.
                  </p>
                </div>
                <button
                  role="switch"
                  aria-checked={notifyDms}
                  onClick={handleToggleNotify}
                  className={`shrink-0 w-11 h-6 rounded-full border transition-colors relative ${
                    notifyDms ? "bg-hive-lime/30 border-hive-lime/60" : "bg-hive-base border-hive-border"
                  }`}
                  title={notifyDms ? "On: click to turn off" : "Off: click to turn on"}
                >
                  <span
                    className={`absolute top-0.5 w-[18px] h-[18px] rounded-full transition-all ${
                      notifyDms ? "left-[22px] bg-hive-lime" : "left-0.5 bg-hive-dimText"
                    }`}
                  />
                </button>
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
              {(showKeyForm || (liveBridges && liveBridges.llm_provider.startsWith("Unconfigured"))) && (
                <div className="p-3 rounded bg-hive-surface border border-hive-lime/40 space-y-2">
                  <div className="font-semibold text-hive-brightText flex items-center gap-1.5">
                    <M3Icon name="key" size={15} className="text-hive-lime" /> NVIDIA NIM API key
                  </div>
                  <p className="text-hive-dimText">
                    Agents need this to think. Get one at build.nvidia.com. It's checked with NVIDIA before it's saved.
                  </p>
                  <div className="flex gap-2">
                    <input
                      type="password"
                      value={apiKeyInput}
                      onChange={(e) => setApiKeyInput(e.target.value)}
                      onKeyDown={(e) => e.key === "Enter" && handleSaveKey()}
                      placeholder="nvapi-..."
                      autoComplete="off"
                      spellCheck={false}
                      className="flex-1 bg-hive-base border border-hive-border rounded px-2.5 py-1.5 font-mono text-hive-brightText outline-none focus:border-hive-lime/60"
                    />
                    <button
                      onClick={handleSaveKey}
                      disabled={savingKey || !apiKeyInput.trim()}
                      className="px-3 py-1.5 rounded bg-hive-lime text-hive-base font-semibold disabled:opacity-50"
                    >
                      {savingKey ? "Checking…" : "Verify & save"}
                    </button>
                  </div>
                </div>
              )}
              <div className="flex items-center justify-between">
                <span className="text-hive-dimText">
                  {liveBridges ? "Checked just now on this machine." : "Checking bridges..."}
                </span>
                <button
                  onClick={refreshBridges}
                  disabled={bridgesLoading}
                  className="px-2.5 py-1 rounded bg-hive-base border border-hive-border text-hive-dimText hover:text-hive-brightText flex items-center gap-1 disabled:opacity-50"
                >
                  <M3Icon name="refresh" size={15} /> {bridgesLoading ? "Checking..." : "Re-check"}
                </button>
              </div>
              {[
                {
                  title: "Language model provider",
                  detail: liveBridges?.llm_provider || "Unknown",
                  action: liveBridges && !liveBridges.llm_provider.startsWith("Unconfigured") && !showKeyForm
                    ? { label: "Replace key", run: () => setShowKeyForm(true) }
                    : undefined,
                  ok: !!liveBridges && !liveBridges.llm_provider.startsWith("Unconfigured"),
                  okLabel: "CONFIGURED",
                  badLabel: "NOT CONFIGURED",
                },
                {
                  title: "Google Account Bridge (gog)",
                  detail: liveBridges?.google_connected
                    ? `Account: ${liveBridges.google_account || "authenticated"}`
                    : "gog CLI not found or not signed in (`gog auth login`)",
                  ok: !!liveBridges?.google_connected,
                  okLabel: "CONNECTED",
                  badLabel: "NOT CONNECTED",
                },
                {
                  title: "Obsidian Vault Bridge",
                  detail: liveBridges?.obsidian_vault_path || "Vault path unknown",
                  ok: !!liveBridges?.obsidian_available,
                  okLabel: "AVAILABLE",
                  badLabel: "UNAVAILABLE",
                },
                {
                  title: "OpenCode Chrome Relay",
                  detail: "`browser-relay` on PATH, used by `/browser`",
                  ok: !!liveBridges?.browser_relay_available,
                  okLabel: "AVAILABLE",
                  badLabel: "NOT INSTALLED",
                },
                {
                  title: "Speech Synthesizer (Windows SAPI)",
                  detail: isMuted ? "Muted. Plays only when you click a message's speaker button." : "Unmuted. Plays only on click.",
                  ok: !!liveBridges?.voice_engine_available,
                  okLabel: "READY",
                  badLabel: "UNAVAILABLE",
                },
              ].map((b) => (
                <div
                  key={b.title}
                  className="p-3 rounded bg-hive-surface border border-hive-border flex items-center justify-between gap-3"
                >
                  <div className="min-w-0">
                    <div className="font-semibold text-hive-brightText">{b.title}</div>
                    <div className="text-hive-dimText font-mono text-[11px] mt-0.5 truncate">{b.detail}</div>
                  </div>
                  {"action" in b && b.action && (
                    <button
                      onClick={b.action.run}
                      className="shrink-0 px-2 py-0.5 rounded border border-hive-border text-hive-dimText hover:text-hive-brightText text-[10px]"
                    >
                      {b.action.label}
                    </button>
                  )}
                  <span
                    className={`shrink-0 px-2 py-0.5 rounded font-mono text-[10px] border ${
                      !liveBridges
                        ? "bg-hive-base text-hive-dimText border-hive-border"
                        : b.ok
                        ? "bg-hive-lime/10 text-hive-lime border-hive-lime/30"
                        : "bg-red-500/10 text-red-400 border-red-500/30"
                    }`}
                  >
                    {!liveBridges ? "CHECKING" : b.ok ? b.okLabel : b.badLabel}
                  </span>
                </div>
              ))}
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
                  className="p-3 rounded-lg border bg-hive-surface border-hive-border text-hive-brightText hover:border-hive-lime/50 flex items-center justify-center gap-1.5 font-medium"
                >
                  <M3Icon name="visibility_off" size={16} className="text-hive-lime" /> Hide Window (Tray)
                </button>

                {agencyState === "running" ? (
                  <button
                    onClick={() => onSetAgencyState("paused")}
                    className="p-3 rounded-lg border bg-hive-surface border-hive-border text-hive-brightText hover:border-hive-lime/50 flex items-center justify-center gap-1.5 font-medium"
                  >
                    <M3Icon name="pause" filled size={16} className="text-hive-lime" /> Pause Agency
                  </button>
                ) : (
                  <button
                    onClick={() => onSetAgencyState("running")}
                    className="p-3 rounded-lg border bg-hive-lime text-hive-base border-hive-lime flex items-center justify-center gap-1.5 font-semibold"
                  >
                    <M3Icon name="play_arrow" filled size={16} /> Resume Agency
                  </button>
                )}

                <button
                  onClick={() => api.stopAgencyAndQuit()}
                  className="p-3 rounded-lg border bg-hive-base border-hive-border text-hive-dimText hover:text-hive-brightText hover:border-hive-lime/40 flex items-center justify-center gap-1.5 font-medium"
                >
                  <M3Icon name="power_settings_new" size={16} /> Stop Agency & Quit
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
                  <M3Icon name="database" size={15} /> Backup Now
                </button>
              </div>

              <div className="space-y-1.5 pt-2 border-t border-hive-border">
                <div className="font-semibold text-hive-brightText uppercase tracking-wider text-[11px]">
                  Recent Security & Governance Audit Log
                </div>
                <div className="max-h-36 overflow-y-auto space-y-1 font-mono text-[10px]">
                  {auditLogs.map((a, idx) => (
                    <div
                      key={a.id ?? idx}
                      className="px-2 py-1 rounded bg-hive-base border border-hive-border flex items-center justify-between"
                    >
                      <span className="text-hive-cyan">
                        [{a.actor}] {a.event_type}
                      </span>
                      <span className="text-hive-dimText truncate max-w-[260px]">
                        {a.details} ({a.status})
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
