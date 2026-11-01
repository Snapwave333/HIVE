import React from "react";
import { M3Icon } from "./M3Icon";

interface NavRailProps {
  currentTab: "chats" | "settings";
  onSelectTab: (tab: "chats" | "settings") => void;
  agencyState: string;
  isMuted: boolean;
  onToggleMute: () => void;
  unreadTotal: number;
}

export const NavRail: React.FC<NavRailProps> = ({
  currentTab,
  onSelectTab,
  agencyState,
  isMuted,
  onToggleMute,
  unreadTotal,
}) => {
  return (
    <aside className="w-14 bg-hive-base border-r border-hive-border flex flex-col items-center justify-between py-4 select-none z-20 flex-shrink-0">
      {/* Top Branding & Main Navigation */}
      <div className="flex flex-col items-center space-y-3.5 w-full">
        {/* Borg Hexagon Logo */}
        <div
          className="w-10 h-10 rounded-xl bg-hive-surface border border-hive-lime/30 flex items-center justify-center text-hive-lime cursor-pointer transition-all hover:border-hive-lime/60"
          title="H.I.V.E. Autonomous Agency"
        >
          <M3Icon name="hexagon" filled size={20} />
        </div>

        <div className="w-6 h-px bg-hive-border my-1" />

        {/* Chats Tab */}
        <button
          onClick={() => onSelectTab("chats")}
          className={`w-10 h-10 rounded-xl flex items-center justify-center transition-all relative ${
            currentTab === "chats"
              ? "bg-hive-surface text-hive-lime border border-hive-lime/40"
              : "text-hive-dimText hover:text-hive-brightText hover:bg-hive-surface"
          }`}
          title="Chats & Direct Lines"
          aria-label="Chats"
        >
          {currentTab === "chats" && (
            <span className="absolute -left-2 w-1 h-5 rounded-r bg-hive-lime" />
          )}
          <M3Icon name="forum" filled={currentTab === "chats"} size={20} />
          {unreadTotal > 0 && (
            <span
              className="absolute top-1 right-1 min-w-[16px] h-4 px-1 rounded-full bg-hive-lime text-hive-base text-[9px] font-bold flex items-center justify-center"
              title={`${unreadTotal} unread`}
            >
              {unreadTotal > 99 ? "99+" : unreadTotal}
            </span>
          )}
        </button>

        {/* Settings & Constitution Tab */}
        <button
          onClick={() => onSelectTab("settings")}
          className={`w-10 h-10 rounded-xl flex items-center justify-center transition-all relative ${
            currentTab === "settings"
              ? "bg-hive-surface text-hive-lime border border-hive-lime/40"
              : "text-hive-dimText hover:text-hive-brightText hover:bg-hive-surface"
          }`}
          title="Agency Settings & Constitution"
          aria-label="Settings"
        >
          {currentTab === "settings" && (
            <span className="absolute -left-2 w-1 h-5 rounded-r bg-hive-lime" />
          )}
          <M3Icon name="tune" filled={currentTab === "settings"} size={20} />
        </button>
      </div>

      {/* Bottom Indicators & Audio Sovereign Switch */}
      <div className="flex flex-col items-center space-y-4">
        {/* Audio Mute Sovereign Switch (Strict Invariant) */}
        <button
          onClick={onToggleMute}
          className={`w-10 h-10 rounded-xl flex items-center justify-center transition-all border ${
            isMuted
              ? "bg-hive-surface text-hive-dimText border-hive-border hover:text-hive-brightText"
              : "bg-hive-surface text-hive-lime border-hive-lime/50"
          }`}
          title={isMuted ? "Audio Strictly Muted (Default)" : "Audio Speech Enabled"}
          aria-label="Audio Mute"
        >
          <M3Icon name={isMuted ? "volume_off" : "volume_up"} filled={!isMuted} size={19} />
        </button>

        {/* 24/7 Agency Operational State Dot */}
        <div
          className="w-8 h-8 rounded-lg bg-hive-surface border border-hive-border flex items-center justify-center cursor-pointer"
          title={`Agency State: ${agencyState.toUpperCase()}`}
        >
          <div
            className={`w-2.5 h-2.5 rounded-full transition-colors ${
              agencyState === "running"
                ? "bg-hive-lime animate-pulse"
                : "bg-hive-dimText"
            }`}
          />
        </div>
      </div>
    </aside>
  );
};
