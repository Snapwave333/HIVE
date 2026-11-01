using System.Collections.Concurrent;
using System.Collections.Generic;

namespace HIVE.Coding
{
    public class CodingTaskQueue
    {
        private readonly CodingTaskPersistence _persistence;
        private readonly ConcurrentQueue<CodingTaskRequest> _pending = new ConcurrentQueue<CodingTaskRequest>();
        private readonly ConcurrentDictionary<string, CodingTaskResult> _results = new ConcurrentDictionary<string, CodingTaskResult>();

        public CodingTaskQueue(string persistencePath = null)
        {
            _persistence = new CodingTaskPersistence(persistencePath);
            Restore();
        }

        private void Restore()
        {
            var (pending, completed) = _persistence.Load();
            foreach (var req in pending) _pending.Enqueue(req);
            foreach (var res in completed) _results[res.TaskId] = res;
        }

        public void Enqueue(CodingTaskRequest request)
        {
            if (request == null) return;
            _pending.Enqueue(request);
            Persist();
        }

        public bool TryDequeue(out CodingTaskRequest request)
        {
            bool dequeued = _pending.TryDequeue(out request);
            if (dequeued) Persist();
            return dequeued;
        }

        public void RecordResult(CodingTaskResult result)
        {
            if (result == null) return;
            _results[result.TaskId] = result;
            Persist();
        }

        public CodingTaskResult GetResult(string taskId)
        {
            return _results.TryGetValue(taskId, out var res) ? res : null;
        }

        private void Persist()
        {
            var pendingList = new List<CodingTaskRequest>(_pending);
            var resultsList = new List<CodingTaskResult>(_results.Values);
            _persistence.Save(pendingList, resultsList);
        }
    }
}
