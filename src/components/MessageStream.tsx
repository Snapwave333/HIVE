import React, { useRef, useEffect } from "react";
import { Message, Agent, TaskChunk } from "../types";
import { Volume2, CornerUpLeft } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { TaskCard } from "./TaskCard";

interface MessageStreamProps {
  messages: Message[];
  agents: Agent[];
  tasks: TaskChunk[];
  onReply: (msg: Message) => void;
  onReact: (messageId: string, emoji: string) => void;
  onPlayVoice: (text: string, agentCodename: string) => void;
  onTaskStepToggle: (taskId: string, stepIndex: number, completed: boolean) => void;
  isMuted: boolean;
}

export const MessageStream: React.FC<MessageStreamProps> = ({
  messages,
  agents,
  tasks,
  onReply,
  onReact,
  onPlayVoice,
  onTaskStepToggle,
  isMuted,
}) => {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages.length]);

  const getAgent = (codename: string): Agent | undefined => {
    return agents.find((a) => a.codename === codename);
  };

  return (
    <div className="flex-1 overflow-y-auto p-4 space-y-3.5 bg-hive-base font-segoe select-text">
      {messages.map((msg) => {
        const isUser = msg.sender === "user";
        const agent = !isUser ? getAgent(msg.sender) : undefined;

        let avatarColor = "#42CFC4";
        if (agent?.avatar_config) {
          try {
            const cfg = JSON.parse(agent.avatar_config);
            avatarColor = cfg.primary_color || avatarColor;
          } catch {
            // ignore
          }
        }

        // Check if message corresponds to a task card
        const associatedTask = tasks.find((t) => msg.content.includes(t.title));

        return (
          <div
            key={msg.id}
            className={`flex flex-col group ${isUser ? "items-end" : "items-start"}`}
          >
            {/* Sender Name & Role Label (for agents) */}
            {!isUser && (
              <div className="flex items-center space-x-2 mb-1 px-1 select-none">
                <span
                  className="text-xs font-semibold"
                  style={{ color: avatarColor }}
                >
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
              {/* Message Markdown Content */}
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
                    strong: ({ children }) => <strong className="text-hive-brightText font-semibold">{children}</strong>,
                  }}
                >
                  {msg.content}
                </ReactMarkdown>
              </div>

              {/* Inline Task Card if present */}
              {associatedTask && (
                <TaskCard task={associatedTask} onStepToggle={onTaskStepToggle} />
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
                    title={isMuted ? "Strict Mute Active: Click to synthesize" : "Listen (Kokoro TTS)"}
                  >
                    <Volume2 className="w-3 h-3" />
                    <span>TTS</span>
                  </button>
                )}
                <span>{formatTime(msg.created_at)}</span>
              </div>

              {/* Hover Action Bar: Reply, React, Kaomoji */}
              <div
                className={`absolute -top-3 hidden group-hover:flex items-center space-x-1 bg-hive-panel border border-hive-border px-1.5 py-0.5 rounded-full shadow-lg z-10 ${
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
                  onClick={() => onReact(msg.id, "(ง'̀-'́)ง")}
                  className="px-1 text-[10px] font-mono hover:text-hive-cyan text-hive-dimText transition-colors"
                  title="React (ง'̀-'́)ง"
                >
                  (ง'̀-'́)ง
                </button>
              </div>
            </div>
          </div>
        );
      })}
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
