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
      workItemId={editor.mode === "edit" ? editor.workItemId : undefined}
      onClose={() => controller.setEditor(null)}
      onSaved={controller.handleWorkItemSaved}
    />
  );
}
