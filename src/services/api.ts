import { Channel, Agent, Message, TaskChunk, MemoryItem, BridgeStatus, Constitution } from "../types";

// Check if running inside Tauri webview
const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  // Browser dev fallback
  console.warn(`[Tauri Mock] ${cmd}`, args);
  return mockHandler<T>(cmd, args);
}

// Dev fallback mock data
const mockChannels: Channel[] = [
  { id: 0, name: "agency-hq", slug: "agency-hq", is_group: true, description: "Primary Agency Group Chat", pinned: true, created_at: new Date().toISOString() },
  { id: 1, name: "Prime", slug: "prime", is_group: false, agent_codename: "PRIME", description: "Command / Strategy", pinned: false, created_at: new Date().toISOString() },
  { id: 2, name: "Sage", slug: "sage", is_group: false, agent_codename: "SAGE", description: "Research & Analysis", pinned: false, created_at: new Date().toISOString() },
  { id: 3, name: "Forge", slug: "forge", is_group: false, agent_codename: "FORGE", description: "Engineering / Build", pinned: false, created_at: new Date().toISOString() },
  { id: 4, name: "Sentinel", slug: "sentinel", is_group: false, agent_codename: "SENTINEL", description: "Security & Audit", pinned: false, created_at: new Date().toISOString() },
  { id: 5, name: "Voice", slug: "voice", is_group: false, agent_codename: "VOICE", description: "Communications", pinned: false, created_at: new Date().toISOString() },
  { id: 6, name: "Archivist", slug: "archivist", is_group: false, agent_codename: "ARCHIVIST", description: "Knowledge & Memory", pinned: false, created_at: new Date().toISOString() },
  { id: 7, name: "Conductor", slug: "conductor", is_group: false, agent_codename: "CONDUCTOR", description: "Operations / Queue", pinned: false, created_at: new Date().toISOString() },
  { id: 8, name: "Vision", slug: "vision", is_group: false, agent_codename: "VISION", description: "Creative / Design", pinned: false, created_at: new Date().toISOString() },
  { id: 9, name: "Ledger", slug: "ledger", is_group: false, agent_codename: "LEDGER", description: "Finance / Resources", pinned: false, created_at: new Date().toISOString() },
  { id: 10, name: "Link", slug: "link", is_group: false, agent_codename: "LINK", description: "Integration / Tools", pinned: false, created_at: new Date().toISOString() },
  { id: 11, name: "Gauge", slug: "gauge", is_group: false, agent_codename: "GAUGE", description: "Quality / Verification", pinned: false, created_at: new Date().toISOString() },
  { id: 12, name: "Spark", slug: "spark", is_group: false, agent_codename: "SPARK", description: "Innovation / Experiments", pinned: false, created_at: new Date().toISOString() },
];

const mockAgents: Agent[] = [
  { codename: "PRIME", display_name: "Prime", role: "Command / Strategy", model_tier: "nvidia/nemotron-3-ultra-550b-a55b", status: "available", avatar_config: '{"primary_color":"#85D94B","secondary_color":"#42CFC4"}', updated_at: new Date().toISOString() },
  { codename: "SAGE", display_name: "Sage", role: "Research & Analysis", model_tier: "nvidia/nemotron-3.5-super-vl-120b-a12b", status: "available", avatar_config: '{"primary_color":"#42CFC4","secondary_color":"#85D94B"}', updated_at: new Date().toISOString() },
  { codename: "FORGE", display_name: "Forge", role: "Engineering / Build", model_tier: "nvidia/nemotron-3.5-lightning-30b-a3b", status: "available", avatar_config: '{"primary_color":"#85D94B","secondary_color":"#42CFC4"}', updated_at: new Date().toISOString() },
  { codename: "SENTINEL", display_name: "Sentinel", role: "Security & Audit", model_tier: "mistralai/mistral-large-4-1t-a52b", status: "available", avatar_config: '{"primary_color":"#E45B5B","secondary_color":"#B8E8D8"}', updated_at: new Date().toISOString() },
  { codename: "VOICE", display_name: "Voice", role: "Communications", model_tier: "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning", status: "available", avatar_config: '{"primary_color":"#42CFC4","secondary_color":"#B8E8D8"}', updated_at: new Date().toISOString() },
  { codename: "ARCHIVIST", display_name: "Archivist", role: "Knowledge & Memory", model_tier: "openai/gpt-oss-20b", status: "available", avatar_config: '{"primary_color":"#B8E8D8","secondary_color":"#42CFC4"}', updated_at: new Date().toISOString() },
  { codename: "CONDUCTOR", display_name: "Conductor", role: "Operations / Queue", model_tier: "minimaxai/minimax-m3.1-flash", status: "available", avatar_config: '{"primary_color":"#85D94B","secondary_color":"#B8E8D8"}', updated_at: new Date().toISOString() },
  { codename: "VISION", display_name: "Vision", role: "Creative / Design", model_tier: "nvidia/nemotron-3.5-super-vl-120b-a12b", status: "available", avatar_config: '{"primary_color":"#42CFC4","secondary_color":"#85D94B"}', updated_at: new Date().toISOString() },
  { codename: "LEDGER", display_name: "Ledger", role: "Finance / Resources", model_tier: "nvidia/nemotron-3-ultra-550b-a55b", status: "available", avatar_config: '{"primary_color":"#E5A94B","secondary_color":"#85D94B"}', updated_at: new Date().toISOString() },
  { codename: "LINK", display_name: "Link", role: "Integration / Tools", model_tier: "mistralai/mistral-large-4-1t-a52b", status: "available", avatar_config: '{"primary_color":"#42CFC4","secondary_color":"#85D94B"}', updated_at: new Date().toISOString() },
  { codename: "GAUGE", display_name: "Gauge", role: "Quality / Verification", model_tier: "nvidia/nemotron-3.5-lightning-30b-a3b", status: "available", avatar_config: '{"primary_color":"#385C32","secondary_color":"#85D94B"}', updated_at: new Date().toISOString() },
  { codename: "SPARK", display_name: "Spark", role: "Innovation / Experiments", model_tier: "nvidia/nemotron-3.5-bolt-e4b", status: "available", avatar_config: '{"primary_color":"#85D94B","secondary_color":"#42CFC4"}', updated_at: new Date().toISOString() },
];

