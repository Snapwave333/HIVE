import React, { useState, useEffect, useCallback, Component, ErrorInfo, ReactNode } from "react";
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
  { hasError: boolean; errorMsg: string }
> {
  constructor(props: { children: ReactNode }) {
    super(props);
    this.state = { hasError: false, errorMsg: "" };
  }

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, errorMsg: error.message };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("H.I.V.E. UI ErrorBoundary caught error, triggering rollback:", error, errorInfo);
    api.rollbackUiVersion(`Auto-rollback from ErrorBoundary: ${error.message}`).catch(() => {});
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="h-screen w-screen bg-hive-base text-hive-brightText flex flex-col items-center justify-center p-6 space-y-3 font-segoe">
          <div className="text-hive-lime font-bold text-lg">
            ⬡ H.I.V.E. Self-Healing Recovery Triggered
          </div>
          <p className="text-xs text-hive-dimText max-w-md text-center">
            An unhandled UI exception occurred ({this.state.errorMsg}). The Blue/Green Deployment Engine has rolled back to the last verified bundle.
          </p>
          <button
            onClick={() => this.setState({ hasError: false, errorMsg: "" })}
            className="px-4 py-1.5 rounded bg-hive-cyan text-hive-base font-semibold text-xs"
          >
            Reload Verified Shell
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

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
  const [typingIndicator, setTypingIndicator] = useState<{ agent: string; activity: string } | null>(null);
  const [constitution, setConstitution] = useState<Constitution | null>(null);
  const [bridgeStatus, setBridgeStatus] = useState<BridgeStatus | null>(null);

  const loadData = useCallback(async () => {
    try {
      const [chList, agList, tList, muteState, agState, constData, brStatus] = await Promise.all([
        api.getChannels(),
        api.getAgents(),
        api.getTasks(),
        api.getAudioMuteState(),
        api.getAgencyState(),
        api.getConstitution().catch(() => null),
        api.getBridgeStatus().catch(() => null),
      ]);

      setChannels(chList);
      setAgents(agList);
      setTasks(tList);
      setIsMuted(muteState);
      setAgencyState(agState);
      if (constData) setConstitution(constData);
      if (brStatus) setBridgeStatus(brStatus);

      const activeMsgs = await api.getMessages(activeChannelId);
      setMessages(activeMsgs);

      const latestMap: Record<number, Message> = {};
      if (activeMsgs.length > 0) {
        latestMap[activeChannelId] = activeMsgs[activeMsgs.length - 1];
      }
      setLatestMessages(latestMap);
    } catch (err) {
      console.error("Failed to load initial H.I.V.E. data:", err);
    }
  }, [activeChannelId]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  useEffect(() => {
    api.getMessages(activeChannelId).then((msgs) => {
      setMessages(msgs);
      if (msgs.length > 0) {
        setLatestMessages((prev) => ({
          ...prev,
          [activeChannelId]: msgs[msgs.length - 1],
        }));
      }
    });
  }, [activeChannelId]);

  // Listen to Tauri hive-event
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
        const { listen } = await import("@tauri-apps/api/event");
        unlisten = await listen<any>("hive-event", (event) => {
          const payload = event.payload;
          if (!payload) return;

          if (payload.type === "agent.message.created") {
            const data = payload.payload;
            const newMsg: Message = {
              id: data.message_id,
              channel_id: data.channel_id,
              sender: data.sender,
              sender_role: "Agent",
              content: data.content,
              is_pinned: false,
              is_edited: false,
              is_deleted: false,
              created_at: data.timestamp,
              updated_at: data.timestamp,
            };

            setLatestMessages((prev) => ({
              ...prev,
              [data.channel_id]: newMsg,
            }));

            if (data.channel_id === activeChannelId) {
              setMessages((prev) => [...prev, newMsg]);
            }
          } else if (payload.type === "agent.typing") {
            const data = payload.payload;
            setAgents((prev) =>
              prev.map((a) =>
                a.codename === data.agent
                  ? { ...a, status: data.is_typing ? "thinking" : "available" }
                  : a
              )
            );
            if (data.channel_id === activeChannelId) {
              if (data.is_typing) {
                setTypingIndicator({ agent: data.agent, activity: data.activity });
              } else {
                setTypingIndicator(null);
              }
            }
          } else if (payload.type === "task.step_completed" || payload.type === "task.created") {
            api.getTasks().then(setTasks);
          } else if (payload.type === "system.lifecycle") {
            setAgencyState(payload.payload.state);
          } else if (payload.type === "audio.mute_toggled") {
            setIsMuted(payload.payload.is_muted);
          }
        });
      }
    };

    setupListener();

    return () => {
      if (unlisten) unlisten();
    };
  }, [activeChannelId]);

  // Global Keyboard Shortcuts
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

  const handleSendMessage = async (
    content: string,
    replyToId?: string,
    attachmentsJson?: string
  ) => {
    try {
      const msg = await api.sendMessage(activeChannelId, content, attachmentsJson, replyToId);
      setMessages((prev) => [...prev, msg]);
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
      setMessages((prev) => [...prev, forwarded]);
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
          resolve(rec.storage_path);
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
          onSelectFont={setCurrentFont}
          fontSize={fontSize}
          onSetFontSize={setFontSize}
        />
      </div>
    </HiveErrorBoundary>
  );
};
