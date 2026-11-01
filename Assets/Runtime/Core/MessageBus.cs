using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using UnityEngine;

namespace HIVE.Core
{
    /// <summary>
    /// In-process event bus for agent communication.
    /// Supports pub/sub with typed messages, request/response, and persistence hooks.
    /// </summary>
    public static class MessageBus
    {
        private static readonly ConcurrentDictionary<Type, List<Delegate>> _subscribers = new();
        private static readonly ConcurrentQueue<Envelope> _persistQueue = new();
        private static readonly CancellationTokenSource _shutdownCts = new();
        private static Task _persistTask;
        private static IMessagePersister _persister;

        public static void Initialize(IMessagePersister persister = null)
        {
            _persister = persister;
            _persistTask = Task.Run(PersistLoop, _shutdownCts.Token);
        }

        public static void Shutdown()
        {
            _shutdownCts.Cancel();
            _persistTask?.Wait(TimeSpan.FromSeconds(5));
        }

        /// <summary>
        /// Subscribe to messages of type T.
        /// </summary>
        public static IDisposable Subscribe<T>(Action<T> handler) where T : class, IMessage
        {
            var type = typeof(T);
            var list = _subscribers.GetOrAdd(type, _ => new List<Delegate>());
            lock (list) list.Add(handler);
            return new Subscription<T>(handler);
        }

        /// <summary>
        /// Publish a message to all subscribers.
        /// </summary>
        public static void Publish<T>(T message) where T : class, IMessage
        {
            if (message == null) throw new ArgumentNullException(nameof(message));
            message.Timestamp = DateTime.UtcNow;
            message.MessageId = Guid.NewGuid().ToString();

            if (_subscribers.TryGetValue(typeof(T), out var list))
            {
                Delegate[] handlers;
                lock (list) handlers = list.ToArray();
                foreach (var h in handlers)
                {
                    try { ((Action<T>)h)(message); }
                    catch (Exception e) { Debug.LogError($"[MessageBus] Handler error: {e}"); }
                }
            }

            // Also notify base IMessage subscribers
            if (typeof(T) != typeof(IMessage) && _subscribers.TryGetValue(typeof(IMessage), out var baseList))
            {
                Delegate[] handlers;
                lock (baseList) handlers = baseList.ToArray();
                foreach (var h in handlers)
                {
                    try { ((Action<IMessage>)h)(message); }
                    catch (Exception e) { Debug.LogError($"[MessageBus] Base handler error: {e}"); }
                }
            }

            // Queue for persistence
            _persistQueue.Enqueue(new Envelope { Type = typeof(T).Name, Json = JsonUtility.ToJson(message) });
        }

        /// <summary>
        /// Request/response pattern with timeout.
        /// </summary>
        public static async Task<TResponse> RequestAsync<TRequest, TResponse>(TRequest request, TimeSpan? timeout = null)
            where TRequest : class, IRequest<TResponse>
            where TResponse : class, IResponse
        {
            var tcs = new TaskCompletionSource<TResponse>();
            var correlationId = Guid.NewGuid().ToString();
            request.CorrelationId = correlationId;

            using var sub = Subscribe<TResponse>(resp =>
            {
                if (resp.CorrelationId == correlationId)
                    tcs.TrySetResult(resp);
            });

            Publish(request);

            var completed = await Task.WhenAny(tcs.Task, Task.Delay(timeout ?? TimeSpan.FromSeconds(30)));
            if (completed == tcs.Task) return tcs.Task.Result;

            throw new TimeoutException($"Request {typeof(TRequest).Name} timed out");
        }

        private static async Task PersistLoop()
        {
            var batch = new List<Envelope>();
            while (!_shutdownCts.Token.IsCancellationRequested)
            {
                try
                {
                    while (_persistQueue.TryDequeue(out var env))
                        batch.Add(env);

                    if (batch.Count > 0 && _persister != null)
                    {
                        await _persister.PersistAsync(batch);
                        batch.Clear();
                    }

                    await Task.Delay(1000, _shutdownCts.Token);
                }
                catch (OperationCanceledException) { break; }
                catch (Exception e) { Debug.LogError($"[MessageBus] Persist error: {e}"); }
            }
        }

        private class Subscription<T> : IDisposable where T : class, IMessage
        {
            private readonly Action<T> _handler;
            private bool _disposed;
            public Subscription(Action<T> handler) => _handler = handler;
            public void Dispose()
            {
                if (_disposed) return;
                _disposed = true;
                if (_subscribers.TryGetValue(typeof(T), out var list))
                    lock (list) list.Remove(_handler);
            }
        }

        public class Envelope
        {
            public string Type { get; set; }
            public string Json { get; set; }
        }

        public interface IMessage
        {
            string MessageId { get; set; }
            DateTime Timestamp { get; set; }
            string SourceAgent { get; set; }
            string TargetAgent { get; set; } // null = broadcast
        }

        public interface IRequest<TResponse> : IMessage where TResponse : class, IResponse
        {
            string CorrelationId { get; set; }
        }

        public interface IResponse : IMessage
        {
            string CorrelationId { get; set; }
            bool Success { get; set; }
            string Error { get; set; }
        }

        public interface IMessagePersister
        {
            Task PersistAsync(IReadOnlyList<Envelope> messages);
        }
    }

    /// <summary>
    /// Base interface for all bus messages.
    /// </summary>
    public interface IMessage : MessageBus.IMessage { }

    /// <summary>
    /// Request message expecting a response.
    /// </summary>
    public interface IRequest<TResponse> : MessageBus.IRequest<TResponse> where TResponse : class, MessageBus.IResponse { }

    /// <summary>
    /// Response message correlated to a request.
    /// </summary>
    public interface IResponse : MessageBus.IResponse { }

    /// <summary>
    /// Persistence abstraction for message durability.
    /// </summary>
    public interface IMessagePersister : MessageBus.IMessagePersister { }

    public class Envelope : MessageBus.Envelope { }
}