import React from "react";
import { M3Icon } from "./M3Icon";

interface NavRailProps {
  currentTab: "chats" | "settings";
  onSelectTab: (tab: "chats" | "settings") => void;
  agencyState: string;
  isMuted: boolean;
  onToggleMute: () => void;
}

export const NavRail: React.FC<NavRailProps> = ({
  currentTab,
  onSelectTab,
  agencyState,
  isMuted,
  onToggleMute,
}) => {
  return (
    <aside className="w-12 bg-hive-panel border-r border-hive-border flex flex-col items-center justify-between py-3 select-none z-20">
      {/* Top Branding & Main Navigation */}
      <div className="flex flex-col items-center space-y-4">
        {/* Borg Hexagon Logo */}
        <div
          className="w-8 h-8 rounded bg-hive-surface border border-hive-cyan/30 flex items-center justify-center text-hive-cyan shadow-inner cursor-pointer"
          title="H.I.V.E. Autonomous Agency"
        >
          <M3Icon name="hexagon" filled size={18} />
        </div>

        {/* Chats Tab */}
        <button
          onClick={() => onSelectTab("chats")}
          className={`w-9 h-9 rounded-lg flex items-center justify-center transition-colors relative ${
            currentTab === "chats"
              ? "bg-hive-surface text-hive-cyan border border-hive-cyan/40"
              : "text-hive-dimText hover:text-hive-brightText hover:bg-hive-surface/60"
          }`}
          title="Chats & Direct Lines"
          aria-label="Chats"
        >
          <M3Icon name="forum" filled={currentTab === "chats"} size={19} />
          <span className="absolute bottom-1 right-1 w-1.5 h-1.5 rounded-full bg-hive-lime" />
        </button>

        {/* Settings & Constitution Tab */}
        <button
          onClick={() => onSelectTab("settings")}
          className={`w-9 h-9 rounded-lg flex items-center justify-center transition-colors ${
            currentTab === "settings"
              ? "bg-hive-surface text-hive-cyan border border-hive-cyan/40"
              : "text-hive-dimText hover:text-hive-brightText hover:bg-hive-surface/60"
          }`}
          title="Agency Settings & Constitution"
          aria-label="Settings"
        >
          <M3Icon name="tune" filled={currentTab === "settings"} size={19} />
        </button>
      </div>

      {/* Bottom Indicators & Audio Sovereign Switch */}
      <div className="flex flex-col items-center space-y-3">
        {/* Audio Mute Sovereign Switch (Strict Invariant) */}
        <button
          onClick={onToggleMute}
          className={`w-8 h-8 rounded flex items-center justify-center transition-colors border ${
            isMuted
              ? "bg-hive-surface text-hive-dimText border-hive-border hover:text-hive-brightText"
              : "bg-hive-surface text-hive-lime border-hive-lime/50"
          }`}
          title={isMuted ? "Audio Strictly Muted (Default)" : "Audio Speech Enabled"}
          aria-label="Audio Mute"
        >
          <M3Icon name={isMuted ? "volume_off" : "volume_up"} filled={!isMuted} size={17} />
        </button>

        {/* 24/7 Agency Operational State Dot */}
        <div
          className="relative cursor-pointer group"
          title={`Agency State: ${agencyState.toUpperCase()}`}
        >
          <div
            className={`w-2.5 h-2.5 rounded-full transition-colors ${
              agencyState === "running"
                ? "bg-hive-lime animate-pulse"
                : agencyState === "paused"
                ? "bg-yellow-500"
                : "bg-red-500"
            }`}
          />
        </div>
      </div>
    </aside>
  );
};
