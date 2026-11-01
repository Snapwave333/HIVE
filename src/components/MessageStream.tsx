import React, { useRef, useEffect, useState } from "react";
import { Message, Agent, TaskChunk, Channel } from "../types";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { TaskCard } from "./TaskCard";
import { M3Icon } from "./M3Icon";

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
    <div className="flex-1 overflow-y-auto px-8 py-6 bg-hive-base font-segoe select-text">
      <div className="max-w-5xl mx-auto w-full space-y-5">
      {messages.map((msg) => {
        const isUser = msg.sender === "user";
        const agent = !isUser ? getAgent(msg.sender) : undefined;
        const repliedMsg = msg.reply_to_id
          ? messages.find((m) => m.id === msg.reply_to_id)
          : undefined;

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
            className={`flex items-start gap-3 group ${isUser ? "justify-end" : "justify-start"}`}
          >
            {!isUser && (
              <img
                src={`/avatars/${agent?.codename || msg.sender}.png`}
                alt={agent?.display_name || msg.sender}
                onError={(e) => {
                  (e.currentTarget as HTMLImageElement).src = "/avatars/PRIME.png";
                }}
                className="w-9 h-9 rounded-xl object-cover border border-hive-lime/40 flex-shrink-0 mt-1"
              />
            )}

            <div className={`flex flex-col max-w-[82%] ${isUser ? "items-end" : "items-start"}`}>
            {/* Sender Name & Role Label */}
            {!isUser && (
              <div className="flex items-center space-x-2.5 mb-1.5 px-1 select-none">
                <span className="text-xs font-semibold text-hive-lime tracking-wide">
                  @{agent?.codename || msg.sender}
                </span>
                <span className="text-[11px] text-hive-brightText font-medium">
                  {agent?.display_name || msg.sender}
                </span>
                <span className="text-[10px] bg-hive-surface text-hive-dimText px-2 py-0.5 rounded-md font-mono border border-hive-border">
                  {msg.sender_role || agent?.role || "Agent"}
                </span>
                <span className="text-[10px] text-hive-dimText font-mono">
                  {formatTime(msg.created_at)}
                </span>
              </div>
            )}

            {/* Bubble Container */}
            <div
              className={`w-full rounded-2xl px-4 py-3 relative border transition-colors ${
                isUser
                  ? "bg-hive-outgoing border-hive-outgoingBorder text-hive-brightText rounded-tr-sm"
                  : "bg-hive-incoming border-hive-border text-hive-brightText rounded-tl-sm"
              }`}
            >
              {/* Reply Quote Banner inside Bubble */}
              {repliedMsg && (
                <div className="mb-2.5 px-3 py-1.5 rounded-lg bg-hive-base border-l-2 border-hive-lime text-[11px]">
                  <div className="text-hive-lime font-semibold">@{repliedMsg.sender}</div>
                  <div className="text-hive-dimText truncate mt-0.5">{repliedMsg.content.slice(0, 90)}</div>
                </div>
              )}

              {/* Inline Editing Mode vs Standard Content */}
              {editingId === msg.id ? (
                <div className="space-y-2.5 min-w-[300px]">
                  <textarea
                    value={editText}
                    onChange={(e) => setEditText(e.target.value)}
                    rows={3}
                    className="w-full bg-hive-base border border-hive-lime/50 rounded-lg p-2.5 text-xs text-hive-brightText outline-none"
                  />
                  <div className="flex justify-end space-x-2">
                    <button
                      onClick={() => setEditingId(null)}
                      className="px-2.5 py-1 text-[11px] rounded-lg bg-hive-surface text-hive-dimText hover:text-hive-brightText flex items-center gap-1 border border-hive-border"
                    >
                      <M3Icon name="close" size={13} /> Cancel
                    </button>
                    <button
                      onClick={() => {
                        if (editText.trim()) {
                          onEdit(msg.id, editText.trim());
                        }
                        setEditingId(null);
                      }}
                      className="px-2.5 py-1 text-[11px] rounded-lg bg-hive-lime text-hive-base font-semibold flex items-center gap-1"
                    >
                      <M3Icon name="check" size={13} /> Save
                    </button>
                  </div>
                </div>
              ) : (
                <div className="text-[13px] leading-relaxed prose prose-invert max-w-none break-words">
                  <ReactMarkdown
                    remarkPlugins={[remarkGfm]}
                    components={{
                      p: ({ children }) => <p className="mb-2 last:mb-0">{children}</p>,
                      code: ({ children }) => (
                        <code className="bg-hive-base px-1.5 py-0.5 rounded-md text-[11.5px] font-mono text-hive-lime border border-hive-border">
                          {children}
                        </code>
                      ),
                      pre: ({ children }) => (
                        <pre className="bg-hive-base p-3 rounded-xl border border-hive-border text-[11.5px] font-mono overflow-x-auto my-2.5 text-hive-lime">
                          {children}
                        </pre>
                      ),
                      ul: ({ children }) => <ul className="list-disc ml-5 my-1.5 space-y-1">{children}</ul>,
                      ol: ({ children }) => <ol className="list-decimal ml-5 my-1.5 space-y-1">{children}</ol>,
                      li: ({ children }) => <li className="leading-relaxed">{children}</li>,
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
                <div className="mt-2.5 px-3 py-2 rounded-lg bg-hive-base border border-hive-border text-[11px] font-mono text-hive-lime flex items-center justify-between">
                  <span className="flex items-center gap-1.5">
                    <M3Icon name="attach_file" size={14} /> Attachment attached
                  </span>
                  <span className="text-hive-dimText text-[10px] truncate max-w-[200px]">
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
                <div className="flex flex-wrap gap-1.5 mt-2.5 select-none">
                  {Object.entries(reactionMap).map(([emoji, users]) => (
                    <button
                      key={emoji}
                      onClick={() => onReact(msg.id, emoji)}
                      className="px-2.5 py-0.5 rounded-full bg-hive-base border border-hive-lime/40 text-[11px] flex items-center space-x-1.5 hover:bg-hive-lime/15 transition-colors"
                    >
                      <span>{emoji}</span>
                      <span className="text-[10px] text-hive-lime font-mono">{users.length}</span>
                    </button>
                  ))}
                </div>
              )}

              {/* Forward Channel Picker Popover */}
              {forwardingId === msg.id && (
                <div className="mt-2.5 p-2.5 rounded-xl bg-hive-panel border border-hive-border shadow-2xl text-xs space-y-2 select-none">
                  <div className="flex items-center justify-between text-[10px] text-hive-dimText uppercase font-semibold">
                    <span>Forward message to:</span>
                    <button onClick={() => setForwardingId(null)} className="hover:text-hive-brightText">
                      <M3Icon name="close" size={13} />
                    </button>
                  </div>
                  <div className="flex flex-wrap gap-1.5 max-h-28 overflow-y-auto">
                    {channels.map((ch) => (
                      <button
                        key={ch.id}
                        onClick={() => {
                          onForward(msg.id, ch.id);
                          setForwardingId(null);
                        }}
                        className="px-2.5 py-1 rounded-lg bg-hive-surface hover:border-hive-lime/50 text-[11px] border border-hive-border text-hive-brightText transition-colors"
                      >
                        {ch.id === 0 ? "#agency-hq" : `@${ch.name}`}
                      </button>
                    ))}
                  </div>
                </div>
              )}

              {/* Bubble Bottom Bar: Time & Status */}
              <div
                className={`flex items-center space-x-2 mt-2 pt-1 border-t border-hive-border/50 select-none text-[10px] text-hive-dimText ${
                  isUser ? "justify-end" : "justify-between"
                }`}
              >
                {!isUser && (
                  <button
                    onClick={() => onPlayVoice(msg.content, msg.sender)}
                    className="hover:text-hive-lime transition-colors flex items-center gap-1"
                    title={
                      isMuted
                        ? "Strict Mute Active: Unmute in header to play speech"
                        : "Listen (Local TTS)"
                    }
                  >
                    <M3Icon name="record_voice_over" size={13} />
                    <span>TTS</span>
                  </button>
                )}
                <div className="flex items-center space-x-1.5 font-mono">
                  {msg.is_edited && <span className="italic text-[9px]">(edited)</span>}
                  <span>{formatTime(msg.created_at)}</span>
                </div>
              </div>

              {/* Hover Action Bar: Reply, React, Kaomoji, Forward, Edit, Delete */}
              <div
                className={`absolute -top-3.5 hidden group-hover:flex items-center space-x-1 bg-hive-panel border border-hive-border px-2 py-1 rounded-full shadow-xl z-10 select-none ${
                  isUser ? "right-3" : "left-3"
                }`}
              >
                <button
                  onClick={() => onReply(msg)}
                  className="p-1 hover:text-hive-lime text-hive-dimText transition-colors"
                  title="Reply"
                >
                  <M3Icon name="reply" size={14} />
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
                  className="px-1 text-[10px] font-mono hover:text-hive-lime text-hive-dimText transition-colors"
                  title="React (ง'̀-'́)ง"
                >
                  (ง'̀-'́)ง
                </button>
                <button
                  onClick={() => setForwardingId(forwardingId === msg.id ? null : msg.id)}
                  className="p-1 hover:text-hive-lime text-hive-dimText transition-colors"
                  title="Forward Message"
                >
                  <M3Icon name="forward" size={14} />
                </button>
                {isUser && !msg.is_deleted && (
                  <>
                    <button
                      onClick={() => {
                        setEditingId(msg.id);
                        setEditText(msg.content);
                      }}
                      className="p-1 hover:text-hive-lime text-hive-dimText transition-colors"
                      title="Edit Message"
                    >
                      <M3Icon name="edit" size={14} />
                    </button>
                    <button
                      onClick={() => onDelete(msg.id)}
                      className="p-1 hover:text-hive-brightText text-hive-dimText transition-colors"
                      title="Delete Message"
                    >
                      <M3Icon name="delete" size={14} />
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
        <div className="flex items-center space-x-2.5 px-4 py-2.5 rounded-xl bg-hive-surface border border-hive-lime/30 w-fit animate-pulse select-none">
          <span className="w-2 h-2 rounded-full bg-hive-lime" />
          <span className="text-xs font-semibold text-hive-lime">@{typingIndicator.agent}</span>
          <span className="text-xs text-hive-dimText">{typingIndicator.activity}</span>
        </div>
      )}

      <div ref={bottomRef} />
      </div>
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
