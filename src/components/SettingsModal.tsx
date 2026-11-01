import React, { useState } from "react";
import { Constitution, BridgeStatus } from "../types";
import { Shield, Type, Play, Pause, CheckCircle } from "lucide-react";

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
  const [tab, setTab] = useState<"constitution" | "accessibility" | "bridges" | "lifecycle">("constitution");

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-4 select-none">
      <div className="bg-hive-panel border border-hive-border rounded-xl w-full max-w-2xl max-h-[85vh] flex flex-col shadow-2xl overflow-hidden font-segoe">
        {/* Modal Header */}
        <div className="p-4 border-b border-hive-border flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <Shield className="w-5 h-5 text-hive-cyan" />
            <h2 className="text-sm font-semibold text-hive-brightText uppercase tracking-wider">
              Agency Settings & Constitution
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
        <div className="flex border-b border-hive-border px-4 bg-hive-base/50">
          <button
            onClick={() => setTab("constitution")}
            className={`py-2.5 px-3 text-xs font-medium border-b-2 transition-colors ${
              tab === "constitution"
                ? "border-hive-cyan text-hive-cyan"
                : "border-transparent text-hive-dimText hover:text-hive-brightText"
            }`}
          >
            Constitution
          </button>
          <button
            onClick={() => setTab("accessibility")}
            className={`py-2.5 px-3 text-xs font-medium border-b-2 transition-colors ${
              tab === "accessibility"
                ? "border-hive-cyan text-hive-cyan"
                : "border-transparent text-hive-dimText hover:text-hive-brightText"
            }`}
          >
            Accessibility
          </button>
          <button
            onClick={() => setTab("bridges")}
            className={`py-2.5 px-3 text-xs font-medium border-b-2 transition-colors ${
              tab === "bridges"
                ? "border-hive-cyan text-hive-cyan"
                : "border-transparent text-hive-dimText hover:text-hive-brightText"
            }`}
          >
            Bridges
          </button>
          <button
            onClick={() => setTab("lifecycle")}
            className={`py-2.5 px-3 text-xs font-medium border-b-2 transition-colors ${
              tab === "lifecycle"
                ? "border-hive-cyan text-hive-cyan"
                : "border-transparent text-hive-dimText hover:text-hive-brightText"
            }`}
          >
            Controls
          </button>
        </div>

        {/* Tab Content */}
        <div className="p-5 flex-1 overflow-y-auto space-y-4">
          {tab === "constitution" && (
            <div className="space-y-3">
              <div className="p-3 bg-hive-surface rounded-lg border border-hive-border">
                <span className="text-xs font-semibold text-hive-lime flex items-center gap-1.5">
                  <CheckCircle className="w-3.5 h-3.5" /> Immutable Constitution v1.0.0
                </span>
                <p className="text-xs text-hive-dimText mt-1">
                  These invariants are machine-verified on every autonomous commit. Agents cannot alter or bypass them.
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

          {tab === "accessibility" && (
            <div className="space-y-4 text-xs">
              {/* Font Selector */}
              <div className="space-y-2">
                <label className="font-semibold text-hive-brightText flex items-center gap-1.5">
                  <Type className="w-4 h-4 text-hive-cyan" /> Typography Preset
                </label>
                <div className="grid grid-cols-3 gap-2">
                  {[
                    { id: "segoe", label: "Segoe UI Variable", desc: "Default Windows 11" },
                    { id: "atkinson", label: "Atkinson Hyperlegible", desc: "Braille Institute" },
                    { id: "dyslexic", label: "OpenDyslexic", desc: "Anti-Gravity Weight" },
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

              {/* Font Scaling */}
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

              {/* Sensory Environment */}
              <div className="p-3 bg-hive-surface rounded-lg border border-hive-border space-y-1">
                <span className="font-semibold text-hive-brightText">
                  Sensory Sovereignty
                </span>
                <p className="text-hive-dimText leading-relaxed">
                  Audio speech generation is 100% muted by default. Sounds play strictly when you click the TTS button on a message.
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
                    Direct markdown note curation inside local vault
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
                    Live browser automation via localhost:3000
                  </div>
                </div>
                <span className="px-2 py-0.5 bg-hive-cyan/10 text-hive-cyan border border-hive-cyan/30 rounded font-mono text-[10px]">
                  ACTIVE
                </span>
              </div>

              <div className="p-3 rounded bg-hive-surface border border-hive-border flex items-center justify-between">
                <div>
                  <div className="font-semibold text-hive-brightText">Voice Synthesis (Kokoro / Voicebox)</div>
                  <div className="text-hive-dimText font-mono text-[11px] mt-0.5">
                    Local neural speech synthesis cache
                  </div>
                </div>
                <span className="px-2 py-0.5 bg-hive-lime/10 text-hive-lime border border-hive-lime/30 rounded font-mono text-[10px]">
                  READY (MUTED)
                </span>
              </div>
            </div>
          )}

          {tab === "lifecycle" && (
            <div className="space-y-3 text-xs">
              <div className="p-3 rounded bg-hive-surface border border-hive-border space-y-2">
                <div className="font-semibold text-hive-brightText">
                  Current Agency State: <span className="text-hive-lime uppercase">{agencyState}</span>
                </div>
                <p className="text-hive-dimText leading-relaxed">
                  The background agency daemon runs 24/7 independently of the desktop window. You can hide the window, pause the agency, or completely stop it.
                </p>
              </div>

              <div className="grid grid-cols-2 gap-2 pt-2">
                <button
                  onClick={() => onSetAgencyState("running")}
                  className={`p-3 rounded border flex items-center justify-center gap-2 font-medium transition-colors ${
                    agencyState === "running"
                      ? "bg-hive-lime text-hive-base border-hive-lime"
                      : "bg-hive-surface border-hive-border text-hive-brightText hover:border-hive-lime"
                  }`}
                >
                  <Play className="w-4 h-4" /> Resume Agency
                </button>

                <button
                  onClick={() => onSetAgencyState("paused")}
                  className={`p-3 rounded border flex items-center justify-center gap-2 font-medium transition-colors ${
                    agencyState === "paused"
                      ? "bg-yellow-500 text-hive-base border-yellow-500"
                      : "bg-hive-surface border-hive-border text-hive-brightText hover:border-yellow-500"
                  }`}
                >
                  <Pause className="w-4 h-4" /> Pause Agency
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