const mockMessages: Message[] = [
  {
    id: "welcome-1",
    channel_id: 0,
    sender: "PRIME",
    sender_role: "Command / Strategy",
    content: "⬡ **H.I.V.E. Agency Online**\n\nAll 12 permanent agents are initialized and synchronized. The agency operates 24/7 across `#agency-hq` and 12 dedicated agent direct lines.\n\n* **PRIME**: Standing by for command directives and strategic planning.\n* **FORGE & GAUGE**: Engineering and verification pipelines active.\n* **SENTINEL**: Threat modeling and permission sandbox locked.\n* **ARCHIVIST**: Knowledge archive and memory graph ready.\n\nType `/help` or `@agent` to coordinate.",
    is_pinned: true,
    is_edited: false,
    is_deleted: false,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  }
];

function mockHandler<T>(cmd: string, args?: Record<string, unknown>): T {
  switch (cmd) {
    case "get_channels":
      return mockChannels as unknown as T;
    case "get_agents":
      return mockAgents as unknown as T;
    case "get_messages": {
      const channelId = (args?.channel_id as number) ?? 0;
      return mockMessages.filter((m) => m.channel_id === channelId) as unknown as T;
    }
    case "send_message": {
      const newMsg: Message = {
        id: "msg-" + Date.now(),
        channel_id: (args?.channel_id as number) ?? 0,
        sender: "user",
        sender_role: "Owner",
        content: (args?.content as string) ?? "",
        is_pinned: false,
        is_edited: false,
        is_deleted: false,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
      mockMessages.push(newMsg);
      return newMsg as unknown as T;
    }
    case "get_bridge_status":
      return {
        google_connected: true,
        google_account: "elikenny333@gmail.com",
        obsidian_available: true,
        browser_relay_available: true,
        voice_engine_available: true,
      } as unknown as T;
    case "get_audio_mute_state":
      return true as unknown as T;
    case "get_agency_state":
      return "running" as unknown as T;
    default:
      return {} as unknown as T;
  }
}

export const api = {
  getChannels: () => tauriInvoke<Channel[]>("get_channels"),
  getAgents: () => tauriInvoke<Agent[]>("get_agents"),
  getMessages: (channelId: number, limit?: number, beforeId?: string) =>
    tauriInvoke<Message[]>("get_messages", { channel_id: channelId, limit, before_id: beforeId }),
  sendMessage: (channelId: number, content: string, attachments?: string, replyToId?: string) =>
    tauriInvoke<Message>("send_message", {
      channel_id: channelId,
      content,
      attachments,
      reply_to_id: replyToId,
    }),
  editMessage: (messageId: string, content: string) =>
    tauriInvoke<void>("edit_message", { message_id: messageId, content }),
  deleteMessage: (messageId: string) =>
    tauriInvoke<void>("delete_message", { message_id: messageId }),
  toggleReaction: (messageId: string, emoji: string, userId: string) =>
    tauriInvoke<string>("toggle_reaction", { message_id: messageId, emoji, user_id: userId }),
  getTasks: (statusFilter?: string) =>
    tauriInvoke<TaskChunk[]>("get_tasks", { status_filter: statusFilter }),
  createTask: (title: string, assignee: string, steps: string[]) =>
    tauriInvoke<string>("create_task", { title, assignee, steps }),
  updateTaskStep: (taskId: string, stepIndex: number, completed: boolean, checkpoint?: string) =>
    tauriInvoke<void>("update_task_step", {
      task_id: taskId,
      step_index: stepIndex,
      completed,
      checkpoint,
    }),
  getMemories: (category?: string) =>
    tauriInvoke<MemoryItem[]>("get_memories", { category }),
  searchUnified: (query: string) =>
    tauriInvoke<{ query: string; messages: unknown[]; tasks: unknown[]; memories: unknown[] }>(
      "search_unified",
      { query }
    ),
  getBridgeStatus: () => tauriInvoke<BridgeStatus>("get_bridge_status"),
  executeGoogleQuery: (service: string, args: string[]) =>
    tauriInvoke<string>("execute_google_query", { service, args }),
  synthesizeVoice: (text: string, agentCodename: string) =>
    tauriInvoke<string>("synthesize_voice", { text, agent_codename: agentCodename }),
  getAgencyState: () => tauriInvoke<string>("get_agency_state"),
  setAgencyState: (stateName: string) =>
    tauriInvoke<void>("set_agency_state", { state_name: stateName }),
  getAudioMuteState: () => tauriInvoke<boolean>("get_audio_mute_state"),
  setAudioMuteState: (isMuted: boolean) =>
    tauriInvoke<void>("set_audio_mute_state", { is_muted: isMuted }),
  getConstitution: () => tauriInvoke<Constitution>("get_constitution"),
};
