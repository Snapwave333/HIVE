import React, { useState, useEffect, useCallback } from "react";
import { Channel, Agent, Message, TaskChunk, BridgeStatus, Constitution } from "./types";
import { api } from "./services/api";
import { NavRail } from "./components/NavRail";
import { ConversationList } from "./components/ConversationList";
import { ChatHeader } from "./components/ChatHeader";
import { MessageStream } from "./components/MessageStream";
import { MessageComposer } from "./components/MessageComposer";
import { SettingsModal } from "./components/SettingsModal";
import { SearchModal } from "./components/SearchModal";

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
  const [constitution, setConstitution] = useState<Constitution | null>(null);
  const [bridgeStatus, setBridgeStatus] = useState<BridgeStatus | null>(null);

  // Load initial data
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

      // Load initial messages for active channel
      const activeMsgs = await api.getMessages(activeChannelId);
      setMessages(activeMsgs);

      // Map latest messages for channels
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

  // Load messages when active channel changes
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
          } else if (payload.type === "task.step_completed") {
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

  const handleSendMessage = async (content: string, replyToId?: string) => {
    try {
      const msg = await api.sendMessage(activeChannelId, content, undefined, replyToId);
      setMessages((prev) => [...prev, msg]);
      setLatestMessages((prev) => ({
        ...prev,
        [activeChannelId]: msg,
      }));
    } catch (err) {
      console.error("Send message error:", err);
    }
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

  const handlePlayVoice = async (text: string, agentCodename: string) => {
    try {
      const audioPath = await api.synthesizeVoice(text, agentCodename);
      console.log("Synthesized audio at:", audioPath);
      // Play via HTML Audio if unmuted
      if (!isMuted) {
        const audio = new Audio(audioPath);
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
    <div
      className={`h-screen w-screen flex bg-hive-base text-hive-brightText overflow-hidden select-none ${fontClass}`}
      style={{ fontSize: `${fontSize}px` }}
    >
      {/* Primary Slim Nav Rail (48px) */}
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

      {/* WhatsApp 2-Pane Layout */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left: Conversation List (Hidden in Focus Mode) */}
        {!isFocusMode && (
          <ConversationList
            channels={channels}
            agents={agents}
            activeChannelId={activeChannelId}
            onSelectChannel={setActiveChannelId}
            onOpenSearch={() => setIsSearchOpen(true)}
            latestMessages={latestMessages}
          />
        )}

        {/* Right: Active Chat View */}
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
            onReply={(msg) =>
              setReplyingTo({
                id: msg.id,
                sender: msg.sender,
                snippet: msg.content.slice(0, 60),
              })
            }
            onReact={(messageId, emoji) => api.toggleReaction(messageId, emoji, "user")}
            onPlayVoice={handlePlayVoice}
            onTaskStepToggle={handleTaskStepToggle}
            isMuted={isMuted}
          />

          <MessageComposer
            onSendMessage={handleSendMessage}
            replyingTo={replyingTo}
            onCancelReply={() => setReplyingTo(null)}
            isMuted={isMuted}
          />
        </main>
      </div>

      {/* Search Modal (Ctrl+K) */}
      <SearchModal
        isOpen={isSearchOpen}
        onClose={() => setIsSearchOpen(false)}
        onNavigateChannel={setActiveChannelId}
      />

      {/* Settings & Constitution Modal */}
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
  );
};
