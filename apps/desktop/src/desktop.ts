import { invoke } from "@tauri-apps/api/core";

export interface DataRootCounts {
  work_items: number;
  contexts: number;
  checkpoints: number;
}

export interface DataIssue {
  severity: "error" | "warning";
  code: string;
  path: string;
  message: string;
}

export interface WorkItemSummary {
  id: string;
  project_id: string;
  title: string;
  status: string;
  updated_at: string;
  current_state: string | null;
  last_checkpoint_id: string | null;
}

export interface DataRootSnapshot {
  root: string;
  counts: DataRootCounts;
  issues: DataIssue[];
  work_items: WorkItemSummary[];
  checkpoint_ids: string[];
}

export interface WorkItemClassification {
  initiative_id: string | null;
  work_types: string[];
  tags: string[];
}

export interface ContextFile {
  path: string;
  description: string | null;
}

export interface VerificationState {
  completed: string[];
  pending: string[];
}

export interface WorkContextDetail {
  updated_at: string;
  last_checkpoint_id: string | null;
  current_state: string;
  decisions: string[];
  files: ContextFile[];
  verification: VerificationState;
  next_steps: string[];
  risks: string[];
}

export interface CheckpointVerification {
  kind: string;
  description: string;
  status: string;
  command: string | null;
  evidence_refs: string[];
}

export interface CheckpointDecision {
  summary: string;
  rationale: string;
  status: string;
}

export interface CheckpointEvidence {
  commits: string[];
  pull_requests: string[];
  issues: string[];
  files: string[];
  commands: string[];
  urls: string[];
}

export interface CheckpointGit {
  repository: string;
  branch: string | null;
  head_before: string | null;
  head_after: string | null;
  dirty: boolean | null;
}

export interface CheckpointSummary {
  id: string;
  kind: string;
  captured_at: string;
  title: string;
  summary: string;
  status_after: string;
  markdown_path: string;
  activities: string[];
  decisions: CheckpointDecision[];
  outcomes: string[];
  verifications: CheckpointVerification[];
  blockers: string[];
  next_steps: string[];
  evidence: CheckpointEvidence;
  git: CheckpointGit | null;
}

export interface WorkItemDetail {
  id: string;
  project_id: string;
  title: string;
  status: string;
  objective: string;
  desired_outcomes: string[];
  classification: WorkItemClassification;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
  context: WorkContextDetail | null;
  checkpoints: CheckpointSummary[];
}

export function setDataRoot(root: string) {
  return invoke<DataRootSnapshot>("set_data_root", { root });
}

export function inspectDataRoot() {
  return invoke<DataRootSnapshot>("inspect_data_root");
}

export function getWorkItemDetail(workItemId: string) {
  return invoke<WorkItemDetail>("get_work_item_detail", {
    workItemId,
  });
}

export function revealWorkItem(workItemId: string) {
  return invoke<void>("reveal_work_item", { workItemId });
}

export function openContextMarkdown(workItemId: string) {
  return invoke<void>("open_context_markdown", { workItemId });
}

export function openCheckpointMarkdown(checkpointId: string) {
  return invoke<void>("open_checkpoint_markdown", { checkpointId });
}
