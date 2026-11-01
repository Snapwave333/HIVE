import React, { useState } from "react";
import { Channel, Agent, Message, TaskChunk } from "../types";
import { Search, Pin, CheckSquare, Plus } from "lucide-react";
import { TaskCard } from "./TaskCard";

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
    <div className="w-80 bg-hive-panel border-r border-hive-border flex flex-col h-full select-none">
      {/* Header & Instant Search Bar */}
      <div className="p-3 border-b border-hive-border flex flex-col space-y-2">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold tracking-wide text-hive-brightText uppercase flex items-center gap-1.5">
            <span className="text-hive-cyan">⬡</span> Chats
          </h2>
          <span className="text-xs text-hive-dimText font-mono">13 channels</span>
        </div>

        {/* Search Input (Ctrl+K trigger) */}
        <div
          onClick={onOpenSearch}
          className="flex items-center px-2.5 py-1.5 rounded bg-hive-surface border border-hive-border hover:border-hive-cyan/50 text-hive-dimText cursor-pointer transition-colors text-xs space-x-2"
        >
          <Search className="w-3.5 h-3.5 text-hive-cyan" />
          <input
            type="text"
            placeholder="Search messages & memories (Ctrl+K)..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onClick={(e) => e.stopPropagation()}
            className="bg-transparent border-none outline-none text-hive-brightText placeholder-hive-dimText w-full text-xs"
          />
          <kbd className="text-[10px] bg-hive-base px-1.5 py-0.5 rounded border border-hive-border text-hive-dimText font-mono">
            Ctrl+K
          </kbd>
        </div>

        {/* Filter Pills */}
        <div className="flex space-x-1.5 pt-1">
          <button
            onClick={() => setFilter("all")}
            className={`px-2.5 py-0.5 rounded text-xs font-medium transition-colors ${
              filter === "all"
                ? "bg-hive-surface text-hive-cyan border border-hive-cyan/30"
                : "text-hive-dimText hover:text-hive-brightText"
            }`}
          >
            All Lines
          </button>
          <button
            onClick={() => setFilter("tasks")}
            className={`px-2.5 py-0.5 rounded text-xs font-medium transition-colors flex items-center gap-1 ${
              filter === "tasks"
                ? "bg-hive-surface text-hive-lime border border-hive-lime/30"
                : "text-hive-dimText hover:text-hive-brightText"
            }`}
          >
            <CheckSquare className="w-3 h-3" /> 5m Tasks ({tasks.length})
          </button>
        </div>
      </div>

      {/* Stream: Either 5-Minute Tasks View or Conversations Stream */}
      {filter === "tasks" ? (
        <div className="flex-1 overflow-y-auto p-2.5 space-y-2">
          <div className="flex items-center justify-between px-1">
            <span className="text-[10px] uppercase tracking-wider font-semibold text-hive-dimText">
              ADHD 5-Minute Task Queue
            </span>
            <button
              onClick={() => setShowNewTaskForm(!showNewTaskForm)}
              className="text-[11px] text-hive-lime hover:underline flex items-center gap-0.5"
            >
              <Plus className="w-3 h-3" /> New 5m Task
            </button>
          </div>

          {showNewTaskForm && (
            <div className="p-2.5 rounded-lg bg-hive-surface border border-hive-lime/40 space-y-2 text-xs">
              <input
                type="text"
                placeholder="Task title (chunked into 5m steps)..."
                value={newTaskTitle}
                onChange={(e) => setNewTaskTitle(e.target.value)}
                className="w-full bg-hive-base border border-hive-border rounded px-2 py-1 text-hive-brightText outline-none"
              />
              <div className="flex items-center justify-between">
                <select
                  value={newTaskAssignee}
                  onChange={(e) => setNewTaskAssignee(e.target.value)}
                  className="bg-hive-base border border-hive-border rounded px-2 py-1 text-hive-cyan text-[11px]"
                >
                  {agents.map((a) => (
                    <option key={a.codename} value={a.codename}>
                      @{a.codename}
                    </option>
                  ))}
                </select>
                <button
                  onClick={handleAddTask}
                  className="px-2.5 py-1 rounded bg-hive-lime text-hive-base font-semibold text-[11px]"
                >
                  Create
                </button>
              </div>
            </div>
          )}

          {tasks.map((t) => (
            <TaskCard key={t.id} task={t} onStepToggle={onTaskStepToggle} />
          ))}
        </div>
      ) : (
        <div className="flex-1 overflow-y-auto divide-y divide-hive-border/40">
          {/* Pinned Section: #agency-hq */}
          {hqChannel && (
            <div className="p-1.5 bg-hive-base/30">
              <div className="text-[10px] uppercase tracking-wider font-semibold text-hive-dimText px-2 py-1 flex items-center gap-1">
                <Pin className="w-2.5 h-2.5 text-hive-lime" /> Pinned
              </div>
              <div
                onClick={() => onSelectChannel(0)}
                className={`p-2.5 rounded-lg cursor-pointer transition-all flex items-start space-x-3 relative ${
                  activeChannelId === 0
                    ? "bg-hive-surface border border-hive-cyan/40 shadow-sm"
                    : "hover:bg-hive-surface/60"
                }`}
              >
                <div className="w-10 h-10 rounded-lg bg-hive-surface border border-hive-lime/40 flex items-center justify-center text-hive-lime font-bold text-lg flex-shrink-0">
                  ⬡
                </div>

                <div className="flex-1 min-w-0">
                  <div className="flex items-center justify-between">
                    <span className="font-semibold text-sm text-hive-brightText flex items-center gap-1 truncate">
                      #agency-hq
                    </span>
                    <span className="text-[10px] text-hive-dimText font-mono">
                      {latestMessages[0] ? formatTime(latestMessages[0].created_at) : "Active"}
                    </span>
                  </div>
                  <p className="text-xs text-hive-dimText truncate mt-0.5">
                    {latestMessages[0]?.content
                      ? stripMarkdown(latestMessages[0].content)
                      : "Primary Agency Town Square"}
                  </p>
                  <div className="flex items-center gap-1.5 mt-1.5">
                    <span className="text-[10px] bg-hive-base text-hive-lime border border-hive-lime/30 px-1.5 py-0.2 rounded font-mono">
                      12 Agents
                    </span>
                    <span className="text-[10px] text-hive-cyan truncate">
                      Collective Town Square
                    </span>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* 12 Permanent Agent DMs */}
          <div className="p-1.5">
            <div className="text-[10px] uppercase tracking-wider font-semibold text-hive-dimText px-2 py-1.5 flex items-center justify-between">
              <span>Direct Messages</span>
              <span className="text-hive-cyan font-mono">12 Permanent</span>
            </div>

            <div className="space-y-1">
              {filteredDms.map((channel) => {
                const agent = getAgentForChannel(channel);
                const isActive = activeChannelId === channel.id;
                const lastMsg = latestMessages[channel.id];

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
                  <div
                    key={channel.id}
                    onClick={() => onSelectChannel(channel.id)}
                    className={`p-2 rounded-lg cursor-pointer transition-all flex items-start space-x-2.5 relative ${
                      isActive
                        ? "bg-hive-surface border border-hive-cyan/40 shadow-sm"
                        : "hover:bg-hive-surface/60"
                    }`}
                  >
                    <div className="relative flex-shrink-0">
                      <div
                        className="w-9 h-9 rounded-full flex items-center justify-center font-bold text-xs border"
                        style={{
                          backgroundColor: `${avatarColor}15`,
                          borderColor: `${avatarColor}40`,
                          color: avatarColor,
                        }}
                      >
                        {channel.name.slice(0, 2).toUpperCase()}
                      </div>
                      <span
                        className={`absolute bottom-0 right-0 w-2.5 h-2.5 rounded-full border-2 border-hive-panel ${
                          agent?.status === "thinking"
                            ? "bg-yellow-400 animate-pulse"
                            : agent?.status === "busy"
                            ? "bg-red-400"
                            : "bg-hive-lime"
                        }`}
                        title={`Status: ${agent?.status || "available"}`}
                      />
                    </div>

                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between">
                        <span className="font-medium text-xs text-hive-brightText truncate flex items-center gap-1">
                          {channel.name}
                          <span className="text-[10px] text-hive-dimText font-normal">
                            @{channel.agent_codename}
                          </span>
                        </span>
                        <span className="text-[10px] text-hive-dimText font-mono">
                          {lastMsg ? formatTime(lastMsg.created_at) : ""}
                        </span>
                      </div>

                      <p className="text-[11px] text-hive-cyan truncate mt-0.5">
                        {agent?.role || channel.description}
                      </p>

                      <p className="text-xs text-hive-dimText truncate mt-0.5">
                        {lastMsg ? stripMarkdown(lastMsg.content) : "Click to direct message..."}
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
