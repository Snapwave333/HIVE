import React, { useRef, useEffect, useLayoutEffect, useState } from "react";
import { Message, Agent, TaskChunk, Channel, AttachmentRef } from "../types";
import { api } from "../services/api";
import ReactMarkdown, { Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { TaskCard } from "./TaskCard";
import { M3Icon } from "./M3Icon";

const markdownComponents: Components = {
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
  a: ({ href, children }) => (
    <a
      href={href}
      title={href}
      onClick={(e) => {
        e.preventDefault();
        if (href) api.openExternal(href).catch((err) => console.warn("Link not opened:", err));
      }}
      className="text-hive-cyan underline decoration-hive-cyan/40 hover:decoration-hive-cyan cursor-pointer break-all"
    >
      {children}
    </a>
  ),
  img: ({ src, alt }) =>
    typeof src === "string" && src.startsWith("data:image/") ? (
      <img src={src} alt={alt ?? ""} className="max-w-[320px] max-h-60 rounded-lg border border-hive-border" />
    ) : (
      <a
        href={typeof src === "string" ? src : undefined}
        onClick={(e) => {
          e.preventDefault();
          if (typeof src === "string") api.openExternal(src).catch((err) => console.warn("Link not opened:", err));
        }}
        className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-hive-base border border-hive-border text-[11px] text-hive-cyan cursor-pointer"
        title={`Remote image not loaded: ${typeof src === "string" ? src : ""}`}
      >
        <M3Icon name="image" size={13} /> {alt || "image"} (open in browser)
      </a>
    ),
};

const remarkPlugins = [remarkGfm];

const imageCache = new Map<string, Promise<string | null>>();

function loadImage(hash: string): Promise<string | null> {
  if (!imageCache.has(hash)) {
    imageCache.set(
      hash,
      api
        .getAttachment(hash)
        .then((a) => a.data_url ?? null)
        .catch(() => null)
    );
  }
  return imageCache.get(hash)!;
}

function parseAttachments(raw?: string): AttachmentRef[] | null {
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${bytes} B`;
}

const AttachmentImage: React.FC<{ attachment: AttachmentRef }> = ({ attachment }) => {
  const [src, setSrc] = useState<string | null | undefined>(undefined);
  useEffect(() => {
    let alive = true;
    loadImage(attachment.hash).then((url) => alive && setSrc(url));
    return () => {
      alive = false;
    };
  }, [attachment.hash]);
  if (src === undefined) {
    return <div className="w-48 h-32 rounded-lg bg-hive-base border border-hive-border animate-pulse" />;
  }
  if (src === null) {
    return <AttachmentChip attachment={attachment} />;
  }
  return (
    <a href={src} download={attachment.filename} title={attachment.filename}>
      <img src={src} alt={attachment.filename} className="max-w-[320px] max-h-60 rounded-lg border border-hive-border object-contain" />
    </a>
  );
};

const AttachmentChip: React.FC<{ attachment: AttachmentRef }> = ({ attachment }) => (
  <span className="px-2.5 py-1.5 rounded-lg bg-hive-base border border-hive-border text-[11px] flex items-center gap-1.5 max-w-[280px]">
    <M3Icon name="description" size={14} className="text-hive-lime" />
    <span className="truncate text-hive-brightText">{attachment.filename}</span>
    <span className="text-hive-dimText font-mono text-[10px]">{formatSize(attachment.size_bytes || 0)}</span>
  </span>
);

const Attachments: React.FC<{ raw: string }> = ({ raw }) => {
  const list = parseAttachments(raw);
  if (list === null) {
    return (
      <div className="mt-2.5 px-3 py-2 rounded-lg bg-hive-base border border-hive-border text-[11px] font-mono text-hive-lime flex items-center gap-1.5">
        <M3Icon name="attach_file" size={14} />
        <span className="truncate max-w-[320px] text-hive-dimText">{raw}</span>
      </div>
    );
  }
  if (list.length === 0) return null;
  return (
    <div className="mt-2.5 flex flex-wrap gap-2">
      {list.map((a) =>
        (a.mime_type || "").startsWith("image/") ? (
          <AttachmentImage key={a.hash} attachment={a} />
        ) : (
          <AttachmentChip key={a.hash} attachment={a} />
        )
      )}
    </div>
  );
};

const MarkdownContent = React.memo(({ content }: { content: string }) => (
  <ReactMarkdown remarkPlugins={remarkPlugins} components={markdownComponents}>
    {content}
  </ReactMarkdown>
));

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
  highlightMessageId?: string | null;
  hasOlder?: boolean;
  loadingOlder?: boolean;
  onLoadOlder?: () => void;
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
  highlightMessageId,
  hasOlder,
  loadingOlder,
  onLoadOlder,
}) => {
  const bottomRef = useRef<HTMLDivElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const nearBottomRef = useRef(true);
  const prevFirstIdRef = useRef<string | undefined>(undefined);
  const prevHeightRef = useRef(0);
  const lastIdRef = useRef<string | undefined>(undefined);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editText, setEditText] = useState<string>("");
  const [forwardingId, setForwardingId] = useState<string | null>(null);

  useLayoutEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const firstId = messages[0]?.id;
    const lastId = messages[messages.length - 1]?.id;
    const prepended = prevFirstIdRef.current !== undefined && firstId !== prevFirstIdRef.current && lastId === lastIdRef.current;
    if (prepended) {
      el.scrollTop += el.scrollHeight - prevHeightRef.current;
    } else if (lastId !== lastIdRef.current) {
      const isOwn = messages[messages.length - 1]?.sender === "user";
      if (lastIdRef.current === undefined || isOwn || nearBottomRef.current) {
        bottomRef.current?.scrollIntoView({ behavior: lastIdRef.current === undefined ? "auto" : "smooth" });
      }
    }
    prevFirstIdRef.current = firstId;
    lastIdRef.current = lastId;
    prevHeightRef.current = el.scrollHeight;
  }, [messages]);

  useEffect(() => {
    if (typingIndicator && nearBottomRef.current) {
      bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [typingIndicator]);

  useEffect(() => {
    if (!highlightMessageId) return;
    const el = document.getElementById(`msg-${highlightMessageId}`);
    if (el) el.scrollIntoView({ behavior: "smooth", block: "center" });
  }, [highlightMessageId, messages]);

  const handleScroll = () => {
    const el = scrollRef.current;
    if (!el) return;
    nearBottomRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < 120;
    prevHeightRef.current = el.scrollHeight;
    if (el.scrollTop < 80 && hasOlder && !loadingOlder && onLoadOlder) onLoadOlder();
  };

  const getAgent = (codename: string): Agent | undefined => {
    return agents.find((a) => a.codename === codename);
  };

  return (
    <div ref={scrollRef} onScroll={handleScroll} className="flex-1 overflow-y-auto px-8 py-6 bg-hive-base select-text">
      <div className="max-w-5xl mx-auto w-full space-y-5">
      {hasOlder && (
        <div className="flex justify-center">
          <button
            onClick={onLoadOlder}
            disabled={loadingOlder}
            className="px-3 py-1 rounded-full bg-hive-surface border border-hive-border text-[11px] text-hive-dimText hover:text-hive-brightText disabled:opacity-60"
          >
            {loadingOlder ? "Loading…" : "Load older messages"}
          </button>
        </div>
      )}
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
            id={`msg-${msg.id}`}
            className={`flex flex-col group rounded-2xl transition-shadow ${isUser ? "items-end" : "items-start"} ${
              highlightMessageId === msg.id ? "ring-2 ring-hive-lime/60 ring-offset-4 ring-offset-hive-base" : ""
            }`}
          >
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
              className={`max-w-[82%] rounded-2xl px-4 py-3 relative border transition-colors ${
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
                  {msg.content.trim() && <MarkdownContent content={msg.content} />}
                </div>
              )}

              {msg.attachments && <Attachments raw={msg.attachments} />}

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
                    onClick={() => !isMuted && onPlayVoice(msg.content, msg.sender)}
                    aria-disabled={isMuted}
                    className={`transition-colors flex items-center gap-1 ${
                      isMuted ? "opacity-60 cursor-not-allowed" : "hover:text-hive-lime"
                    }`}
                    title={isMuted ? "Speech is muted. Unmute with the speaker button to listen." : "Listen to this message"}
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
