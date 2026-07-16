import { CheckpointEditor } from "../../CheckpointEditor";
import { PerformanceNoteEditor } from "../../PerformanceNoteEditor";
import { WorkItemEditor } from "../../WorkItemEditor";
import type { WorkspaceController } from "./useWorkspaceController";

interface EditorHostProps {
  controller: WorkspaceController;
}

export function EditorHost({ controller }: EditorHostProps) {
  const { editor } = controller;
  if (!editor) return null;

  const projectOptions = Array.from(
    new Set(controller.snapshot?.work_items.map((item) => item.project_id) ?? []),
  ).sort((left, right) => left.localeCompare(right, "ko-KR"));

  if (editor.mode === "performance-note") {
    return (
      <PerformanceNoteEditor
        workItemId={editor.workItemId}
        onClose={() => controller.setEditor(null)}
        onCreated={controller.handlePerformanceNoteCreated}
      />
    );
  }

  if (editor.mode === "checkpoint") {
    return (
      <CheckpointEditor
        workItemId={editor.workItemId}
        onClose={() => controller.setEditor(null)}
        onSaved={controller.handleWorkItemSaved}
      />
    );
  }

  return (
    <WorkItemEditor
      mode={editor.mode}
      projectOptions={projectOptions}
      workItemId={editor.mode === "edit" ? editor.workItemId : undefined}
      onClose={() => controller.setEditor(null)}
      onSaved={controller.handleWorkItemSaved}
    />
  );
}
