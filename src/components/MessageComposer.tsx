import React, { useState, useRef, useEffect } from "react";
import { Send, Paperclip, Smile, Mic, Film } from "lucide-react";

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
      className={`bg-hive-panel border-t p-3 relative select-none transition-colors ${
        isDragging ? "border-hive-lime bg-hive-lime/5" : "border-hive-border"
      }`}
    >
      {/* Hidden File Input */}
      <input
        ref={fileInputRef}
        type="file"
        onChange={handleFileSelect}
        className="hidden"
      />

      {/* Replying banner */}
      {replyingTo && (
        <div className="mb-2 px-3 py-1.5 rounded bg-hive-surface border-l-2 border-hive-cyan flex items-center justify-between text-xs">
          <div className="truncate">
            <span className="text-hive-cyan font-medium">Replying to {replyingTo.sender}: </span>
            <span className="text-hive-dimText">{replyingTo.snippet}</span>
          </div>
          <button
            onClick={onCancelReply}
            className="text-hive-dimText hover:text-hive-brightText text-xs ml-2"
          >
            ✕
          </button>
        </div>
      )}

      {/* Attached File Preview */}
      {attachedFileInfo && (
        <div className="mb-2 px-3 py-1.5 rounded bg-hive-surface border-l-2 border-hive-lime flex items-center justify-between text-xs font-mono">
          <span className="text-hive-lime truncate">📎 {attachedFileInfo}</span>
          <button
            onClick={() => setAttachedFileInfo(null)}
            className="text-hive-dimText hover:text-hive-brightText ml-2"
          >
            ✕
          </button>
        </div>
      )}

      {/* Kaomoji Drawer */}
      {showKaomoji && (
        <div className="absolute bottom-16 left-3 bg-hive-panel border border-hive-border rounded-lg shadow-xl p-2 z-30 flex flex-wrap gap-1 max-w-xs">
          {KAOMOJIS.map((k) => (
            <button
              key={k}
              onClick={() => {
                insertText(` ${k} `);
                setShowKaomoji(false);
              }}
              className="px-2 py-1 bg-hive-surface hover:bg-hive-cyan/20 text-xs rounded font-mono text-hive-brightText transition-colors"
            >
              {k}
            </button>
          ))}
        </div>
      )}

      {/* Emoji Picker */}
      {showEmojis && (
        <div className="absolute bottom-16 left-12 bg-hive-panel border border-hive-border rounded-lg shadow-xl p-2 z-30 flex flex-wrap gap-1.5 max-w-xs">
          {EMOJIS.map((e) => (
            <button
              key={e}
              onClick={() => {
                insertText(e);
                setShowEmojis(false);
              }}
              className="w-7 h-7 flex items-center justify-center hover:bg-hive-surface rounded text-sm transition-colors"
            >
              {e}
            </button>
          ))}
        </div>
      )}

      {/* GIF Picker */}
      {showGifs && (
        <div className="absolute bottom-16 right-16 bg-hive-panel border border-hive-border rounded-lg shadow-xl p-2 z-30 flex flex-col gap-1 w-48">
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
              className="px-2 py-1.5 text-left bg-hive-surface hover:bg-hive-cyan/20 text-xs rounded text-hive-brightText transition-colors"
            >
              {g.label}
            </button>
          ))}
        </div>
      )}

      {/* Composer Input Area */}
      <div className="flex items-end space-x-2 bg-hive-surface rounded-lg p-2 border border-hive-border focus-within:border-hive-cyan/50 transition-colors">
        {/* Attachment Button */}
        <button
          onClick={() => fileInputRef.current?.click()}
          className="p-1.5 rounded hover:bg-hive-panel text-hive-dimText hover:text-hive-brightText transition-colors flex-shrink-0"
          title="Attach file (drag & drop or Ctrl+V paste supported)"
        >
          <Paperclip className="w-4 h-4" />
        </button>

        {/* Text Area */}
        <textarea
          ref={textareaRef}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
          placeholder="Type a message, @agent, or /help (/gog, /obsidian, /browser, /task, /debate, /deploy)..."
          rows={1}
          className="flex-1 bg-transparent border-none outline-none resize-none text-xs text-hive-brightText placeholder-hive-dimText leading-relaxed max-h-36 py-1 font-segoe"
        />

        {/* GIF Trigger Button */}
        <button
          onClick={() => setShowGifs(!showGifs)}
          className="p-1.5 rounded hover:bg-hive-panel text-hive-dimText hover:text-hive-cyan transition-colors flex-shrink-0"
          title="GIF Picker"
        >
          <Film className="w-4 h-4" />
        </button>

        {/* Kaomoji Trigger Button */}
        <button
          onClick={() => setShowKaomoji(!showKaomoji)}
          className="px-1.5 py-1 rounded hover:bg-hive-panel text-[11px] font-mono text-hive-dimText hover:text-hive-cyan transition-colors flex-shrink-0"
          title="Kaomoji Drawer (ง'̀-'́)ง"
        >
          (ง'̀-'́)ง
        </button>

        {/* Emoji Trigger Button */}
        <button
          onClick={() => setShowEmojis(!showEmojis)}
          className="p-1.5 rounded hover:bg-hive-panel text-hive-dimText hover:text-hive-cyan transition-colors flex-shrink-0"
          title="Emoji Picker"
        >
          <Smile className="w-4 h-4" />
        </button>

        {/* Voice Status Indicator */}
        <button
          className={`p-1.5 rounded transition-colors flex-shrink-0 ${
            isMuted
              ? "text-hive-dimText/60 hover:text-hive-dimText"
              : "text-hive-lime hover:bg-hive-lime/10"
          }`}
          title={isMuted ? "Audio muted by default (Strict Invariant)" : "Voice synthesis enabled"}
        >
          <Mic className="w-4 h-4" />
        </button>

        {/* Send Button */}
        <button
          onClick={handleSend}
          disabled={!text.trim() && !attachedFileInfo}
          className={`p-1.5 rounded transition-colors flex-shrink-0 ${
            text.trim() || attachedFileInfo
              ? "bg-hive-cyan text-hive-base hover:bg-hive-cyan/90 font-bold"
              : "text-hive-dimText/40 cursor-not-allowed"
          }`}
          title="Send (Enter)"
        >
          <Send className="w-4 h-4" />
        </button>
      </div>
    </div>
  );
};
