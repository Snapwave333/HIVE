using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using HIVE.Core;
using HIVE.Persistence;

namespace HIVE.Tasks
{
    /// <summary>
    /// Manages task lifecycle: creation, assignment, state transitions, dependencies, verification.
    /// </summary>
    public class TaskManager : MonoBehaviour
    {
        public static TaskManager Instance { get; private set; }

        private Repository _repository;
        private readonly Dictionary<string, TaskState> _activeTasks = new();
        private readonly object _lock = new();

        public void Initialize(Repository repository)
        {
            Instance = this;
            _repository = repository;
            SubscribeToBus();
            LoadPendingTasks();
        }

        private void SubscribeToBus()
        {
            MessageBus.Subscribe<CreateTaskRequest>(OnCreateTask);
            MessageBus.Subscribe<AssignTaskRequest>(OnAssignTask);
            MessageBus.Subscribe<UpdateTaskStateRequest>(OnUpdateState);
            MessageBus.Subscribe<TaskHandoffRequest>(OnHandoff);
            MessageBus.Subscribe<RequestApprovalRequest>(OnRequestApproval);
            MessageBus.Subscribe<ApprovalDecisionRequest>(OnApprovalDecision);
        }

        // ========== Public API ==========

        public string CreateTask(TaskRecord task)
        {
            task.Id = task.Id ?? Guid.NewGuid().ToString();
            task.State = "Draft";
            task.CreatedAt = DateTime.UtcNow;
            task.UpdatedAt = DateTime.UtcNow;

            lock (_lock)
            {
                _activeTasks[task.Id] = new TaskState { Record = task };
                _repository.SaveTask(task);
            }

            MessageBus.Publish(new TaskCreatedMessage { Task = task });
            return task.Id;
        }

        public void AssignTask(string taskId, string assignee, string projectId = null)
        {
            lock (_lock)
            {
                if (!_activeTasks.TryGetValue(taskId, out var state))
                {
                    var record = _repository.GetTask(taskId);
                    if (record == null) throw new KeyNotFoundException($"Task {taskId} not found");
                    state = new TaskState { Record = record };
                    _activeTasks[taskId] = state;
                }

                state.Record.Assignee = assignee;
                state.Record.ProjectId = projectId;
                state.Record.State = "Queued";
                state.Record.UpdatedAt = DateTime.UtcNow;

                _repository.SaveTask(state.Record);
            }

            MessageBus.Publish(new TaskAssignedMessage
            {
                TaskId = taskId,
                Assignee = assignee,
                ProjectId = projectId,
                SourceAgent = "TASK_MANAGER"
            });
        }

        public void UpdateState(string taskId, string newState, string actor = null)
        {
            var validTransitions = new Dictionary<string, string[]>
            {
                ["Draft"] = new[] { "Ready", "Cancelled" },
                ["Ready"] = new[] { "Queued", "Cancelled" },
                ["Queued"] = new[] { "InProgress", "Blocked", "Cancelled" },
                ["InProgress"] = new[] { "Waiting", "Blocked", "AwaitingApproval", "Review", "Completed", "Failed" },
                ["Waiting"] = new[] { "InProgress", "Blocked", "Cancelled" },
                ["Blocked"] = new[] { "InProgress", "Cancelled" },
                ["AwaitingApproval"] = new[] { "InProgress", "Blocked", "Failed" },
                ["Review"] = new[] { "Completed", "Verified", "InProgress", "Failed" },
                ["Completed"] = new[] { "Verified", "InProgress" },
                ["Verified"] = new[] { }, // Terminal
                ["Failed"] = new[] { "InProgress", "Cancelled" },
                ["Cancelled"] = new[] { } // Terminal
            };

            lock (_lock)
            {
                if (!_activeTasks.TryGetValue(taskId, out var state))
                {
                    var record = _repository.GetTask(taskId);
                    if (record == null) throw new KeyNotFoundException($"Task {taskId} not found");
                    state = new TaskState { Record = record };
                    _activeTasks[taskId] = state;
                }

                var current = state.Record.State;
                if (!validTransitions.TryGetValue(current, out var allowed) || !allowed.Contains(newState))
                    throw new InvalidOperationException($"Invalid state transition: {current} -> {newState}");

                state.Record.State = newState;
                state.Record.UpdatedAt = DateTime.UtcNow;

                if (newState == "Completed") state.Record.CompletedAt = DateTime.UtcNow;
                if (newState == "Verified") state.Record.VerifiedAt = DateTime.UtcNow;

                _repository.SaveTask(state.Record);
            }

            MessageBus.Publish(new TaskStateChangedMessage
            {
                TaskId = taskId,
                OldState = _activeTasks[taskId].Record.State,
                NewState = newState,
                Actor = actor
            });

            // Check dependent tasks
            CheckDependents(taskId);
        }

