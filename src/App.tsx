import React, { useState, useEffect, useRef, Component, ErrorInfo, ReactNode } from "react";
import { Channel, Agent, Message, TaskChunk, BridgeStatus, Constitution } from "./types";
import { api } from "./services/api";
import { NavRail } from "./components/NavRail";
import { ConversationList } from "./components/ConversationList";
import { ChatHeader } from "./components/ChatHeader";
import { MessageStream } from "./components/MessageStream";
import { MessageComposer } from "./components/MessageComposer";
import { SettingsModal } from "./components/SettingsModal";
import { SearchModal } from "./components/SearchModal";

class HiveErrorBoundary extends Component<
  { children: ReactNode },
  { hasError: boolean; errorMsg: string; recovery: string }
> {
  constructor(props: { children: ReactNode }) {
    super(props);
    this.state = { hasError: false, errorMsg: "", recovery: "Attempting rollback..." };
  }

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, errorMsg: error.message };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("H.I.V.E. UI error, attempting rollback:", error, errorInfo);
    api
      .rollbackUiVersion(`Auto-rollback from ErrorBoundary: ${error.message}`)
      .then((dep) => this.setState({ recovery: `Rolled back to ${dep.version}. The window will reload onto it.` }))
      .catch((e) => this.setState({ recovery: `No rollback available (${String(e)}). Reload to try again.` }));
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="h-screen w-screen bg-hive-base text-hive-brightText flex flex-col items-center justify-center p-6 space-y-3 font-segoe">
          <div className="text-hive-lime font-bold text-lg">⬡ H.I.V.E. hit a UI error</div>
          <p className="text-xs text-hive-dimText max-w-md text-center">{this.state.errorMsg}</p>
          <p className="text-xs text-hive-brightText max-w-md text-center">{this.state.recovery}</p>
          <button
            onClick={() => window.location.reload()}
            className="px-4 py-1.5 rounded bg-hive-cyan text-hive-base font-semibold text-xs"
          >
            Reload
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

type TypingState = { agent: string; activity: string };

const appendUnique = (list: Message[], msg: Message): Message[] =>
  list.some((m) => m.id === msg.id) ? list : [...list, msg];

const FONT_SIZE_MIN = 12;
const FONT_SIZE_MAX = 20;

