import React, { useState, useRef, useEffect } from "react";
import { M3Icon } from "./M3Icon";
import { AttachmentRecord } from "../types";

interface MessageComposerProps {
  onSendMessage: (content: string, replyToId?: string, attachmentsJson?: string) => void;
  onUploadFile?: (file: File) => Promise<AttachmentRecord | null>;
  replyingTo?: { id: string; sender: string; snippet: string } | null;
  onCancelReply?: () => void;
  isMuted: boolean;
  onToggleMute: () => void;
  channelLabel: string;
}

const MAX_ATTACHMENTS = 6;

const KAOMOJIS = [
  "(ง'̀-'́)ง",
  "(•̀ᴗ•́)و",
  "(⌐■_■)",
  "(⊙_⊙)",
  "(•_•)",
  "ʕ•ᴥ•ʔ",
  "(ノಠ益ಠ)ノ",
  "¯\\_(ツ)_/¯",
  "(づ｡◕‿‿◕｡)づ",
  "٩(◕‿◕｡)۶",
];

const EMOJIS = ["👍", "❤️", "🔥", "🚀", "⬡", "⚡", "💡", "🎯", "🛡️", "👀", "✅", "⚠️"];

const SLASH_COMMANDS = [
  { cmd: "/help", hint: "list commands" },
  { cmd: "/task ", hint: "queue a goal as 5-minute steps" },
  { cmd: "/gog ", hint: "run the Google CLI" },
  { cmd: "/obsidian ", hint: "save or search vault notes" },
  { cmd: "/browser ", hint: "query the Chrome relay" },
  { cmd: "/debate ", hint: "council vote on a proposal" },
  { cmd: "/spawn_temp ", hint: "temporary specialist" },
  { cmd: "/deploy ", hint: "snapshot the running UI" },
  { cmd: "/rollback", hint: "previous UI snapshot" },
  { cmd: "/backup", hint: "SQLite snapshot now" },
  { cmd: "/pause", hint: "pause background work" },
  { cmd: "/resume", hint: "resume background work" },
  { cmd: "/activate", hint: "rerun the agency check-in" },
];

