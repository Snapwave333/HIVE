import React from "react";
import { TaskChunk } from "../types";
import { M3Icon } from "./M3Icon";

interface TaskCardProps {
  task: TaskChunk;
  onStepToggle: (taskId: string, stepIndex: number, completed: boolean) => void;
}

export const TaskCard: React.FC<TaskCardProps> = ({ task, onStepToggle }) => {
  const completedCount = task.steps.filter((s) => s.completed).length;
  const totalCount = task.steps.length;
  const progressPercent = totalCount > 0 ? Math.round((completedCount / totalCount) * 100) : 0;

  const currentStep = task.steps[task.current_step_index] || task.steps[task.steps.length - 1];

  return (
    <div className="my-2.5 p-3.5 rounded-xl bg-hive-panel border border-hive-border hover:border-hive-lime/40 transition-all select-none">
      {/* Task Header */}
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center space-x-2 min-w-0">
          <div className="p-1.5 rounded-lg bg-hive-surface border border-hive-lime/30 text-hive-lime flex items-center justify-center flex-shrink-0">
            <M3Icon name="checklist" size={15} />
          </div>
          <span className="font-semibold text-xs text-hive-brightText truncate">
            {task.title}
          </span>
          <span className="text-[10px] bg-hive-surface px-1.5 py-0.5 rounded-md text-hive-dimText border border-hive-border font-mono flex-shrink-0">
            @{task.assignee}
          </span>
        </div>

        <div className="flex items-center space-x-1.5 flex-shrink-0">
          <span className="text-[10px] px-1.5 py-0.5 rounded-md font-mono bg-hive-lime/10 text-hive-lime border border-hive-lime/30">
            {task.status.toUpperCase()}
          </span>
          <span className="text-[10px] text-hive-dimText font-mono">{progressPercent}%</span>
        </div>
      </div>

      {/* Progress Bar */}
      <div className="w-full bg-hive-surface h-1.5 rounded-full mt-3 overflow-hidden border border-hive-border">
        <div
          className="bg-hive-lime h-full transition-all duration-300 rounded-full"
          style={{ width: `${progressPercent}%` }}
        />
      </div>

      {/* Active 5-Minute Micro-Action (ADHD Anti-Overload) */}
      {currentStep && (
        <div className="mt-3 p-2.5 rounded-lg bg-hive-surface border border-hive-border flex items-start justify-between">
          <div className="flex items-start space-x-2.5 min-w-0">
            <button
              onClick={() => onStepToggle(task.id, currentStep.index, !currentStep.completed)}
              className="mt-0.5 text-hive-lime hover:opacity-80 transition-opacity flex-shrink-0"
            >
              {currentStep.completed ? (
                <M3Icon name="check_circle" filled size={17} className="text-hive-lime" />
              ) : (
                <M3Icon name="radio_button_unchecked" size={17} />
              )}
            </button>
            <div className="min-w-0">
              <div className="flex items-center space-x-2">
                <span className="text-[10px] font-mono uppercase tracking-wide text-hive-lime">
                  Action Step {currentStep.index + 1}/{totalCount}
                </span>
                <span className="text-[10px] text-hive-dimText flex items-center gap-0.5">
                  <M3Icon name="schedule" size={12} /> ~{currentStep.estimated_minutes}m
                </span>
              </div>
              <p
                className={`text-xs mt-1 leading-relaxed ${
                  currentStep.completed
                    ? "line-through text-hive-dimText"
                    : "text-hive-brightText font-medium"
                }`}
              >
                {currentStep.title}
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Saved Checkpoint Indicator */}
      {task.resume_checkpoint && (
        <div className="mt-2.5 flex items-center text-[10px] text-hive-dimText space-x-1.5 bg-hive-base px-2.5 py-1.5 rounded-lg border border-hive-border">
          <M3Icon name="bookmark_added" size={13} className="text-hive-lime flex-shrink-0" />
          <span className="truncate">Checkpoint: {task.resume_checkpoint}</span>
        </div>
      )}
    </div>
  );
};
