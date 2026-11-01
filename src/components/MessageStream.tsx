import React, { useRef, useEffect, useState } from "react";
import { Message, Agent, TaskChunk, Channel } from "../types";
import { Volume2, CornerUpLeft, Edit2, Trash2, Share2, Check, X } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { TaskCard } from "./TaskCard";

interface MessageStreamProps {
  messages: Message[];
  agents: Agent[];
  tasks: TaskChunk[];
  channels: Channel[];
  typingIndicator?: { agent: string; activity: string } | null;
  onReply: (msg: Message) => void;
  onReact: (messageId: string, emoji: string) => void;
  onEdit: (messageId: string, content: string) => void;
  onDelete: (messageId: string) => void;
  onForward: (messageId: string, targetChannelId: number) => void;
  onPlayVoice: (text: string, agentCodename: string) => void;
  onTaskStepToggle: (taskId: string, stepIndex: number, completed: boolean) => void;
  isMuted: boolean;
}

export const MessageStream: React.FC<MessageStreamProps> = ({
  messages,
  agents,
  tasks,
  channels,
  typingIndicator,
  onReply,
  onReact,
  onEdit,
  onDelete,
  onForward,
  onPlayVoice,
  onTaskStepToggle,
  isMuted,
}) => {
  const bottomRef = useRef<HTMLDivElement>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editText, setEditText] = useState<string>("");
  const [forwardingId, setForwardingId] = useState<string | null>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages.length, typingIndicator]);

  const getAgent = (codename: string): Agent | undefined => {
    return agents.find((a) => a.codename === codename);
  };

  return (
    <div className="flex-1 overflow-y-auto p-4 space-y-3.5 bg-hive-base font-segoe select-text">
      {messages.map((msg) => {
        const isUser = msg.sender === "user";
        const agent = !isUser ? getAgent(msg.sender) : undefined;
        const repliedMsg = msg.reply_to_id
          ? messages.find((m) => m.id === msg.reply_to_id)
          : undefined;

        let avatarColor = "#42CFC4";
        if (agent?.avatar_config) {
          try {
            const cfg = JSON.parse(agent.avatar_config);
            avatarColor = cfg.primary_color || avatarColor;
          } catch {
            // ignore
          }
        }

        let reactionMap: Record<string, string[]> = {};
        if (msg.reactions) {
          try {
            reactionMap = JSON.parse(msg.reactions);
          } catch {
            reactionMap = {};
          }
        }

        const associatedTask = tasks.find((t) => msg.content.includes(t.title));

        return (
          <div
            key={msg.id}
            className={`flex flex-col group ${isUser ? "items-end" : "items-start"}`}
          >
            {/* Sender Name & Role Label */}
            {!isUser && (
              <div className="flex items-center space-x-2 mb-1 px-1 select-none">
                <span className="text-xs font-semibold" style={{ color: avatarColor }}>
                  {agent?.display_name || msg.sender}
                </span>
                <span className="text-[10px] bg-hive-surface text-hive-dimText px-1.5 py-0.2 rounded font-mono border border-hive-border">
                  {msg.sender_role || agent?.role || "Agent"}
                </span>
                <span className="text-[10px] text-hive-dimText font-mono">
                  {formatTime(msg.created_at)}
                </span>
              </div>
            )}

            {/* Bubble Container */}
            <div
              className={`max-w-[78%] rounded-xl px-3.5 py-2.5 relative border shadow-sm ${
                isUser
                  ? "bg-hive-outgoing border-hive-outgoingBorder text-hive-brightText rounded-tr-none"
                  : "bg-hive-incoming border-hive-border text-hive-brightText rounded-tl-none"
              }`}
            >
              {/* Reply Quote Banner inside Bubble */}
              {repliedMsg && (
                <div className="mb-2 px-2.5 py-1 rounded bg-hive-base/60 border-l-2 border-hive-cyan text-[11px]">
                  <div className="text-hive-cyan font-semibold">{repliedMsg.sender}</div>
                  <div className="text-hive-dimText truncate">{repliedMsg.content.slice(0, 80)}</div>
                </div>
              )}

              {/* Inline Editing Mode vs Standard Content */}
              {editingId === msg.id ? (
                <div className="space-y-2 min-w-[260px]">
                  <textarea
                    value={editText}
                    onChange={(e) => setEditText(e.target.value)}
                    rows={3}
                    className="w-full bg-hive-base border border-hive-cyan/50 rounded p-2 text-xs text-hive-brightText outline-none"
                  />
                  <div className="flex justify-end space-x-1.5">
                    <button
                      onClick={() => setEditingId(null)}
                      className="px-2 py-1 text-[10px] rounded bg-hive-surface text-hive-dimText hover:text-hive-brightText flex items-center gap-1"
                    >
                      <X className="w-3 h-3" /> Cancel
                    </button>
                    <button
                      onClick={() => {
                        if (editText.trim()) {
                          onEdit(msg.id, editText.trim());
                        }
                        setEditingId(null);
                      }}
                      className="px-2 py-1 text-[10px] rounded bg-hive-cyan text-hive-base font-semibold flex items-center gap-1"
                    >
                      <Check className="w-3 h-3" /> Save
                    </button>
                  </div>
                </div>
              ) : (
                <div className="text-xs leading-relaxed prose prose-invert max-w-none break-words">
                  <ReactMarkdown
                    remarkPlugins={[remarkGfm]}
                    components={{
                      p: ({ children }) => <p className="mb-1.5 last:mb-0">{children}</p>,
                      code: ({ children }) => (
                        <code className="bg-hive-base px-1 py-0.5 rounded text-[11px] font-mono text-hive-cyan border border-hive-border">
                          {children}
                        </code>
                      ),
                      pre: ({ children }) => (
                        <pre className="bg-hive-base p-2.5 rounded-lg border border-hive-border text-[11px] font-mono overflow-x-auto my-2 text-hive-lime">
                          {children}
                        </pre>
                      ),
                      ul: ({ children }) => <ul className="list-disc ml-4 my-1">{children}</ul>,
                      li: ({ children }) => <li className="my-0.5">{children}</li>,
                      strong: ({ children }) => (
                        <strong className="text-hive-brightText font-semibold">{children}</strong>
                      ),
                    }}
                  >
                    {msg.content}
                  </ReactMarkdown>
                </div>
              )}

              {/* Attachments badge */}
              {msg.attachments && (
                <div className="mt-2 px-2.5 py-1.5 rounded bg-hive-base/70 border border-hive-border text-[11px] font-mono text-hive-cyan flex items-center justify-between">
                  <span>📎 Attachment attached</span>
                  <span className="text-hive-dimText text-[10px] truncate max-w-[180px]">
                    {msg.attachments}
                  </span>
                </div>
              )}

              {/* Inline Task Card if present */}
              {associatedTask && (
                <TaskCard task={associatedTask} onStepToggle={onTaskStepToggle} />
              )}

              {/* Reactions Row */}
              {Object.keys(reactionMap).length > 0 && (
                <div className="flex flex-wrap gap-1 mt-2 select-none">
                  {Object.entries(reactionMap).map(([emoji, users]) => (
                    <button
                      key={emoji}
                      onClick={() => onReact(msg.id, emoji)}
                      className="px-2 py-0.5 rounded-full bg-hive-base/80 border border-hive-cyan/40 text-[11px] flex items-center space-x-1 hover:bg-hive-cyan/20 transition-colors"
                    >
                      <span>{emoji}</span>
                      <span className="text-[10px] text-hive-cyan font-mono">{users.length}</span>
                    </button>
                  ))}
                </div>
              )}

              {/* Forward Channel Picker Popover */}
              {forwardingId === msg.id && (
                <div className="mt-2 p-2 rounded bg-hive-panel border border-hive-border shadow-xl text-xs space-y-1.5 select-none">
                  <div className="flex items-center justify-between text-[10px] text-hive-dimText uppercase font-semibold">
                    <span>Forward message to:</span>
                    <button onClick={() => setForwardingId(null)} className="hover:text-hive-brightText">
                      ✕
                    </button>
                  </div>
                  <div className="flex flex-wrap gap-1 max-h-24 overflow-y-auto">
                    {channels.map((ch) => (
                      <button
                        key={ch.id}
                        onClick={() => {
                          onForward(msg.id, ch.id);
                          setForwardingId(null);
                        }}
                        className="px-2 py-0.5 rounded bg-hive-surface hover:bg-hive-cyan/20 text-[11px] border border-hive-border text-hive-brightText"
                      >
                        {ch.id === 0 ? "#agency-hq" : `@${ch.name}`}
                      </button>
                    ))}
                  </div>
                </div>
              )}

              {/* Bubble Bottom Bar: Time & Status */}
              <div
                className={`flex items-center space-x-1.5 mt-1 select-none text-[10px] text-hive-dimText ${
                  isUser ? "justify-end" : "justify-between"
                }`}
              >
                {!isUser && (
                  <button
                    onClick={() => onPlayVoice(msg.content, msg.sender)}
                    className="hover:text-hive-cyan transition-colors flex items-center gap-1"
                    title={
                      isMuted
                        ? "Strict Mute Active: Unmute in header to play speech"
                        : "Listen (Local TTS)"
                    }
                  >
                    <Volume2 className="w-3 h-3" />
                    <span>TTS</span>
                  </button>
                )}
                <div className="flex items-center space-x-1">
                  {msg.is_edited && <span className="italic text-[9px]">(edited)</span>}
                  <span>{formatTime(msg.created_at)}</span>
                </div>
              </div>

              {/* Hover Action Bar: Reply, React, Kaomoji, Forward, Edit, Delete */}
              <div
                className={`absolute -top-3 hidden group-hover:flex items-center space-x-1 bg-hive-panel border border-hive-border px-1.5 py-0.5 rounded-full shadow-lg z-10 select-none ${
                  isUser ? "right-2" : "left-2"
                }`}
              >
                <button
                  onClick={() => onReply(msg)}
                  className="p-1 hover:text-hive-cyan text-hive-dimText transition-colors"
                  title="Reply"
                >
                  <CornerUpLeft className="w-3 h-3" />
                </button>
                <button
                  onClick={() => onReact(msg.id, "👍")}
                  className="p-1 hover:text-hive-lime text-hive-dimText transition-colors text-xs"
                  title="React 👍"
                >
                  👍
                </button>
                <button
                  onClick={() => onReact(msg.id, "🔥")}
                  className="p-1 hover:text-hive-lime text-hive-dimText transition-colors text-xs"
                  title="React 🔥"
                >
                  🔥
                </button>
                <button
                  onClick={() => onReact(msg.id, "(ง'̀-'́)ง")}
                  className="px-1 text-[10px] font-mono hover:text-hive-cyan text-hive-dimText transition-colors"
                  title="React (ง'̀-'́)ง"
                >
                  (ง'̀-'́)ง
                </button>
                <button
                  onClick={() => setForwardingId(forwardingId === msg.id ? null : msg.id)}
                  className="p-1 hover:text-hive-cyan text-hive-dimText transition-colors"
                  title="Forward Message"
                >
                  <Share2 className="w-3 h-3" />
                </button>
                {isUser && !msg.is_deleted && (
                  <>
                    <button
                      onClick={() => {
                        setEditingId(msg.id);
                        setEditText(msg.content);
                      }}
                      className="p-1 hover:text-hive-cyan text-hive-dimText transition-colors"
                      title="Edit Message"
                    >
                      <Edit2 className="w-3 h-3" />
                    </button>
                    <button
                      onClick={() => onDelete(msg.id)}
                      className="p-1 hover:text-red-400 text-hive-dimText transition-colors"
                      title="Delete Message"
                    >
                      <Trash2 className="w-3 h-3" />
                    </button>
                  </>
                )}
              </div>
            </div>
          </div>
        );
      })}

      {/* Live Agent Typing & Activity Indicator */}
      {typingIndicator && (
        <div className="flex items-center space-x-2 px-3 py-2 rounded-lg bg-hive-surface/70 border border-hive-cyan/30 w-fit animate-pulse select-none">
          <span className="w-2 h-2 rounded-full bg-hive-lime" />
          <span className="text-xs font-semibold text-hive-cyan">@{typingIndicator.agent}</span>
          <span className="text-xs text-hive-dimText">{typingIndicator.activity}</span>
        </div>
      )}

      <div ref={bottomRef} />
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
