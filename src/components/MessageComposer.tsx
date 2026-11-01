import React, { useState, useRef, useEffect } from "react";
import { Send, Paperclip, Smile, Mic } from "lucide-react";

interface MessageComposerProps {
  onSendMessage: (content: string, replyToId?: string) => void;
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

export const MessageComposer: React.FC<MessageComposerProps> = ({
  onSendMessage,
  replyingTo,
  onCancelReply,
  isMuted,
}) => {
  const [text, setText] = useState("");
  const [showEmojis, setShowEmojis] = useState(false);
  const [showKaomoji, setShowKaomoji] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Auto-resize textarea
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

  const handleSend = () => {
    if (!text.trim()) return;
    onSendMessage(text.trim(), replyingTo?.id);
    setText("");
    setShowEmojis(false);
    setShowKaomoji(false);
    if (onCancelReply) onCancelReply();
  };

  const insertText = (str: string) => {
    setText((prev) => prev + str);
    if (textareaRef.current) {
      textareaRef.current.focus();
    }
  };


  return (
    <div className="bg-hive-panel border-t border-hive-border p-3 relative select-none">
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

      {/* Composer Input Area */}
      <div className="flex items-end space-x-2 bg-hive-surface rounded-lg p-2 border border-hive-border focus-within:border-hive-cyan/50 transition-colors">
        {/* Attachment Button */}
        <button
          className="p-1.5 rounded hover:bg-hive-panel text-hive-dimText hover:text-hive-brightText transition-colors flex-shrink-0"
          title="Attach file (drag & drop supported)"
        >
          <Paperclip className="w-4 h-4" />
        </button>

        {/* Text Area */}
        <textarea
          ref={textareaRef}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Type a message, @agent, or /command..."
          rows={1}
          className="flex-1 bg-transparent border-none outline-none resize-none text-xs text-hive-brightText placeholder-hive-dimText leading-relaxed max-h-36 py-1 font-segoe"
        />

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

        {/* Voice Request Toggle (Subject to Audio Sovereign Invariant) */}
        <button
          className={`p-1.5 rounded transition-colors flex-shrink-0 ${
            isMuted
              ? "text-hive-dimText/60 hover:text-hive-dimText"
              : "text-hive-lime hover:bg-hive-lime/10"
          }`}
          title={isMuted ? "Audio muted by default (Strict Invariant)" : "Voice synthesis active"}
        >
          <Mic className="w-4 h-4" />
        </button>

        {/* Send Button */}
        <button
          onClick={handleSend}
          disabled={!text.trim()}
          className={`p-1.5 rounded transition-colors flex-shrink-0 ${
            text.trim()
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
