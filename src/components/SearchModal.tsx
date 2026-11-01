import React, { useState, useEffect } from "react";
import { api } from "../services/api";
import { M3Icon } from "./M3Icon";
import { pressable } from "../utils/a11y";

interface SearchModalProps {
  isOpen: boolean;
  onClose: () => void;
  onNavigateChannel: (channelId: number, messageId?: string) => void;
  channels: { id: number; name: string }[];
}

interface SearchResult {
  query: string;
  messages: Array<{ id: string; channel_id: number; sender: string; snippet: string; created_at: string }>;
  tasks: Array<{ id: string; title: string; assignee: string; status: string }>;
  memories: Array<{ id: string; category: string; content: string; provenance: string }>;
}

export const SearchModal: React.FC<SearchModalProps> = ({ isOpen, onClose, onNavigateChannel, channels }) => {
  const channelLabel = (id: number) => (id === 0 ? "#agency-hq" : channels.find((c) => c.id === id)?.name ?? `channel ${id}`);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [selected, setSelected] = useState(0);

  const openMessage = (m: { id: string; channel_id: number }) => {
    onNavigateChannel(m.channel_id, m.id);
    onClose();
  };

  useEffect(() => {
    if (!isOpen) {
      setQuery("");
      setResults(null);
      return;
    }

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onClose]);

  useEffect(() => {
    if (!query.trim()) {
      setResults(null);
      return;
    }

    let stale = false;
    const timer = setTimeout(async () => {
      setLoading(true);
      try {
        const res = await api.searchUnified(query);
        if (!stale) {
          setResults(res as unknown as SearchResult);
          setSelected(0);
        }
      } catch (e) {
        console.error("Search error", e);
      } finally {
        if (!stale) setLoading(false);
      }
    }, 200);

    return () => {
      stale = true;
      clearTimeout(timer);
    };
  }, [query]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/75 backdrop-blur-sm z-50 flex items-start justify-center pt-20 p-4 select-none">
      <div className="bg-hive-panel border border-hive-border rounded-xl w-full max-w-xl shadow-2xl overflow-hidden flex flex-col max-h-[75vh]">
        {/* Search Bar Input */}
        <div className="p-3 border-b border-hive-border flex items-center space-x-2.5 bg-hive-surface">
          <M3Icon name="search" size={18} className="text-hive-cyan flex-shrink-0" />
          <input
            autoFocus
            type="text"
            placeholder="Search across messages, 5m tasks, and indexed memory..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              const list = results?.messages ?? [];
              if (list.length === 0) return;
              if (e.key === "ArrowDown" || e.key === "ArrowUp") {
                e.preventDefault();
                const next = (selected + (e.key === "ArrowDown" ? 1 : -1) + list.length) % list.length;
                setSelected(next);
                document.getElementById(`search-hit-${list[next].id}`)?.scrollIntoView({ block: "nearest" });
              } else if (e.key === "Enter") {
                e.preventDefault();
                openMessage(list[Math.min(selected, list.length - 1)]);
              }
            }}
            aria-label="Search messages, tasks and memory"
            className="flex-1 bg-transparent border-none outline-none text-xs text-hive-brightText placeholder-hive-dimText"
          />
          <kbd className="text-[10px] bg-hive-base px-1.5 py-0.5 rounded border border-hive-border text-hive-dimText font-mono">
            ↑↓ Enter · Esc
          </kbd>
        </div>

        {/* Results Area */}
        <div className="p-3 overflow-y-auto space-y-4 text-xs">
          {loading && (
            <div className="py-8 text-center text-hive-dimText text-xs font-mono">
              Searching SQLite index...
            </div>
          )}

          {!loading && !query && (
            <div className="py-8 text-center text-hive-dimText text-xs">
              Type keywords to search across messages, tasks, and memory.
            </div>
          )}

          {!loading && results && (
            <>
              {/* Messages Results */}
              {results.messages?.length > 0 && (
                <div className="space-y-1.5">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-hive-dimText flex items-center gap-1">
                    <M3Icon name="forum" size={14} className="text-hive-cyan" /> Messages
                  </span>
                  <div className="space-y-1">
                    {results.messages.map((m, i) => (
                      <div
                        key={m.id}
                        id={`search-hit-${m.id}`}
                        aria-selected={i === selected}
                        {...pressable(() => openMessage(m), `Open message from ${m.sender === "user" ? "you" : m.sender} in ${channelLabel(m.channel_id)}`)}
                        onMouseEnter={() => setSelected(i)}
                        className={`p-2 rounded bg-hive-surface hover:bg-hive-surfaceHover border cursor-pointer transition-colors flex items-center justify-between ${
                          i === selected ? "border-hive-lime/60" : "border-hive-border"
                        }`}
                      >
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center space-x-1.5">
                            <span className="font-semibold text-hive-cyan text-[11px]">{m.sender === "user" ? "You" : `@${m.sender}`}</span>
                            <span className="text-[10px] text-hive-dimText font-mono">
                              {channelLabel(m.channel_id)}
                            </span>
                          </div>
                          <p className="text-xs text-hive-brightText truncate mt-0.5">{m.snippet}</p>
                        </div>
                        <M3Icon name="arrow_forward" size={15} className="text-hive-dimText flex-shrink-0 ml-2" />
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Tasks Results */}
              {results.tasks?.length > 0 && (
                <div className="space-y-1.5">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-hive-dimText flex items-center gap-1">
                    <M3Icon name="checklist" size={14} className="text-hive-lime" /> Action Tasks
                  </span>
                  <div className="space-y-1">
                    {results.tasks.map((t) => (
                      <div
                        key={t.id}
                        className="p-2 rounded bg-hive-surface border border-hive-border flex items-center justify-between"
                      >
                        <div>
                          <span className="font-medium text-hive-brightText">{t.title}</span>
                          <span className="ml-2 text-[10px] bg-hive-base text-hive-dimText px-1.5 py-0.2 rounded font-mono">
                            @{t.assignee}
                          </span>
                        </div>
                        <span className="text-[10px] text-hive-lime font-mono">{t.status}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Memories Results */}
              {results.memories?.length > 0 && (
                <div className="space-y-1.5">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-hive-dimText flex items-center gap-1">
                    <M3Icon name="psychology" size={14} className="text-hive-lime" /> Agency Memories
                  </span>
                  <div className="space-y-1">
                    {results.memories.map((mem) => (
                      <div
                        key={mem.id}
                        className="p-2.5 rounded-lg bg-hive-surface border border-hive-border"
                      >
                        <div className="flex items-center space-x-1.5 mb-1">
                          <span className="text-[10px] font-mono uppercase bg-hive-base text-hive-lime border border-hive-lime/30 px-1.5 py-0.5 rounded">
                            {mem.category}
                          </span>
                          <span className="text-[10px] text-hive-dimText font-mono">
                            from {mem.provenance}
                          </span>
                        </div>
                        <p className="text-xs text-hive-brightText">{mem.content}</p>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {results.messages?.length === 0 &&
                results.tasks?.length === 0 &&
                results.memories?.length === 0 && (
                  <div className="py-6 text-center text-hive-dimText text-xs">
                    No results found for &quot;{query}&quot;
                  </div>
                )}
            </>
          )}
        </div>
      </div>
    </div>
  );
};
