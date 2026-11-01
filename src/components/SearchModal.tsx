import React, { useState, useEffect } from "react";
import { api } from "../services/api";
import { Search, MessageSquare, CheckSquare, Brain, ArrowRight } from "lucide-react";

interface SearchModalProps {
  isOpen: boolean;
  onClose: () => void;
  onNavigateChannel: (channelId: number) => void;
}

interface SearchResult {
  query: string;
  messages: Array<{ id: string; channel_id: number; sender: string; snippet: string; created_at: string }>;
  tasks: Array<{ id: string; title: string; assignee: string; status: string }>;
  memories: Array<{ id: string; category: string; content: string; provenance: string }>;
}

export const SearchModal: React.FC<SearchModalProps> = ({ isOpen, onClose, onNavigateChannel }) => {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult | null>(null);
  const [loading, setLoading] = useState(false);

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

    const timer = setTimeout(async () => {
      setLoading(true);
      try {
        const res = await api.searchUnified(query);
        setResults(res as unknown as SearchResult);
      } catch (e) {
        console.error("Search error", e);
      } finally {
        setLoading(false);
      }
    }, 200);

    return () => clearTimeout(timer);
  }, [query]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/75 backdrop-blur-sm z-50 flex items-start justify-center pt-20 p-4 select-none">
      <div className="bg-hive-panel border border-hive-border rounded-xl w-full max-w-xl shadow-2xl overflow-hidden font-segoe flex flex-col max-h-[75vh]">
        {/* Search Bar Input */}
        <div className="p-3 border-b border-hive-border flex items-center space-x-2.5 bg-hive-surface">
          <Search className="w-4 h-4 text-hive-cyan flex-shrink-0" />
          <input
            autoFocus
            type="text"
            placeholder="Search across messages, 5m tasks, and indexed memory..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            className="flex-1 bg-transparent border-none outline-none text-xs text-hive-brightText placeholder-hive-dimText"
          />
          <kbd className="text-[10px] bg-hive-base px-1.5 py-0.5 rounded border border-hive-border text-hive-dimText font-mono">
            ESC to close
          </kbd>
        </div>

        {/* Results Area */}
        <div className="p-3 overflow-y-auto space-y-4 text-xs">
          {loading && (
            <div className="py-8 text-center text-hive-dimText text-xs font-mono">
              Indexing memory vectors...
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
                    <MessageSquare className="w-3 h-3 text-hive-cyan" /> Messages
                  </span>
                  <div className="space-y-1">
                    {results.messages.map((m) => (
                      <div
                        key={m.id}
                        onClick={() => {
                          onNavigateChannel(m.channel_id);
                          onClose();
                        }}
                        className="p-2 rounded bg-hive-surface hover:bg-hive-surfaceHover border border-hive-border cursor-pointer transition-colors flex items-center justify-between"
                      >
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center space-x-1.5">
                            <span className="font-semibold text-hive-cyan text-[11px]">{m.sender}</span>
                            <span className="text-[10px] text-hive-dimText font-mono">
                              Channel #{m.channel_id}
                            </span>
                          </div>
                          <p className="text-xs text-hive-brightText truncate mt-0.5">{m.snippet}</p>
                        </div>
                        <ArrowRight className="w-3.5 h-3.5 text-hive-dimText flex-shrink-0 ml-2" />
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Tasks Results */}
              {results.tasks?.length > 0 && (
                <div className="space-y-1.5">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-hive-dimText flex items-center gap-1">
                    <CheckSquare className="w-3 h-3 text-hive-lime" /> Action Tasks
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
                    <Brain className="w-3 h-3 text-purple-400" /> Agency Memories
                  </span>
                  <div className="space-y-1">
                    {results.memories.map((mem) => (
                      <div
                        key={mem.id}
                        className="p-2 rounded bg-hive-surface border border-hive-border"
                      >
                        <div className="flex items-center space-x-1.5 mb-0.5">
                          <span className="text-[10px] font-mono uppercase bg-hive-base text-purple-400 px-1 rounded">
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