        public void AddDeliverable(string taskId, string deliverablePath)
        {
            lock (_lock)
            {
                if (_activeTasks.TryGetValue(taskId, out var state))
                {
                    state.Record.Deliverables ??= new List<string>();
                    if (!state.Record.Deliverables.Contains(deliverablePath))
                    {
                        state.Record.Deliverables.Add(deliverablePath);
                        _repository.SaveTask(state.Record);
                    }
                }
            }
        }

        public TaskRecord GetTask(string taskId)
        {
            lock (_lock)
            {
                if (_activeTasks.TryGetValue(taskId, out var state))
                    return state.Record;
            }
            return _repository.GetTask(taskId);
        }

        public List<TaskRecord> GetTasks(string assignee = null, string state = null, string projectId = null)
        {
            return _repository.GetTasks(assignee, state, projectId);
        }

        // ========== Dependency Management ==========

        public void AddDependency(string taskId, string dependsOnTaskId)
        {
            lock (_lock)
            {
                if (!_activeTasks.TryGetValue(taskId, out var state))
                {
                    var record = _repository.GetTask(taskId);
                    if (record == null) throw new KeyNotFoundException($"Task {taskId} not found");
                    state = new TaskState { Record = record };
                    _activeTasks[taskId] = state;
                }

                state.Record.Dependencies ??= new List<string>();
                if (!state.Record.Dependencies.Contains(dependsOnTaskId))
                {
                    state.Record.Dependencies.Add(dependsOnTaskId);
                    _repository.SaveTask(state.Record);
                }
            }
        }

        private void CheckDependents(string completedTaskId)
        {
            var dependents = _repository.GetTasks()
                .Where(t => t.Dependencies?.Contains(completedTaskId) == true && t.State == "Waiting")
                .ToList();

            foreach (var dep in dependents)
            {
                var allMet = dep.Dependencies.All(d =>
                {
                    var dTask = GetTask(d);
                    return dTask?.State == "Verified" || dTask?.State == "Completed";
                });

                if (allMet)
                    UpdateState(dep.Id, "Ready", "TASK_MANAGER");
            }
        }

        // ========== Bus Handlers ==========

        private void OnCreateTask(CreateTaskRequest req)
        {
            var task = new TaskRecord
            {
                Title = req.Title,
                Description = req.Description,
                ProjectId = req.ProjectId,
                Priority = req.Priority,
                AcceptanceCriteria = req.AcceptanceCriteria
            };
            var id = CreateTask(task);

            var resp = new CreateTaskResponse
            {
                CorrelationId = req.CorrelationId,
                Success = true,
                TaskId = id
            };
            MessageBus.Publish(resp);
        }

        private void OnAssignTask(AssignTaskRequest req)
        {
            try
            {
                AssignTask(req.TaskId, req.Assignee, req.ProjectId);
                MessageBus.Publish(new AssignTaskResponse { CorrelationId = req.CorrelationId, Success = true });
            }
            catch (Exception e)
            {
                MessageBus.Publish(new AssignTaskResponse { CorrelationId = req.CorrelationId, Success = false, Error = e.Message });
            }
        }

        private void OnUpdateState(UpdateTaskStateRequest req)
        {
            try
            {
                UpdateState(req.TaskId, req.NewState, req.Actor);
                MessageBus.Publish(new UpdateTaskStateResponse { CorrelationId = req.CorrelationId, Success = true });
            }
            catch (Exception e)
            {
                MessageBus.Publish(new UpdateTaskStateResponse { CorrelationId = req.CorrelationId, Success = false, Error = e.Message });
            }
        }

        private void OnHandoff(TaskHandoffRequest req)
        {
            var task = GetTask(req.TaskId);
            if (task == null) return;

            var fromAgent = task.Assignee;
            task.Assignee = req.ToAgent;
            task.State = "Queued";
            task.UpdatedAt = DateTime.UtcNow;
            _repository.SaveTask(task);

            MessageBus.Publish(new TaskHandoffMessage
            {
                TaskId = req.TaskId,
                FromAgent = fromAgent,
                ToAgent = req.ToAgent,
                Context = req.Context
            });
        }

