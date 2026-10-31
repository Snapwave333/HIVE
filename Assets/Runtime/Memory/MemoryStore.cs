using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Threading.Tasks;
using UnityEngine;
using HIVE.Core;
using HIVE.Persistence;

namespace HIVE.Memory
{
    /// <summary>
    /// Per-agent and shared memory system with vector index for semantic search.
    /// Append-only JSONL + SQLite for metadata + FAISS/ONNX for vectors.
    /// </summary>
    public class MemoryStore : MonoBehaviour
    {
        public static MemoryStore Instance { get; private set; }

        private Repository _repository;
        private readonly Dictionary<string, List<MemoryRecord>> _cache = new();
        private readonly object _lock = new();
        private VectorIndex _vectorIndex;

        public void Initialize(Repository repository)
        {
            Instance = this;
            _repository = repository;
            _vectorIndex = new VectorIndex();
            SubscribeToBus();
        }

        private void SubscribeToBus()
        {
            MessageBus.Subscribe<StoreMemoryRequest>(OnStoreMemory);
            MessageBus.Subscribe<SearchMemoryRequest>(OnSearchMemory);
            MessageBus.Subscribe<GetMemoryRequest>(OnGetMemory);
        }

        // ========== Public API ==========

        public string Store(MemoryRecord memory)
        {
            memory.Id = memory.Id ?? Guid.NewGuid().ToString();
            memory.CreatedAt = DateTime.UtcNow;

            lock (_lock)
            {
                if (!_cache.ContainsKey(memory.AgentId ?? "shared"))
                    _cache[memory.AgentId ?? "shared"] = new List<MemoryRecord>();
                _cache[memory.AgentId ?? "shared"].Add(memory);
            }

            _repository.SaveMemory(memory);
            _vectorIndex.Add(memory);

            MessageBus.Publish(new MemoryStoredMessage { Memory = memory });
            return memory.Id;
        }

        public List<MemoryRecord> Search(string agentId = null, string type = null, string[] tags = null, string query = null, int limit = 50)
        {
            // First try vector search if query provided
            if (!string.IsNullOrEmpty(query))
            {
                var vectorResults = _vectorIndex.Search(query, limit);
                if (vectorResults.Count > 0)
                    return vectorResults;
            }

            // Fallback to SQL search
            return _repository.SearchMemories(agentId, type, tags, query, limit);
        }

        public MemoryRecord Get(string id)
        {
            // Check cache first
            lock (_lock)
            {
                foreach (var list in _cache.Values)
                {
                    var found = list.FirstOrDefault(m => m.Id == id);
                    if (found != null) return found;
                }
            }
            return _repository.SearchMemories(query: id, limit: 1).FirstOrDefault();
        }

        public void LinkMemory(string memoryId, string relatedMemoryId, string relationType)
        {
            // Store as relationship in memory context
            var mem = Get(memoryId);
            if (mem != null)
            {
                mem.Tags ??= new List<string>();
                mem.Tags.Add($"rel:{relationType}:{relatedMemoryId}");
                _repository.SaveMemory(mem);
            }
        }

        // ========== Bus Handlers ==========

        private void OnStoreMemory(StoreMemoryRequest req)
        {
            var memory = new MemoryRecord
            {
                AgentId = req.AgentId,
                Type = req.Type,
                Content = req.Content,
                Tags = req.Tags,
                Source = req.Source,
                Confidence = req.Confidence,
                ExpiresAt = req.ExpiresAt
            };
            var id = Store(memory);

            var resp = new StoreMemoryResponse
            {
                CorrelationId = req.CorrelationId,
                Success = true,
                MemoryId = id
            };
            MessageBus.Publish(resp);
        }

        private void OnSearchMemory(SearchMemoryRequest req)
        {
            var results = Search(req.AgentId, req.Type, req.Tags, req.Query, req.Limit);
            var resp = new SearchMemoryResponse
            {
                CorrelationId = req.CorrelationId,
                Success = true,
                Memories = results
            };
            MessageBus.Publish(resp);
        }

        private void OnGetMemory(GetMemoryRequest req)
        {
            var memory = Get(req.MemoryId);
            var resp = new GetMemoryResponse
            {
                CorrelationId = req.CorrelationId,
                Success = memory != null,
                Memory = memory,
                Error = memory == null ? "Not found" : null
            };
            MessageBus.Publish(resp);
        }

        // ========== Vector Index (simplified - replace with FAISS/ONNX in production) ==========

        private class VectorIndex
        {
            private readonly Dictionary<string, float[]> _embeddings = new();
            private readonly Dictionary<string, MemoryRecord> _records = new();

            public void Add(MemoryRecord memory)
            {
                // In production: use sentence transformer to generate embedding
                // For now, simple keyword-based index
                var keywords = ExtractKeywords(memory.Content);
                _records[memory.Id] = memory;
            }

            public List<MemoryRecord> Search(string query, int limit)
            {
                var queryKeywords = ExtractKeywords(query);
                var scored = _records.Values
                    .Select(m => new { Record = m, Score = CalculateScore(ExtractKeywords(m.Content), queryKeywords) })
                    .Where(x => x.Score > 0)
                    .OrderByDescending(x => x.Score)
                    .Take(limit)
                    .Select(x => x.Record)
                    .ToList();
                return scored;
            }

            private List<string> ExtractKeywords(string text)
            {
                return text.ToLowerInvariant()
                    .Split(new[] { ' ', '\n', '\t', '.', ',', ';', ':', '(', ')', '[', ']', '{', '}' }, StringSplitOptions.RemoveEmptyEntries)
                    .Where(w => w.Length > 3)
                    .Distinct()
                    .ToList();
            }

            private float CalculateScore(List<string> docKeywords, List<string> queryKeywords)
            {
                if (docKeywords.Count == 0 || queryKeywords.Count == 0) return 0;
                var matches = queryKeywords.Count(q => docKeywords.Contains(q));
                return (float)matches / queryKeywords.Count;
            }
        }
    }

    // ========== Message Types ==========

    public class StoreMemoryRequest : MessageBus.IRequest<StoreMemoryResponse>
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string CorrelationId { get; set; }
        public string AgentId { get; set; }
        public string Type { get; set; }
        public string Content { get; set; }
        public List<string> Tags { get; set; }
        public string Source { get; set; }
        public float Confidence { get; set; } = 1.0f;
        public DateTime? ExpiresAt { get; set; }
    }

    public class StoreMemoryResponse : MessageBus.IResponse
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string CorrelationId { get; set; }
        public bool Success { get; set; }
        public string Error { get; set; }
        public string MemoryId { get; set; }
    }

    public class SearchMemoryRequest : MessageBus.IRequest<SearchMemoryResponse>
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string CorrelationId { get; set; }
        public string AgentId { get; set; }
        public string Type { get; set; }
        public string[] Tags { get; set; }
        public string Query { get; set; }
        public int Limit { get; set; } = 50;
    }

    public class SearchMemoryResponse : MessageBus.IResponse
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string CorrelationId { get; set; }
        public bool Success { get; set; }
        public string Error { get; set; }
        public List<MemoryRecord> Memories { get; set; }
    }

    public class GetMemoryRequest : MessageBus.IRequest<GetMemoryResponse>
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string CorrelationId { get; set; }
        public string MemoryId { get; set; }
    }

    public class GetMemoryResponse : MessageBus.IResponse
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string CorrelationId { get; set; }
        public bool Success { get; set; }
        public string Error { get; set; }
        public MemoryRecord Memory { get; set; }
    }

    public class MemoryStoredMessage : MessageBus.IMessage
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public MemoryRecord Memory { get; set; }
    }
}