function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${bytes} B`;
}

export const MessageComposer: React.FC<MessageComposerProps> = ({
  onSendMessage,
  onUploadFile,
  replyingTo,
  onCancelReply,
  isMuted,
  onToggleMute,
  channelLabel,
}) => {
  const [text, setText] = useState("");
  const [showEmojis, setShowEmojis] = useState(false);
  const [showKaomoji, setShowKaomoji] = useState(false);
  const [attachments, setAttachments] = useState<AttachmentRecord[]>([]);
  const [uploading, setUploading] = useState(0);
  const [uploadError, setUploadError] = useState<string | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [slashIndex, setSlashIndex] = useState(0);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const slashMatches =
    text.startsWith("/") && !text.includes(" ") && !text.includes("\n")
      ? SLASH_COMMANDS.filter((c) => c.cmd.trim().startsWith(text.toLowerCase()) && c.cmd.trim() !== text.toLowerCase())
      : [];

  useEffect(() => {
    setSlashIndex(0);
  }, [text]);

  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 140)}px`;
    }
  }, [text]);

  useEffect(() => {
    textareaRef.current?.focus();
  }, [channelLabel, replyingTo]);

  const addFiles = async (files: File[]) => {
    if (!onUploadFile || files.length === 0) return;
    setUploadError(null);
    const room = MAX_ATTACHMENTS - attachments.length;
    const batch = files.slice(0, Math.max(0, room));
    if (batch.length < files.length) setUploadError(`Up to ${MAX_ATTACHMENTS} files per message.`);
    setUploading((n) => n + batch.length);
    for (const file of batch) {
      const rec = await onUploadFile(file);
      setUploading((n) => n - 1);
      if (rec) {
        setAttachments((prev) => (prev.some((a) => a.hash === rec.hash) ? prev : [...prev, rec]));
      } else {
        setUploadError(`Couldn't attach ${file.name}.`);
      }
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (slashMatches.length > 0) {
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        const delta = e.key === "ArrowDown" ? 1 : -1;
        setSlashIndex((i) => (i + delta + slashMatches.length) % slashMatches.length);
        return;
      }
      if (e.key === "Tab") {
        e.preventDefault();
        setText(slashMatches[slashIndex].cmd);
        return;
      }
    }
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
    if (e.key === "Escape" && replyingTo && onCancelReply) {
      onCancelReply();
    }
  };

  const handlePaste = (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const files = Array.from(e.clipboardData?.files ?? []);
    if (files.length > 0) {
      e.preventDefault();
      addFiles(files);
    }
  };

  const handleDrop = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    setIsDragging(false);
    addFiles(Array.from(e.dataTransfer?.files ?? []));
  };

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    addFiles(Array.from(e.target.files ?? []));
    e.target.value = "";
  };

  const canSend = (text.trim().length > 0 || attachments.length > 0) && uploading === 0;

  const handleSend = () => {
    if (!canSend) return;
    const attachmentsJson =
      attachments.length > 0
        ? JSON.stringify(
            attachments.map((a) => ({
              hash: a.hash,
              filename: a.filename,
              mime_type: a.mime_type,
              size_bytes: a.size_bytes,
            }))
          )
        : undefined;
    onSendMessage(text.trim(), replyingTo?.id, attachmentsJson);
    setText("");
    setAttachments([]);
    setUploadError(null);
    setShowEmojis(false);
    setShowKaomoji(false);
    if (onCancelReply) onCancelReply();
  };

  const insertText = (str: string) => {
    setText((prev) => prev + str);
    textareaRef.current?.focus();
  };

  return (
    <div
      onDragOver={(e) => {
        e.preventDefault();
        setIsDragging(true);
      }}
      onDragLeave={() => setIsDragging(false)}
      onDrop={handleDrop}
      className={`bg-hive-panel border-t px-8 py-4 relative select-none transition-colors flex-shrink-0 ${
        isDragging ? "border-hive-lime bg-hive-lime/5" : "border-hive-border"
      }`}
    >
      <div className="max-w-5xl mx-auto w-full relative">
        <input ref={fileInputRef} type="file" multiple onChange={handleFileSelect} className="hidden" />

        {replyingTo && (
          <div className="mb-2.5 px-3.5 py-2 rounded-xl bg-hive-surface border-l-2 border-hive-lime flex items-center justify-between text-xs">
            <div className="truncate">
              <span className="text-hive-lime font-semibold">Replying to @{replyingTo.sender}: </span>
              <span className="text-hive-dimText">{replyingTo.snippet}</span>
            </div>
            <button onClick={onCancelReply} className="text-hive-dimText hover:text-hive-brightText text-xs ml-2" title="Cancel reply (Esc)">
              <M3Icon name="close" size={14} />
            </button>
          </div>
        )}

        {(attachments.length > 0 || uploading > 0 || uploadError) && (
          <div className="mb-2.5 flex flex-wrap gap-2 text-xs">
            {attachments.map((a) => (
              <span
                key={a.hash}
                className="pl-2 pr-1 py-1 rounded-lg bg-hive-surface border border-hive-border flex items-center gap-1.5 max-w-[260px]"
              >
                {a.data_url ? (
                  <img src={a.data_url} alt="" className="w-6 h-6 rounded object-cover" />
                ) : (
                  <M3Icon name="description" size={15} className="text-hive-lime" />
                )}
                <span className="truncate text-hive-brightText">{a.filename}</span>
                <span className="text-hive-dimText font-mono text-[10px]">{formatSize(a.size_bytes)}</span>
                <button
                  onClick={() => setAttachments((prev) => prev.filter((x) => x.hash !== a.hash))}
                  className="text-hive-dimText hover:text-hive-brightText"
                  title="Remove"
                >
                  <M3Icon name="close" size={13} />
                </button>
              </span>
            ))}
            {uploading > 0 && (
              <span className="px-2 py-1 rounded-lg bg-hive-surface border border-hive-border text-hive-dimText">
                Uploading {uploading}…
              </span>
            )}
            {uploadError && <span className="px-2 py-1 text-red-400">{uploadError}</span>}
          </div>
        )}

        {slashMatches.length > 0 && (
          <div className="absolute bottom-16 left-10 bg-hive-panel border border-hive-border rounded-xl shadow-2xl py-1.5 z-30 w-80 text-xs">
            {slashMatches.map((c, i) => (
              <button
                key={c.cmd}
                onMouseDown={(e) => {
                  e.preventDefault();
                  setText(c.cmd);
                  textareaRef.current?.focus();
                }}
                className={`w-full text-left px-3 py-1.5 flex items-center justify-between ${
                  i === slashIndex ? "bg-hive-surface text-hive-lime" : "text-hive-brightText hover:bg-hive-surface"
                }`}
              >
                <span className="font-mono">{c.cmd.trim()}</span>
                <span className="text-hive-dimText">{c.hint}</span>
              </button>
            ))}
            <div className="px-3 pt-1 text-[10px] text-hive-dimText">↑↓ to pick · Tab to complete</div>
          </div>
        )}

        {showKaomoji && (
          <div className="absolute bottom-16 right-24 bg-hive-panel border border-hive-border rounded-xl shadow-2xl p-2.5 z-30 flex flex-wrap gap-1.5 max-w-xs">
            {KAOMOJIS.map((k) => (
              <button
                key={k}
                onClick={() => {
                  insertText(` ${k} `);
                  setShowKaomoji(false);
                }}
                className="px-2.5 py-1 bg-hive-surface hover:border-hive-lime/50 border border-hive-border text-xs rounded-lg font-mono text-hive-brightText transition-colors"
              >
                {k}
              </button>
            ))}
          </div>
        )}

        {showEmojis && (
          <div className="absolute bottom-16 right-16 bg-hive-panel border border-hive-border rounded-xl shadow-2xl p-2.5 z-30 flex flex-wrap gap-1.5 max-w-xs">
            {EMOJIS.map((e) => (
              <button
                key={e}
                onClick={() => {
                  insertText(e);
                  setShowEmojis(false);
                }}
                className="w-8 h-8 flex items-center justify-center hover:bg-hive-surface rounded-lg text-sm transition-colors"
              >
                {e}
              </button>
            ))}
          </div>
        )}

        <div className="flex items-end space-x-2.5 bg-hive-surface rounded-xl px-3.5 py-2 border border-hive-border focus-within:border-hive-lime/50 transition-all">
          <button
            onClick={() => fileInputRef.current?.click()}
            className="w-8 h-8 rounded-lg hover:bg-hive-base text-hive-dimText hover:text-hive-brightText transition-colors flex items-center justify-center flex-shrink-0"
            title="Attach files (drag & drop or paste also work). Agents can read text files."
          >
            <M3Icon name="attach_file" size={18} />
          </button>

          <textarea
            ref={textareaRef}
            value={text}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={handleKeyDown}
            onPaste={handlePaste}
            placeholder={`Message ${channelLabel}… (/ for commands, @NAME to tag)`}
            rows={1}
            className="flex-1 bg-transparent border-none outline-none resize-none text-[13px] text-hive-brightText placeholder-hive-dimText leading-relaxed max-h-36 py-1"
          />

          <button
            onClick={() => {
              setShowKaomoji(!showKaomoji);
              setShowEmojis(false);
            }}
            className="px-2 h-8 rounded-lg hover:bg-hive-base text-[11px] font-mono text-hive-dimText hover:text-hive-lime transition-colors flex items-center justify-center flex-shrink-0"
            title="Kaomoji"
          >
            (ง'̀-'́)ง
          </button>

          <button
            onClick={() => {
              setShowEmojis(!showEmojis);
              setShowKaomoji(false);
            }}
            className="w-8 h-8 rounded-lg hover:bg-hive-base text-hive-dimText hover:text-hive-lime transition-colors flex items-center justify-center flex-shrink-0"
            title="Emoji"
          >
            <M3Icon name="mood" size={18} />
          </button>

          <button
            onClick={onToggleMute}
            className={`w-8 h-8 rounded-lg transition-colors flex items-center justify-center flex-shrink-0 ${
              isMuted ? "text-hive-dimText hover:text-hive-brightText" : "text-hive-lime hover:bg-hive-lime/10"
            }`}
            title={isMuted ? "Speech is muted. Click to allow playback on click." : "Speech playback allowed. Click to mute."}
          >
            <M3Icon name={isMuted ? "volume_off" : "volume_up"} filled={!isMuted} size={18} />
          </button>

          <button
            onClick={handleSend}
            disabled={!canSend}
            className={`w-8 h-8 rounded-lg transition-all flex items-center justify-center flex-shrink-0 ${
              canSend ? "bg-hive-lime text-hive-base hover:opacity-90 font-bold" : "text-hive-dimText/40 cursor-not-allowed"
            }`}
            title={uploading > 0 ? "Waiting for uploads…" : "Send (Enter)"}
          >
            <M3Icon name="send" filled={canSend} size={17} />
          </button>
        </div>
      </div>
    </div>
  );
};
