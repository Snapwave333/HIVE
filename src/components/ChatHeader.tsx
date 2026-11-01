import React from "react";
import { Channel, Agent } from "../types";
import { VolumeX, Volume2, Maximize2, Minimize2, Search } from "lucide-react";

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
  let avatarColor = "#42CFC4";
  if (agent?.avatar_config) {
    try {
      const cfg = JSON.parse(agent.avatar_config);
      avatarColor = cfg.primary_color || avatarColor;
    } catch {
      // ignore
    }
  }

  return (
    <header className="h-14 bg-hive-panel border-b border-hive-border px-4 flex items-center justify-between select-none">
      {/* Left: Channel / Agent Identity */}
      <div className="flex items-center space-x-3 min-w-0">
        <div
          className="w-8 h-8 rounded-full flex items-center justify-center font-bold text-xs border flex-shrink-0"
          style={{
            backgroundColor: `${avatarColor}15`,
            borderColor: `${avatarColor}40`,
            color: avatarColor,
          }}
        >
          {channel.id === 0 ? "⬡" : channel.name.slice(0, 2).toUpperCase()}
        </div>

        <div className="min-w-0">
          <div className="flex items-center space-x-2">
            <h1 className="text-sm font-semibold text-hive-brightText truncate">
              {channel.id === 0 ? "#agency-hq" : channel.name}
            </h1>
            {channel.id === 0 ? (
              <span className="text-[10px] bg-hive-base text-hive-lime border border-hive-lime/30 px-1.5 py-0.2 rounded font-mono">
                12 Agents
              </span>
            ) : (
              <span className="text-[10px] bg-hive-surface text-hive-cyan border border-hive-border px-1.5 py-0.2 rounded font-mono">
                {agent?.role}
              </span>
            )}
          </div>
          <p className="text-xs text-hive-dimText truncate">
            {channel.id === 0
              ? "Primary Agency Town Square — Transparent Multi-Agent Collaboration"
              : `Direct line to ${agent?.display_name} • ${agent?.model_tier}`}
          </p>
        </div>
      </div>

      {/* Right: Quick Action Controls */}
      <div className="flex items-center space-x-1.5">
        {/* Search within conversation */}
        <button
          onClick={onOpenSearch}
          className="p-1.5 rounded hover:bg-hive-surface text-hive-dimText hover:text-hive-brightText transition-colors"
          title="Search messages (Ctrl+K)"
          aria-label="Search"
        >
          <Search className="w-4 h-4" />
        </button>

        {/* Audio Mute Toggle Button */}
        <button
          onClick={onToggleMute}
          className={`p-1.5 rounded transition-colors ${
            isMuted
              ? "text-hive-dimText hover:text-hive-brightText hover:bg-hive-surface"
              : "text-hive-lime bg-hive-lime/10 hover:bg-hive-lime/20"
          }`}
          title={isMuted ? "Audio Muted (Strict Invariant)" : "Speech Enabled (Click to Mute)"}
          aria-label="Toggle Audio Mute"
        >
          {isMuted ? <VolumeX className="w-4 h-4" /> : <Volume2 className="w-4 h-4" />}
        </button>

        {/* Focus Mode Toggle (Ctrl+F11) */}
        <button
          onClick={onToggleFocusMode}
          className={`p-1.5 rounded transition-colors ${
            isFocusMode
              ? "text-hive-cyan bg-hive-cyan/10 hover:bg-hive-cyan/20"
              : "text-hive-dimText hover:text-hive-brightText hover:bg-hive-surface"
          }`}
          title="Focus Mode (Ctrl+F11)"
          aria-label="Focus Mode"
        >
          {isFocusMode ? <Minimize2 className="w-4 h-4" /> : <Maximize2 className="w-4 h-4" />}
        </button>
      </div>
    </header>
  );
};
