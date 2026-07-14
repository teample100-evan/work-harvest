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
}

export function setDataRoot(root: string) {
  return invoke<DataRootSnapshot>("set_data_root", { root });
}

export function inspectDataRoot() {
  return invoke<DataRootSnapshot>("inspect_data_root");
}
