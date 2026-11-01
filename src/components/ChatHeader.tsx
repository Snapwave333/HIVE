import React from "react";
import { Channel, Agent } from "../types";
import { M3Icon } from "./M3Icon";

interface ChatHeaderProps {
  channel: Channel;
  agent?: Agent;
  isMuted: boolean;
  onToggleMute: () => void;
  isFocusMode: boolean;
  onToggleFocusMode: () => void;
  onOpenSearch: () => void;
}

export const ChatHeader: React.FC<ChatHeaderProps> = ({
  channel,
  agent,
  isMuted,
  onToggleMute,
  isFocusMode,
  onToggleFocusMode,
  onOpenSearch,
}) => {
  return (
    <header className="h-16 bg-hive-panel border-b border-hive-border px-6 flex items-center justify-between select-none flex-shrink-0">
      {/* Left: Channel / Agent Identity */}
      <div className="flex items-center space-x-3.5 min-w-0">
        <div className="w-10 h-10 rounded-xl bg-hive-surface border border-hive-lime/40 text-hive-lime flex items-center justify-center font-bold text-xs flex-shrink-0">
          {channel.id === 0 ? (
            <M3Icon name="hub" filled size={20} />
          ) : (
            channel.name.slice(0, 2).toUpperCase()
          )}
        </div>

        <div className="min-w-0 space-y-0.5">
          <div className="flex items-center space-x-2.5">
            <h1 className="text-[15px] font-semibold text-hive-brightText truncate">
              {channel.id === 0 ? "#agency-hq" : channel.name}
            </h1>
            {channel.id === 0 ? (
              <span className="text-[10px] bg-hive-base text-hive-lime border border-hive-lime/30 px-2 py-0.5 rounded-md font-mono">
                12 Agents Active
              </span>
            ) : (
              <span className="text-[10px] bg-hive-surface text-hive-lime border border-hive-border px-2 py-0.5 rounded-md font-mono">
                {agent?.role}
              </span>
            )}
          </div>
          <p className="text-xs text-hive-dimText truncate">
            {channel.id === 0
              ? "Primary Agency Town Square — Autonomous 24/7 Multi-Agent Collaboration"
              : `Direct line to ${agent?.display_name} • ${agent?.model_tier}`}
          </p>
        </div>
      </div>

      {/* Right: Quick Action Controls */}
      <div className="flex items-center space-x-2">
        {/* Search within conversation */}
        <button
          onClick={onOpenSearch}
          className="w-9 h-9 rounded-xl border border-hive-border bg-hive-surface hover:border-hive-lime/40 text-hive-dimText hover:text-hive-brightText transition-all flex items-center justify-center"
          title="Search messages (Ctrl+K)"
          aria-label="Search"
        >
          <M3Icon name="search" size={18} />
        </button>

        {/* Audio Mute Toggle Button */}
        <button
          onClick={onToggleMute}
          className={`w-9 h-9 rounded-xl border transition-all flex items-center justify-center ${
            isMuted
              ? "border-hive-border bg-hive-surface text-hive-dimText hover:text-hive-brightText"
              : "border-hive-lime/50 bg-hive-lime/15 text-hive-lime"
          }`}
          title={isMuted ? "Audio Muted (Strict Invariant)" : "Speech Enabled (Click to Mute)"}
          aria-label="Toggle Audio Mute"
        >
          <M3Icon name={isMuted ? "volume_off" : "volume_up"} filled={!isMuted} size={18} />
        </button>

        {/* Focus Mode Toggle (Ctrl+F11) */}
        <button
          onClick={onToggleFocusMode}
          className={`w-9 h-9 rounded-xl border transition-all flex items-center justify-center ${
            isFocusMode
              ? "border-hive-lime/50 bg-hive-lime/15 text-hive-lime"
              : "border-hive-border bg-hive-surface text-hive-dimText hover:text-hive-brightText"
          }`}
          title="Focus Mode (Ctrl+F11)"
          aria-label="Focus Mode"
        >
          <M3Icon name={isFocusMode ? "close_fullscreen" : "open_in_full"} size={18} />
        </button>
      </div>
    </header>
  );
};
