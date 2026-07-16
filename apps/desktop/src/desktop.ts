import { invoke } from "@tauri-apps/api/core";

export interface BuildInfo {
  version: string;
  commit: string;
  dirty: boolean;
  built_at_unix: number;
  profile: "debug" | "release";
}

export function getBuildInfo() {
  return invoke<BuildInfo>("get_build_info");
}

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
  activity_dates: string[];
  current_state: string | null;
  last_checkpoint_id: string | null;
  last_checkpoint_confidentiality: "normal" | "sensitive" | "restricted" | null;
}

export interface DataRootSnapshot {
  root: string;
  counts: DataRootCounts;
  issues: DataIssue[];
  work_items: WorkItemSummary[];
  checkpoint_ids: string[];
  restricted_checkpoint_ids: string[];
}

export interface DataRootUpdate {
  snapshot: DataRootSnapshot;
  changed_work_item_ids: string[];
  full_rescan: boolean;
  reloaded_files: number;
  revision: number;
  applied: boolean;
}

export interface DataRootChange extends DataRootUpdate {
  paths: string[];
  event_count: number;
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
  confidentiality: "normal" | "sensitive" | "restricted";
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

export type WorkItemStatus =
  | "planned"
  | "in_progress"
  | "blocked"
  | "completed"
  | "cancelled";

export interface FileRevision {
  sha256: string;
  bytes: number;
}

export interface WorkItemEditRevisions {
  work_item: FileRevision;
  context_data: FileRevision;
  context: FileRevision;
}

export interface StoredWorkItemClassification {
  initiative_id: string | null;
  work_types: string[];
  tags: string[];
}

export interface StoredContextFile {
  path: string;
  description: string | null;
}

export interface StoredContextGit {
  repository: string | null;
  branch: string | null;
  commit: string | null;
  checked_at: string | null;
}

export interface StoredContextVerification {
  completed: string[];
  pending: string[];
}

export interface WorkItemDocument {
  schema_version: string;
  id: string;
  project_id: string;
  title: string;
  status: WorkItemStatus;
  objective: string;
  desired_outcomes: string[];
  classification: StoredWorkItemClassification;
  repositories: unknown[];
  links: unknown[];
  context_path: string;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
}

export interface WorkContextDocument {
  schema_version: string;
  work_item_id: string;
  project_id: string;
  updated_at: string;
  last_checkpoint_id: string | null;
  last_verified_git_ref: string | null;
  current_state: string;
  decisions: string[];
  files: StoredContextFile[];
  verification: StoredContextVerification;
  next_steps: string[];
  risks: string[];
  git: StoredContextGit;
}

export interface WorkItemPaths {
  work_item: string;
  context_data: string;
  context: string;
}

export interface WorkItemEditSnapshot {
  work_item: WorkItemDocument;
  context: WorkContextDocument;
  work_item_json: string;
  context_json: string;
  markdown: string;
  paths: WorkItemPaths;
  revisions: WorkItemEditRevisions;
}

export interface WorkItemClassificationInput {
  initiative_id?: string | null;
  work_types?: string[];
  tags?: string[];
}

export interface ContextVerificationInput {
  completed?: string[];
  pending?: string[];
}

export interface WorkContextInput {
  current_state?: string;
  decisions?: string[];
  verification?: ContextVerificationInput;
  next_steps?: string[];
  risks?: string[];
}

export interface WorkItemCreateInput {
  id: string;
  project_id: string;
  title: string;
  status?: WorkItemStatus;
  objective: string;
  desired_outcomes?: string[];
  classification?: WorkItemClassificationInput;
  repositories?: unknown[];
  links?: unknown[];
  context?: WorkContextInput;
}

export interface WorkContextPatch {
  current_state?: string;
  decisions?: string[];
  verification?: ContextVerificationInput;
  next_steps?: string[];
  risks?: string[];
}

export interface WorkItemUpdatePatch {
  title?: string;
  status?: WorkItemStatus;
  objective?: string;
  desired_outcomes?: string[];
  classification?: WorkItemClassificationInput;
  context?: WorkContextPatch;
}

export interface WorkItemFileChange {
  path: string;
  operation: "create" | "replace";
  before: string | null;
  after: string;
}

export interface WorkItemWritePreview {
  work_item: WorkItemDocument;
  context: WorkContextDocument;
  files: WorkItemFileChange[];
}

export interface WorkItemWriteResult {
  work_item: WorkItemDocument;
  context: WorkContextDocument;
  paths: WorkItemPaths;
  revisions: WorkItemEditRevisions;
  commit: {
    transaction_id: string;
    written_paths: string[];
  };
}

export type CheckpointKind = "progress" | "final" | "correction" | "backfill";

export interface CheckpointSourceInput {
  agent?: "codex" | "claude-code" | "manual" | "other";
  surface?: "desktop" | "cli" | "ide" | "cloud" | "unknown";
  session_ref?: string | null;
  task_title?: string | null;
}

export interface CheckpointWorkPeriodInput {
  start?: string | null;
  end?: string | null;
  precision?: "exact" | "day" | "range" | "unknown";
  basis?: Array<"checkpoint" | "activity-event" | "transcript" | "git" | "user" | "unknown">;
  timezone?: string;
}

export interface CheckpointDecisionInput {
  summary: string;
  rationale: string;
  status: "proposed" | "accepted" | "superseded";
}

export interface CheckpointVerificationInput {
  type: "test" | "build" | "lint" | "manual" | "measurement" | "review" | "other";
  description: string;
  status: "passed" | "failed" | "partial" | "not_run";
  command: string | null;
  evidence_refs: string[];
}

export interface CheckpointOutcomeInput {
  description: string;
  impact: string | null;
  evidence_refs: string[];
}

export interface CheckpointEvidenceInput {
  commits?: string[];
  pull_requests?: string[];
  issues?: string[];
  files?: string[];
  commands?: string[];
  urls?: string[];
}

export interface CheckpointGitInput {
  repository: string;
  branch: string | null;
  head_before: string | null;
  head_after: string | null;
  dirty: boolean | null;
}

export interface CheckpointContextUpdateInput {
  current_state?: string;
  decisions?: string[];
  files?: Array<string | StoredContextFile>;
  verification?: ContextVerificationInput;
  verification_completed?: string[];
  verification_pending?: string[];
  next_steps?: string[];
  risks?: string[];
  git?: Partial<StoredContextGit>;
}

export interface CheckpointInput {
  id?: string;
  work_item_id: string;
  kind?: CheckpointKind;
  source?: CheckpointSourceInput;
  captured_at?: string;
  work_period?: CheckpointWorkPeriodInput;
  title: string;
  summary: string;
  status_after?: WorkItemStatus;
  activities?: string[];
  decisions?: CheckpointDecisionInput[];
  verifications?: CheckpointVerificationInput[];
  outcomes?: CheckpointOutcomeInput[];
  blockers?: string[];
  next_steps?: string[];
  evidence?: CheckpointEvidenceInput;
  git?: CheckpointGitInput;
  related_checkpoint_ids?: string[];
  correction_of?: string | null;
  confidentiality?: "normal" | "sensitive" | "restricted";
  context_update?: CheckpointContextUpdateInput;
}

export interface CheckpointDocument extends Omit<CheckpointInput, "id" | "kind" | "source" | "captured_at" | "work_period" | "status_after" | "correction_of" | "context_update"> {
  schema_version: string;
  id: string;
  project_id: string;
  kind: CheckpointKind;
  source: Required<CheckpointSourceInput>;
  captured_at: string;
  work_period: Required<CheckpointWorkPeriodInput>;
  status_after: WorkItemStatus;
  activities: string[];
  decisions: CheckpointDecisionInput[];
  verifications: CheckpointVerificationInput[];
  outcomes: CheckpointOutcomeInput[];
  blockers: string[];
  next_steps: string[];
  evidence: Required<CheckpointEvidenceInput>;
  related_checkpoint_ids: string[];
  correction_of: string | null;
  confidentiality: "normal" | "sensitive" | "restricted";
}

export interface CheckpointPaths {
  checkpoint: string;
  checkpoint_markdown: string;
  work_item: string;
  context_data: string;
  context: string;
}

export interface CheckpointWritePreview {
  checkpoint: CheckpointDocument;
  work_item: WorkItemDocument;
  context: WorkContextDocument;
  paths: CheckpointPaths;
  files: WorkItemFileChange[];
}

export interface CheckpointWriteResult {
  checkpoint: CheckpointDocument;
  work_item: WorkItemDocument;
  context: WorkContextDocument;
  paths: CheckpointPaths;
  revisions: WorkItemEditRevisions;
  commit: WorkItemWriteResult["commit"];
}

export interface PerformanceNoteInput {
  work_item_id: string;
  output?: string | null;
  markdown?: string | null;
}

export interface PerformanceNoteSourceRevision {
  path: string;
  revision: FileRevision;
}

export interface PerformanceNotePaths {
  report: string;
}

export interface PerformanceNoteWritePreview {
  work_item: WorkItemDocument;
  checkpoint_count: number;
  redacted_checkpoint_count: number;
  excluded_checkpoint_count: number;
  paths: PerformanceNotePaths;
  source_revisions: PerformanceNoteSourceRevision[];
  files: WorkItemFileChange[];
}

export interface PerformanceNoteWriteResult {
  work_item: WorkItemDocument;
  checkpoint_count: number;
  redacted_checkpoint_count: number;
  excluded_checkpoint_count: number;
  paths: PerformanceNotePaths;
  source_revisions: PerformanceNoteSourceRevision[];
  commit: WorkItemWriteResult["commit"];
}

export interface WeeklyReportInput {
  start_date: string;
  end_date: string;
  output?: string | null;
  markdown?: string | null;
}

export interface WeeklyReportStats {
  work_item_count: number;
  checkpoint_count: number;
  redacted_checkpoint_count: number;
  excluded_checkpoint_count: number;
  unknown_period_checkpoint_count: number;
  git_commit_count: number;
  verification_count: number;
  passed_verification_count: number;
  failed_verification_count: number;
  partial_verification_count: number;
  not_run_verification_count: number;
}

export interface WeeklyReportWritePreview {
  start_date: string;
  end_date: string;
  stats: WeeklyReportStats;
  paths: PerformanceNotePaths;
  source_revisions: PerformanceNoteSourceRevision[];
  report_revision: FileRevision | null;
  replaces_existing: boolean;
  files: WorkItemFileChange[];
}

export interface WeeklyReportWriteResult {
  start_date: string;
  end_date: string;
  stats: WeeklyReportStats;
  paths: PerformanceNotePaths;
  source_revisions: PerformanceNoteSourceRevision[];
  replaced_existing: boolean;
  commit: WorkItemWriteResult["commit"];
}

export type DesktopWriteErrorKind =
  | "root_required"
  | "not_found"
  | "validation"
  | "create_conflict"
  | "revision_conflict"
  | "lock_busy"
  | "write_failed";

export interface DesktopWriteError {
  kind: DesktopWriteErrorKind;
  message: string;
}

export function desktopWriteError(error: unknown): DesktopWriteError {
  if (
    typeof error === "object" &&
    error !== null &&
    "kind" in error &&
    "message" in error &&
    typeof error.kind === "string" &&
    typeof error.message === "string"
  ) {
    return error as DesktopWriteError;
  }
  if (typeof error === "string") {
    try {
      return desktopWriteError(JSON.parse(error));
    } catch {
      return { kind: "write_failed", message: error };
    }
  }
  return {
    kind: "write_failed",
    message: error instanceof Error ? error.message : String(error),
  };
}

export function setDataRoot(root: string) {
  return invoke<DataRootSnapshot>("set_data_root", { root });
}

export function inspectDataRoot() {
  return invoke<DataRootUpdate>("inspect_data_root");
}

export function getWorkItemDetail(workItemId: string) {
  return invoke<WorkItemDetail>("get_work_item_detail", {
    workItemId,
  });
}

export function getWorkItemEditSnapshot(workItemId: string) {
  return invoke<WorkItemEditSnapshot>("get_work_item_edit_snapshot", {
    workItemId,
  });
}

export function previewCreateWorkItem(input: WorkItemCreateInput, now: string) {
  return invoke<WorkItemWritePreview>("preview_create_work_item", {
    input,
    now,
  });
}

export function previewUpdateWorkItem(
  workItemId: string,
  expected: WorkItemEditRevisions,
  patch: WorkItemUpdatePatch,
  now: string,
) {
  return invoke<WorkItemWritePreview>("preview_update_work_item", {
    workItemId,
    expected,
    patch,
    now,
  });
}

export function createWorkItem(input: WorkItemCreateInput, now: string) {
  return invoke<WorkItemWriteResult>("create_work_item", { input, now });
}

export function updateWorkItem(
  workItemId: string,
  expected: WorkItemEditRevisions,
  patch: WorkItemUpdatePatch,
  now: string,
) {
  return invoke<WorkItemWriteResult>("update_work_item", {
    workItemId,
    expected,
    patch,
    now,
  });
}

export function previewCaptureCheckpoint(
  input: CheckpointInput,
  expected: WorkItemEditRevisions,
  now: string,
) {
  return invoke<CheckpointWritePreview>("preview_capture_checkpoint", {
    input,
    expected,
    now,
  });
}

export function captureCheckpoint(
  input: CheckpointInput,
  expected: WorkItemEditRevisions,
  now: string,
) {
  return invoke<CheckpointWriteResult>("capture_checkpoint", {
    input,
    expected,
    now,
  });
}

export function previewPerformanceNote(input: PerformanceNoteInput, generatedAt: string) {
  return invoke<PerformanceNoteWritePreview>("preview_performance_note", {
    input,
    generatedAt,
  });
}

export function createPerformanceNote(
  input: PerformanceNoteInput,
  expected: PerformanceNoteSourceRevision[],
  generatedAt: string,
) {
  return invoke<PerformanceNoteWriteResult>("create_performance_note", {
    input,
    expected,
    generatedAt,
  });
}

export function previewWeeklyReport(input: WeeklyReportInput, generatedAt: string) {
  return invoke<WeeklyReportWritePreview>("preview_weekly_report", {
    input,
    generatedAt,
  });
}

export function createWeeklyReport(
  input: WeeklyReportInput,
  expected: PerformanceNoteSourceRevision[],
  expectedReportRevision: FileRevision | null,
  generatedAt: string,
) {
  return invoke<WeeklyReportWriteResult>("create_weekly_report", {
    input,
    expected,
    expectedReportRevision,
    generatedAt,
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

export function openExternalUrl(url: string) {
  return invoke<void>("open_external_url", { url });
}

export function openPerformanceNoteMarkdown(report: string) {
  return invoke<void>("open_performance_note_markdown", { report });
}

export function openWeeklyReportMarkdown(report: string) {
  return invoke<void>("open_weekly_report_markdown", { report });
}
