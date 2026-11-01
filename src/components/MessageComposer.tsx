import React, { useState, useRef, useEffect } from "react";
import { M3Icon } from "./M3Icon";

interface MessageComposerProps {
  onSendMessage: (content: string, replyToId?: string, attachmentsJson?: string) => void;
  onUploadFile?: (file: File) => Promise<string | null>;
  replyingTo?: { id: string; sender: string; snippet: string } | null;
  onCancelReply?: () => void;
  isMuted: boolean;
}

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

const GIF_PRESETS = [
  { label: "Ship It 🚀", text: "![Ship It GIF](https://media.giphy.com/media/143v0Z4767T15e/giphy.gif)" },
  { label: "Mind Blown 🤯", text: "![Mind Blown GIF](https://media.giphy.com/media/xT0xeJpnrWC4XWblEk/giphy.gif)" },
  { label: "High Five 🙌", text: "![High Five GIF](https://media.giphy.com/media/3oEjHV0z8S7WM4MwnK/giphy.gif)" },
  { label: "Deploying ⚡", text: "![Deploying GIF](https://media.giphy.com/media/LmNwrBhejkK9EFP504/giphy.gif)" },
];

export const MessageComposer: React.FC<MessageComposerProps> = ({
  onSendMessage,
  onUploadFile,
  replyingTo,
  onCancelReply,
  isMuted,
}) => {
  const [text, setText] = useState("");
  const [showEmojis, setShowEmojis] = useState(false);
  const [showKaomoji, setShowKaomoji] = useState(false);
  const [showGifs, setShowGifs] = useState(false);
  const [attachedFileInfo, setAttachedFileInfo] = useState<string | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 140)}px`;
    }
  }, [text]);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handlePaste = async (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const items = e.clipboardData?.items;
    if (!items || !onUploadFile) return;
    for (let i = 0; i < items.length; i++) {
      const item = items[i];
      if (item.kind === "file") {
        const file = item.getAsFile();
        if (file) {
          e.preventDefault();
          const storedPath = await onUploadFile(file);
          if (storedPath) {
            setAttachedFileInfo(`${file.name} (${storedPath})`);
          }
        }
      }
    }
  };

  const handleDrop = async (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    setIsDragging(false);
    const files = e.dataTransfer?.files;
    if (files && files.length > 0 && onUploadFile) {
      const file = files[0];
      const storedPath = await onUploadFile(file);
      if (storedPath) {
        setAttachedFileInfo(`${file.name} (${storedPath})`);
      }
    }
  };

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (files && files.length > 0 && onUploadFile) {
      const file = files[0];
      const storedPath = await onUploadFile(file);
      if (storedPath) {
        setAttachedFileInfo(`${file.name} (${storedPath})`);
      }
    }
  };

  const handleSend = () => {
    if (!text.trim() && !attachedFileInfo) return;
    const msgContent = text.trim() || `Attached file: ${attachedFileInfo}`;
    onSendMessage(msgContent, replyingTo?.id, attachedFileInfo || undefined);
    setText("");
    setAttachedFileInfo(null);
    setShowEmojis(false);
    setShowKaomoji(false);
    setShowGifs(false);
    if (onCancelReply) onCancelReply();
  };

  const insertText = (str: string) => {
    setText((prev) => prev + str);
    if (textareaRef.current) {
      textareaRef.current.focus();
    }
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
      {/* Hidden File Input */}
      <input
        ref={fileInputRef}
        type="file"
        onChange={handleFileSelect}
        className="hidden"
      />

      {/* Replying banner */}
      {replyingTo && (
        <div className="mb-2.5 px-3.5 py-2 rounded-xl bg-hive-surface border-l-2 border-hive-lime flex items-center justify-between text-xs">
          <div className="truncate">
            <span className="text-hive-lime font-semibold">Replying to @{replyingTo.sender}: </span>
            <span className="text-hive-dimText">{replyingTo.snippet}</span>
          </div>
          <button
            onClick={onCancelReply}
            className="text-hive-dimText hover:text-hive-brightText text-xs ml-2"
          >
            <M3Icon name="close" size={14} />
          </button>
        </div>
      )}

      {/* Attached File Preview */}
      {attachedFileInfo && (
        <div className="mb-2.5 px-3.5 py-2 rounded-xl bg-hive-surface border-l-2 border-hive-lime flex items-center justify-between text-xs font-mono">
          <span className="text-hive-lime truncate flex items-center gap-1.5">
            <M3Icon name="attach_file" size={14} /> {attachedFileInfo}
          </span>
          <button
            onClick={() => setAttachedFileInfo(null)}
            className="text-hive-dimText hover:text-hive-brightText ml-2"
          >
            <M3Icon name="close" size={14} />
          </button>
        </div>
      )}

      {/* Kaomoji Drawer */}
      {showKaomoji && (
        <div className="absolute bottom-16 left-0 bg-hive-panel border border-hive-border rounded-xl shadow-2xl p-2.5 z-30 flex flex-wrap gap-1.5 max-w-xs">
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

      {/* Emoji Picker */}
      {showEmojis && (
        <div className="absolute bottom-16 left-12 bg-hive-panel border border-hive-border rounded-xl shadow-2xl p-2.5 z-30 flex flex-wrap gap-1.5 max-w-xs">
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

      {/* GIF Picker */}
      {showGifs && (
        <div className="absolute bottom-16 right-16 bg-hive-panel border border-hive-border rounded-xl shadow-2xl p-2.5 z-30 flex flex-col gap-1.5 w-48">
          <div className="text-[10px] text-hive-dimText uppercase font-semibold px-1">
            Quick GIFs
          </div>
          {GIF_PRESETS.map((g) => (
            <button
              key={g.label}
              onClick={() => {
                onSendMessage(g.text, replyingTo?.id);
                setShowGifs(false);
              }}
              className="px-2.5 py-1.5 text-left bg-hive-surface hover:border-hive-lime/50 border border-hive-border text-xs rounded-lg text-hive-brightText transition-colors"
            >
              {g.label}
            </button>
          ))}
        </div>
      )}

      {/* Composer Input Area */}
      <div className="flex items-end space-x-2.5 bg-hive-surface rounded-xl px-3.5 py-2 border border-hive-border focus-within:border-hive-lime/50 transition-all">
        {/* Attachment Button */}
        <button
          onClick={() => fileInputRef.current?.click()}
          className="w-8 h-8 rounded-lg hover:bg-hive-base text-hive-dimText hover:text-hive-brightText transition-colors flex items-center justify-center flex-shrink-0"
          title="Attach file (drag & drop or Ctrl+V paste supported)"
        >
          <M3Icon name="attach_file" size={18} />
        </button>

        {/* Text Area */}
        <textarea
          ref={textareaRef}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
          placeholder="Message #agency-hq, @agent, or /help (/gog, /obsidian, /browser, /task, /debate, /deploy)..."
          rows={1}
          className="flex-1 bg-transparent border-none outline-none resize-none text-[13px] text-hive-brightText placeholder-hive-dimText leading-relaxed max-h-36 py-1 font-segoe"
        />

        {/* GIF Trigger Button */}
        <button
          onClick={() => setShowGifs(!showGifs)}
          className="w-8 h-8 rounded-lg hover:bg-hive-base text-hive-dimText hover:text-hive-lime transition-colors flex items-center justify-center flex-shrink-0"
          title="GIF Picker"
        >
          <M3Icon name="gif_box" size={18} />
        </button>

        {/* Kaomoji Trigger Button */}
        <button
          onClick={() => setShowKaomoji(!showKaomoji)}
          className="px-2 h-8 rounded-lg hover:bg-hive-base text-[11px] font-mono text-hive-dimText hover:text-hive-lime transition-colors flex items-center justify-center flex-shrink-0"
          title="Kaomoji Drawer (ง'̀-'́)ง"
        >
          (ง'̀-'́)ง
        </button>

        {/* Emoji Trigger Button */}
        <button
          onClick={() => setShowEmojis(!showEmojis)}
          className="w-8 h-8 rounded-lg hover:bg-hive-base text-hive-dimText hover:text-hive-lime transition-colors flex items-center justify-center flex-shrink-0"
          title="Emoji Picker"
        >
          <M3Icon name="mood" size={18} />
        </button>

        {/* Voice Status Indicator */}
        <button
          className={`w-8 h-8 rounded-lg transition-colors flex items-center justify-center flex-shrink-0 ${
            isMuted
              ? "text-hive-dimText/60 hover:text-hive-dimText"
              : "text-hive-lime hover:bg-hive-lime/10"
          }`}
          title={isMuted ? "Audio muted by default (Strict Invariant)" : "Voice synthesis enabled"}
        >
          <M3Icon name={isMuted ? "mic_off" : "mic"} filled={!isMuted} size={18} />
        </button>

        {/* Send Button */}
        <button
          onClick={handleSend}
          disabled={!text.trim() && !attachedFileInfo}
          className={`w-8 h-8 rounded-lg transition-all flex items-center justify-center flex-shrink-0 ${
            text.trim() || attachedFileInfo
              ? "bg-hive-lime text-hive-base hover:opacity-90 font-bold"
              : "text-hive-dimText/40 cursor-not-allowed"
          }`}
          title="Send (Enter)"
        >
          <M3Icon name="send" filled={Boolean(text.trim() || attachedFileInfo)} size={17} />
        </button>
      </div>
      </div>
    </div>
  );
};