        private void OnRequestApproval(RequestApprovalRequest req)
        {
            UpdateState(req.TaskId, "AwaitingApproval", req.Requester);

            MessageBus.Publish(new ApprovalRequestedMessage
            {
                TaskId = req.TaskId,
                Requester = req.Requester,
                Action = req.Action,
                Details = req.Details,
                DataAffected = req.DataAffected
            });
        }

        private void OnApprovalDecision(ApprovalDecisionRequest req)
        {
            if (req.Approved)
                UpdateState(req.TaskId, "InProgress", "APPROVAL_GATE");
            else
                UpdateState(req.TaskId, "Blocked", "APPROVAL_GATE");

            MessageBus.Publish(new ApprovalDecisionMessage
            {
                TaskId = req.TaskId,
                Approved = req.Approved,
                Decider = req.Decider,
                Reason = req.Reason
            });
        }

        private void LoadPendingTasks()
        {
            var tasks = _repository.GetTasks(state: "Queued");
            foreach (var t in tasks)
                _activeTasks[t.Id] = new TaskState { Record = t };
        }

        private class TaskState
        {
            public TaskRecord Record { get; set; }
        }
    }

    // ========== Message Types ==========

    public class CreateTaskRequest : MessageBus.IRequest<CreateTaskResponse>
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string CorrelationId { get; set; }
        public string Title { get; set; }
        public string Description { get; set; }
        public string ProjectId { get; set; }
        public int Priority { get; set; }
        public string AcceptanceCriteria { get; set; }
    }

    public class CreateTaskResponse : MessageBus.IResponse
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string CorrelationId { get; set; }
        public bool Success { get; set; }
        public string Error { get; set; }
        public string TaskId { get; set; }
    }

    public class AssignTaskRequest : MessageBus.IRequest<AssignTaskResponse>
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string CorrelationId { get; set; }
        public string TaskId { get; set; }
        public string Assignee { get; set; }
        public string ProjectId { get; set; }
    }

    public class AssignTaskResponse : MessageBus.IResponse
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string CorrelationId { get; set; }
        public bool Success { get; set; }
        public string Error { get; set; }
    }

    public class UpdateTaskStateRequest : MessageBus.IRequest<UpdateTaskStateResponse>
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string CorrelationId { get; set; }
        public string TaskId { get; set; }
        public string NewState { get; set; }
        public string Actor { get; set; }
    }

    public class UpdateTaskStateResponse : MessageBus.IResponse
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string CorrelationId { get; set; }
        public bool Success { get; set; }
        public string Error { get; set; }
    }

    public class TaskHandoffRequest : MessageBus.IMessage
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string TaskId { get; set; }
        public string ToAgent { get; set; }
        public Dictionary<string, object> Context { get; set; }
    }

    public class RequestApprovalRequest : MessageBus.IMessage
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string TaskId { get; set; }
        public string Requester { get; set; }
        public string Action { get; set; }
        public string Details { get; set; }
        public string DataAffected { get; set; }
    }

    public class ApprovalDecisionRequest : MessageBus.IMessage
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string TaskId { get; set; }
        public bool Approved { get; set; }
        public string Decider { get; set; }
        public string Reason { get; set; }
    }

    // Events
    public class TaskCreatedMessage : MessageBus.IMessage
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public TaskRecord Task { get; set; }
    }

    public class TaskStateChangedMessage : MessageBus.IMessage
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string TaskId { get; set; }
        public string OldState { get; set; }
        public string NewState { get; set; }
        public string Actor { get; set; }
    }

    public class TaskHandoffMessage : MessageBus.IMessage
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string TaskId { get; set; }
        public string FromAgent { get; set; }
        public string ToAgent { get; set; }
        public Dictionary<string, object> Context { get; set; }
    }

    public class ApprovalRequestedMessage : MessageBus.IMessage
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string TaskId { get; set; }
        public string Requester { get; set; }
        public string Action { get; set; }
        public string Details { get; set; }
        public string DataAffected { get; set; }
    }

    public class ApprovalDecisionMessage : MessageBus.IMessage
    {
        public string MessageId { get; set; }
        public DateTime Timestamp { get; set; }
        public string SourceAgent { get; set; }
        public string TargetAgent { get; set; }
        public string TaskId { get; set; }
        public bool Approved { get; set; }
        public string Decider { get; set; }
        public string Reason { get; set; }
    }
}