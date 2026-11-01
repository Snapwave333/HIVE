import React, { useState } from "react";
import { Channel, Agent, Message, TaskChunk } from "../types";
import { TaskCard } from "./TaskCard";
import { M3Icon } from "./M3Icon";

interface ConversationListProps {
  channels: Channel[];
  agents: Agent[];
  tasks: TaskChunk[];
  activeChannelId: number;
  onSelectChannel: (channelId: number) => void;
  onOpenSearch: () => void;
  onTaskStepToggle: (taskId: string, stepIndex: number, completed: boolean) => void;
  onCreateTask: (title: string, assignee: string, steps: string[]) => void;
  latestMessages: Record<number, Message>;
}

export const ConversationList: React.FC<ConversationListProps> = ({
  channels,
  agents,
  tasks,
  activeChannelId,
  onSelectChannel,
  onOpenSearch,
  onTaskStepToggle,
  onCreateTask,
  latestMessages,
}) => {
  const [filter, setFilter] = useState<"all" | "tasks">("all");
  const [query, setQuery] = useState("");
  const [showNewTaskForm, setShowNewTaskForm] = useState(false);
  const [newTaskTitle, setNewTaskTitle] = useState("");
  const [newTaskAssignee, setNewTaskAssignee] = useState("FORGE");

  const hqChannel = channels.find((c) => c.id === 0);
  const dmChannels = channels.filter((c) => c.id !== 0);

  const getAgentForChannel = (channel: Channel): Agent | undefined => {
    return agents.find((a) => a.codename === channel.agent_codename);
  };

  const filteredDms = dmChannels.filter((c) => {
    if (!query) return true;
    const matchName = c.name.toLowerCase().includes(query.toLowerCase());
    const matchDesc = c.description?.toLowerCase().includes(query.toLowerCase()) ?? false;
    const agent = getAgentForChannel(c);
    const matchRole = agent?.role.toLowerCase().includes(query.toLowerCase()) ?? false;
    return matchName || matchDesc || matchRole;
  });

  const handleAddTask = () => {
    if (!newTaskTitle.trim()) return;
    onCreateTask(newTaskTitle.trim(), newTaskAssignee, [
      `Step 1 (5m): Scope and inspect "${newTaskTitle.trim()}"`,
      `Step 2 (5m): Execute primary action & verify`,
      `Step 3 (5m): Record checkpoint and notify #agency-hq`,
    ]);
    setNewTaskTitle("");
    setShowNewTaskForm(false);
  };

  return (
    <div className="w-[340px] bg-hive-panel border-r border-hive-border flex flex-col h-full select-none flex-shrink-0">
      {/* Header & Instant Search Bar */}
      <div className="px-4 py-4 border-b border-hive-border flex flex-col space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold tracking-wider text-hive-brightText uppercase flex items-center gap-2">
            <M3Icon name="hexagon" filled size={16} className="text-hive-lime" /> H.I.V.E. Channels
          </h2>
          <span className="text-[11px] text-hive-dimText font-mono bg-hive-surface px-2 py-0.5 rounded-md border border-hive-border">
            13 lines
          </span>
        </div>

        {/* Search Input (Ctrl+K trigger) */}
        <div
          onClick={onOpenSearch}
          className="flex items-center px-3 py-2 rounded-xl bg-hive-surface border border-hive-border hover:border-hive-lime/40 text-hive-dimText cursor-pointer transition-all text-xs space-x-2.5"
        >
          <M3Icon name="search" size={16} className="text-hive-lime flex-shrink-0" />
          <input
            type="text"
            placeholder="Search messages & memory..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onClick={(e) => e.stopPropagation()}
            className="bg-transparent border-none outline-none text-hive-brightText placeholder-hive-dimText w-full text-xs"
          />
          <kbd className="text-[10px] bg-hive-base px-1.5 py-0.5 rounded border border-hive-border text-hive-dimText font-mono flex-shrink-0">
            Ctrl+K
          </kbd>
        </div>

        {/* Segmented Filter Pills */}
        <div className="grid grid-cols-2 gap-1.5 p-1 bg-hive-base rounded-xl border border-hive-border">
          <button
            onClick={() => setFilter("all")}
            className={`py-1.5 rounded-lg text-xs font-medium transition-all flex items-center justify-center gap-1.5 ${
              filter === "all"
                ? "bg-hive-surface text-hive-lime border border-hive-lime/30"
                : "text-hive-dimText hover:text-hive-brightText"
            }`}
          >
            <M3Icon name="chat" size={14} filled={filter === "all"} /> All Lines
          </button>
          <button
            onClick={() => setFilter("tasks")}
            className={`py-1.5 rounded-lg text-xs font-medium transition-all flex items-center justify-center gap-1.5 ${
              filter === "tasks"
                ? "bg-hive-surface text-hive-lime border border-hive-lime/30"
                : "text-hive-dimText hover:text-hive-brightText"
            }`}
          >
            <M3Icon name="checklist" size={15} filled={filter === "tasks"} /> 5m Tasks ({tasks.length})
          </button>
        </div>
      </div>

      {/* Stream: Either 5-Minute Tasks View or Conversations Stream */}
      {filter === "tasks" ? (
        <div className="flex-1 overflow-y-auto px-3.5 py-3 space-y-2.5">
          <div className="flex items-center justify-between px-1">
            <span className="text-[10px] uppercase tracking-wider font-semibold text-hive-dimText">
              ADHD 5-Minute Task Queue
            </span>
            <button
              onClick={() => setShowNewTaskForm(!showNewTaskForm)}
              className="text-[11px] text-hive-lime hover:underline flex items-center gap-1 font-medium"
            >
              <M3Icon name="add_task" size={14} /> New 5m Task
            </button>
          </div>

          {showNewTaskForm && (
            <div className="p-3 rounded-xl bg-hive-surface border border-hive-lime/40 space-y-2.5 text-xs">
              <input
                type="text"
                placeholder="Task title (chunked into 5m steps)..."
                value={newTaskTitle}
                onChange={(e) => setNewTaskTitle(e.target.value)}
                className="w-full bg-hive-base border border-hive-border rounded-lg px-2.5 py-1.5 text-hive-brightText outline-none focus:border-hive-lime/50"
              />
              <div className="flex items-center justify-between gap-2">
                <select
                  value={newTaskAssignee}
                  onChange={(e) => setNewTaskAssignee(e.target.value)}
                  className="bg-hive-base border border-hive-border rounded-lg px-2.5 py-1.5 text-hive-lime text-[11px] font-mono outline-none"
                >
                  {agents.map((a) => (
                    <option key={a.codename} value={a.codename}>
                      @{a.codename}
                    </option>
                  ))}
                </select>
                <button
                  onClick={handleAddTask}
                  className="px-3 py-1.5 rounded-lg bg-hive-lime text-hive-base font-semibold text-[11px] hover:opacity-90 transition-opacity"
                >
                  Create Task
                </button>
              </div>
            </div>
          )}

          {tasks.map((t) => (
            <TaskCard key={t.id} task={t} onStepToggle={onTaskStepToggle} />
          ))}
        </div>
      ) : (
        <div className="flex-1 overflow-y-auto px-3 py-3 space-y-4">
          {/* Pinned Section: #agency-hq */}
          {hqChannel && (
            <div className="space-y-1.5">
              <div className="text-[10px] uppercase tracking-wider font-semibold text-hive-dimText px-2 flex items-center gap-1.5">
                <M3Icon name="keep" filled size={12} className="text-hive-lime" /> Pinned Town Square
              </div>
              <div
                onClick={() => onSelectChannel(0)}
                className={`p-3 rounded-xl cursor-pointer transition-all flex items-start space-x-3 relative border ${
                  activeChannelId === 0
                    ? "bg-hive-surface border-hive-lime/40"
                    : "bg-hive-base/40 border-hive-border hover:bg-hive-surface/70"
                }`}
              >
                {activeChannelId === 0 && (
                  <span className="absolute left-0 top-3 bottom-3 w-1 rounded-r bg-hive-lime" />
                )}
                <img
                  src="/avatars/HQ.png"
                  alt="#agency-hq"
                  className="w-10 h-10 rounded-xl object-cover border border-hive-lime/40 flex-shrink-0"
                />

                <div className="flex-1 min-w-0">
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-semibold text-[13px] text-hive-brightText flex items-center gap-1 truncate">
                      #agency-hq
                    </span>
                    <span className="text-[10px] text-hive-dimText font-mono flex-shrink-0">
                      {latestMessages[0] ? formatTime(latestMessages[0].created_at) : "LIVE"}
                    </span>
                  </div>
                  <p className="text-xs text-hive-dimText truncate mt-1">
                    {latestMessages[0]?.content
                      ? stripMarkdown(latestMessages[0].content)
                      : "Primary Agency Town Square"}
                  </p>
                  <div className="flex items-center gap-2 mt-2">
                    <span className="text-[10px] bg-hive-base text-hive-lime border border-hive-lime/30 px-2 py-0.5 rounded-md font-mono">
                      12 Agents Online
                    </span>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* 12 Permanent Agent DMs */}
          <div className="space-y-1.5">
            <div className="text-[10px] uppercase tracking-wider font-semibold text-hive-dimText px-2 flex items-center justify-between">
              <span>Direct Agent Lines</span>
              <span className="text-hive-lime font-mono">12 Permanent</span>
            </div>

            <div className="space-y-1.5">
              {filteredDms.map((channel) => {
                const agent = getAgentForChannel(channel);
                const isActive = activeChannelId === channel.id;
                const lastMsg = latestMessages[channel.id];

                return (
                  <div
                    key={channel.id}
                    onClick={() => onSelectChannel(channel.id)}
                    className={`px-3 py-2.5 rounded-xl cursor-pointer transition-all flex items-start space-x-3 relative border ${
                      isActive
                        ? "bg-hive-surface border-hive-lime/40"
                        : "border-transparent hover:bg-hive-surface/70 hover:border-hive-border"
                    }`}
                  >
                    {isActive && (
                      <span className="absolute left-0 top-2.5 bottom-2.5 w-1 rounded-r bg-hive-lime" />
                    )}
                    <div className="relative flex-shrink-0 mt-0.5">
                      <div
                        className={`w-9 h-9 rounded-xl flex items-center justify-center font-bold text-xs border transition-colors ${
                          isActive
                            ? "bg-hive-lime/15 border-hive-lime/50 text-hive-lime"
                            : "bg-hive-surface border-hive-border text-hive-brightText"
                        }`}
                      >
                        {channel.name.slice(0, 2).toUpperCase()}
                      </div>
                      <span
                        className={`absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full border-2 border-hive-panel ${
                          agent?.status === "thinking"
                            ? "bg-hive-lime animate-pulse"
                            : "bg-hive-lime"
                        }`}
                        title={`Status: ${agent?.status || "available"}`}
                      />
                    </div>

                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-semibold text-xs text-hive-brightText truncate flex items-center gap-1.5">
                          {channel.name}
                          <span className="text-[10px] text-hive-dimText font-mono font-normal">
                            @{channel.agent_codename}
                          </span>
                        </span>
                        <span className="text-[10px] text-hive-dimText font-mono flex-shrink-0">
                          {lastMsg ? formatTime(lastMsg.created_at) : ""}
                        </span>
                      </div>

                      <p className="text-[11px] text-hive-lime/90 truncate mt-0.5 font-medium">
                        {agent?.role || channel.description}
                      </p>

                      <p className="text-xs text-hive-dimText truncate mt-0.5">
                        {lastMsg ? stripMarkdown(lastMsg.content) : "Direct specialist line ready"}
                      </p>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

function formatTime(isoString: string): string {
  try {
    const d = new Date(isoString);
    return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  } catch {
    return "";
  }
}

function stripMarkdown(text: string): string {
  return text.replace(/[#*`_~[\]()]/g, "").trim();
}