export const App: React.FC = () => {
  const [channels, setChannels] = useState<Channel[]>([]);
  const [activeChannelId, setActiveChannelId] = useState<number>(0);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);
  const [tasks, setTasks] = useState<TaskChunk[]>([]);
  const [latestMessages, setLatestMessages] = useState<Record<number, Message>>({});
  const [isMuted, setIsMuted] = useState<boolean>(true);
  const [agencyState, setAgencyState] = useState<string>("running");
  const [isFocusMode, setIsFocusMode] = useState<boolean>(false);
  const [currentFont, setCurrentFont] = useState<string>("segoe");
  const [fontSize, setFontSize] = useState<number>(14);
  const [currentTab, setCurrentTab] = useState<"chats" | "settings">("chats");
  const [isSettingsOpen, setIsSettingsOpen] = useState<boolean>(false);
  const [isSearchOpen, setIsSearchOpen] = useState<boolean>(false);
  const [replyingTo, setReplyingTo] = useState<{ id: string; sender: string; snippet: string } | null>(null);
  const [typingByChannel, setTypingByChannel] = useState<Record<number, Record<string, TypingState>>>({});
  const [constitution, setConstitution] = useState<Constitution | null>(null);
  const [bridgeStatus, setBridgeStatus] = useState<BridgeStatus | null>(null);

  const activeChannelRef = useRef<number>(activeChannelId);
  activeChannelRef.current = activeChannelId;

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [chList, agList, tList, muteState, agState, constData, font, size] = await Promise.all([
          api.getChannels(),
          api.getAgents(),
          api.getTasks(),
          api.getAudioMuteState(),
          api.getAgencyState(),
          api.getConstitution().catch(() => null),
          api.getSetting("font").catch(() => null),
          api.getSetting("font_size").catch(() => null),
        ]);
        if (cancelled) return;
        setChannels(chList);
        setAgents(agList);
        setTasks(tList);
        setIsMuted(muteState);
        setAgencyState(agState);
        if (constData) setConstitution(constData);
        if (font) setCurrentFont(font);
        const parsedSize = Number(size);
        if (parsedSize >= FONT_SIZE_MIN && parsedSize <= FONT_SIZE_MAX) setFontSize(parsedSize);

        const previews = await Promise.all(
          chList.map((c) =>
            api
              .getMessages(c.id, 1)
              .then((m) => [c.id, m[m.length - 1]] as const)
              .catch(() => [c.id, undefined] as const)
          )
        );
        if (cancelled) return;
        setLatestMessages((prev) => {
          const next = { ...prev };
          for (const [id, m] of previews) if (m && !next[id]) next[id] = m;
          return next;
        });
      } catch (err) {
        console.error("Failed to load initial H.I.V.E. data:", err);
      }
    })();
    api.getBridgeStatus().then((b) => !cancelled && setBridgeStatus(b)).catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    setMessages([]);
    api
      .getMessages(activeChannelId)
      .then((msgs) => {
        if (cancelled) return;
        setMessages(msgs);
        if (msgs.length > 0) {
          setLatestMessages((prev) => ({ ...prev, [activeChannelId]: msgs[msgs.length - 1] }));
        }
      })
      .catch((err) => console.error("Failed to load messages:", err));
    return () => {
      cancelled = true;
    };
  }, [activeChannelId]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;

    if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) return;

    (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      const stop = await listen<any>("hive-event", (event) => {
        const payload = event.payload;
        if (!payload) return;
        const data = payload.payload;

        if (payload.type === "agent.message.created") {
          const newMsg: Message = {
            id: data.message_id,
            channel_id: data.channel_id,
            sender: data.sender,
            sender_role: data.sender_role ?? undefined,
            content: data.content,
            is_pinned: false,
            is_edited: false,
            is_deleted: false,
            created_at: data.timestamp,
            updated_at: data.timestamp,
          };
          setLatestMessages((prev) => ({ ...prev, [data.channel_id]: newMsg }));
          if (data.channel_id === activeChannelRef.current) {
            setMessages((prev) => appendUnique(prev, newMsg));
          }
        } else if (payload.type === "agent.typing") {
          const codename: string = data.codename;
          setAgents((prev) =>
            prev.map((a) =>
              a.codename === codename ? { ...a, status: data.is_typing ? "thinking" : "available" } : a
            )
          );
          setTypingByChannel((prev) => {
            const forChannel = { ...(prev[data.channel_id] || {}) };
            if (data.is_typing) {
              forChannel[codename] = { agent: codename, activity: data.activity };
            } else {
              delete forChannel[codename];
            }
            return { ...prev, [data.channel_id]: forChannel };
          });
        } else if (payload.type === "task.step_completed" || payload.type === "task.created") {
          api.getTasks().then(setTasks).catch(() => {});
        } else if (payload.type === "system.lifecycle") {
          setAgencyState(data.state);
        } else if (payload.type === "audio.mute_toggled") {
          setIsMuted(data.is_muted);
        }
      });
      if (disposed) stop();
      else unlisten = stop;
    })();

    return () => {
      disposed = true;
      if (unlisten) unlisten();
    };
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setIsSearchOpen(true);
      }
      if (e.key === "F11" && e.ctrlKey) {
        e.preventDefault();
        setIsFocusMode((prev) => !prev);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  const handleSelectFont = (font: string) => {
    setCurrentFont(font);
    api.setSetting("font", font).catch(() => {});
  };

  const handleSetFontSize = (size: number) => {
    const clamped = Math.min(FONT_SIZE_MAX, Math.max(FONT_SIZE_MIN, size));
    setFontSize(clamped);
    api.setSetting("font_size", String(clamped)).catch(() => {});
  };

  const typingAgents = Object.values(typingByChannel[activeChannelId] || {});
  const typingIndicator: TypingState | null =
    typingAgents.length === 0
      ? null
      : typingAgents.length === 1
      ? typingAgents[0]
      : {
          agent: typingAgents.map((t) => t.agent).join(", @"),
          activity: "are typing...",
        };

  const handleSendMessage = async (
    content: string,
    replyToId?: string,
    attachmentsJson?: string
  ) => {
    try {
      const msg = await api.sendMessage(activeChannelId, content, attachmentsJson, replyToId);
      setMessages((prev) => appendUnique(prev, msg));
      setLatestMessages((prev) => ({
        ...prev,
        [activeChannelId]: msg,
      }));
    } catch (err) {
      console.error("Send message error:", err);
    }
  };

  const handleEditMessage = async (messageId: string, content: string) => {
    await api.editMessage(messageId, content);
    const updated = await api.getMessages(activeChannelId);
    setMessages(updated);
  };

  const handleDeleteMessage = async (messageId: string) => {
    await api.deleteMessage(messageId);
    const updated = await api.getMessages(activeChannelId);
    setMessages(updated);
  };

  const handleForwardMessage = async (messageId: string, targetChannelId: number) => {
    const forwarded = await api.forwardMessage(messageId, targetChannelId);
    if (targetChannelId === activeChannelId) {
      setMessages((prev) => appendUnique(prev, forwarded));
    }
  };

  const handleReactMessage = async (messageId: string, emoji: string) => {
    const updatedReactions = await api.toggleReaction(messageId, emoji, "user");
    setMessages((prev) =>
      prev.map((m) => (m.id === messageId ? { ...m, reactions: updatedReactions } : m))
    );
  };

  const handleUploadFile = async (file: File): Promise<string | null> => {
    return new Promise((resolve) => {
      const reader = new FileReader();
      reader.onload = async () => {
        try {
          const result = reader.result as string;
          const base64 = result.includes(",") ? result.split(",")[1] : result;
          const rec = await api.uploadAttachment(
            file.name,
            file.type || "application/octet-stream",
            base64
          );
          resolve(rec.file_path);
        } catch (e) {
          console.error("Attachment upload error:", e);
          resolve(null);
        }
      };
      reader.onerror = () => resolve(null);
      reader.readAsDataURL(file);
    });
  };

  const handleToggleMute = async () => {
    const next = !isMuted;
    setIsMuted(next);
    await api.setAudioMuteState(next);
  };

  const handleSetAgencyState = async (stateName: string) => {
    setAgencyState(stateName);
    await api.setAgencyState(stateName);
  };

  const handleTaskStepToggle = async (taskId: string, stepIndex: number, completed: boolean) => {
    await api.updateTaskStep(taskId, stepIndex, completed);
    const updated = await api.getTasks();
    setTasks(updated);
  };

  const handleCreateTask = async (title: string, assignee: string, steps: string[]) => {
    await api.createTask(title, assignee, steps);
    const updated = await api.getTasks();
    setTasks(updated);
  };

  const handlePlayVoice = async (text: string, agentCodename: string) => {
    if (isMuted) {
      return;
    }
    try {
      const audioDataUrl = await api.synthesizeVoice(text, agentCodename);
      if (audioDataUrl && !isMuted) {
        const audio = new Audio(audioDataUrl);
        audio.play().catch(console.error);
      }
    } catch (err) {
      console.error("Voice synthesis failed:", err);
    }
  };

  const activeChannel = channels.find((c) => c.id === activeChannelId) || {
    id: 0,
    name: "agency-hq",
    slug: "agency-hq",
    is_group: true,
    description: "Primary Agency Group Chat",
    pinned: true,
    created_at: new Date().toISOString(),
  };

  const activeAgent = agents.find((a) => a.codename === activeChannel.agent_codename);

  const fontClass =
    currentFont === "atkinson"
      ? "font-atkinson"
      : currentFont === "dyslexic"
      ? "font-dyslexic"
      : "font-segoe";

  return (
    <HiveErrorBoundary>
      <div
        className={`h-screen w-screen flex bg-hive-base text-hive-brightText overflow-hidden select-none ${fontClass}`}
        style={{ fontSize: `${fontSize}px` }}
      >
        {!isFocusMode && (
          <NavRail
            currentTab={currentTab}
            onSelectTab={(tab) => {
              if (tab === "settings") {
                setIsSettingsOpen(true);
              } else {
                setCurrentTab("chats");
              }
            }}
            agencyState={agencyState}
            isMuted={isMuted}
            onToggleMute={handleToggleMute}
          />
        )}

        <div className="flex-1 flex overflow-hidden">
          {!isFocusMode && (
            <ConversationList
              channels={channels}
              agents={agents}
              tasks={tasks}
              activeChannelId={activeChannelId}
              onSelectChannel={setActiveChannelId}
              onOpenSearch={() => setIsSearchOpen(true)}
              onTaskStepToggle={handleTaskStepToggle}
              onCreateTask={handleCreateTask}
              latestMessages={latestMessages}
            />
          )}

          <main className="flex-1 flex flex-col bg-hive-base overflow-hidden">
            <ChatHeader
              channel={activeChannel}
              agent={activeAgent}
              isMuted={isMuted}
              onToggleMute={handleToggleMute}
              isFocusMode={isFocusMode}
              onToggleFocusMode={() => setIsFocusMode(!isFocusMode)}
              onOpenSearch={() => setIsSearchOpen(true)}
            />

            <MessageStream
              messages={messages}
              agents={agents}
              tasks={tasks}
              channels={channels}
              typingIndicator={typingIndicator}
              onReply={(msg) =>
                setReplyingTo({
                  id: msg.id,
                  sender: msg.sender,
                  snippet: msg.content.slice(0, 60),
                })
              }
              onReact={handleReactMessage}
              onEdit={handleEditMessage}
              onDelete={handleDeleteMessage}
              onForward={handleForwardMessage}
              onPlayVoice={handlePlayVoice}
              onTaskStepToggle={handleTaskStepToggle}
              isMuted={isMuted}
            />

            <MessageComposer
              onSendMessage={handleSendMessage}
              onUploadFile={handleUploadFile}
              replyingTo={replyingTo}
              onCancelReply={() => setReplyingTo(null)}
              isMuted={isMuted}
            />
          </main>
        </div>

        <SearchModal
          isOpen={isSearchOpen}
          onClose={() => setIsSearchOpen(false)}
          onNavigateChannel={setActiveChannelId}
        />

        <SettingsModal
          isOpen={isSettingsOpen}
          onClose={() => setIsSettingsOpen(false)}
          constitution={constitution}
          bridgeStatus={bridgeStatus}
          agencyState={agencyState}
          onSetAgencyState={handleSetAgencyState}
          currentFont={currentFont}
          onSelectFont={handleSelectFont}
          fontSize={fontSize}
          onSetFontSize={handleSetFontSize}
          isMuted={isMuted}
        />
      </div>
    </HiveErrorBoundary>
  );
};
