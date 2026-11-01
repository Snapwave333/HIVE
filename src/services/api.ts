import {
  Channel,
  Agent,
  Message,
  TaskChunk,
  MemoryItem,
  RelationshipEdge,
  TempSpecialist,
  AttachmentRecord,
  UiDeployment,
  AuditEntry,
  BridgeStatus,
  Constitution,
} from "../types";
import { invoke } from "@tauri-apps/api/core";

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(cmd, args);
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
  forwardMessage: (messageId: string, targetChannelId: number) =>
    tauriInvoke<Message>("forward_message", {
      message_id: messageId,
      target_channel_id: targetChannelId,
    }),
  toggleReaction: (messageId: string, emoji: string, userId: string) =>
    tauriInvoke<string>("toggle_reaction", { message_id: messageId, emoji, user_id: userId }),
  uploadAttachment: (filename: string, mimeType: string, base64Data: string) =>
    tauriInvoke<AttachmentRecord>("upload_attachment", {
      filename,
      mime_type: mimeType,
      data_base64: base64Data,
    }),
  getAttachment: (hash: string) => tauriInvoke<AttachmentRecord>("get_attachment", { hash }),
  markChannelRead: (channelId: number) => tauriInvoke<void>("mark_channel_read", { channel_id: channelId }),
  getUnreadCounts: () => tauriInvoke<Record<string, number>>("get_unread_counts"),
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
  updateMemory: (memoryId: string, content: string, isSpeculation = false) =>
    tauriInvoke<void>("update_memory", { id: memoryId, content, is_speculation: isSpeculation }),
  deleteMemory: (memoryId: string) =>
    tauriInvoke<void>("delete_memory", { id: memoryId }),
  getRelationships: () => tauriInvoke<RelationshipEdge[]>("get_relationships"),
  getTempSpecialists: () => tauriInvoke<TempSpecialist[]>("get_temp_specialists"),
  spawnTempSpecialist: (
    name: string,
    role: string,
    sponsor: string,
    ttlMinutes: number
  ) =>
    tauriInvoke<TempSpecialist>("spawn_temp_specialist", {
      name,
      role,
      sponsor,
      ttl_minutes: ttlMinutes,
    }),
  getUiDeployments: () => tauriInvoke<UiDeployment[]>("get_ui_deployments"),
  deployUiVersion: (
    version: string,
    authorAgent: string,
    summary: string,
    activate = true
  ) =>
    tauriInvoke<UiDeployment>("deploy_ui_version", {
      version,
      author_agent: authorAgent,
      summary,
      activate,
    }),
  rollbackUiVersion: (reason: string) =>
    tauriInvoke<UiDeployment>("rollback_ui_version", { reason }),
  openPreviewWindow: (version: string) =>
    tauriInvoke<void>("open_preview_window", { version }),
  getAuditLog: (limit?: number) =>
    tauriInvoke<AuditEntry[]>("get_audit_log", { limit }),
  createDbBackup: () => tauriInvoke<string>("create_db_backup"),
  hideMainWindow: () => tauriInvoke<void>("hide_main_window"),
  stopAgencyAndQuit: () => tauriInvoke<void>("stop_agency_and_quit"),
  searchUnified: (query: string) =>
    tauriInvoke<{ query: string; messages: unknown[]; tasks: unknown[]; memories: unknown[] }>(
      "search_unified",
      { query }
    ),
  getBridgeStatus: () => tauriInvoke<BridgeStatus>("get_bridge_status"),
  setNimApiKey: (key: string) => tauriInvoke<string>("set_nim_api_key", { key }),
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
  getSetting: (key: string) => tauriInvoke<string | null>("get_setting", { key }),
  setSetting: (key: string, value: string) => tauriInvoke<void>("set_setting", { key, value }),
};
