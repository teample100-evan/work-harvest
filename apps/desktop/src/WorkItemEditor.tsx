import { useEffect, useState, type FormEvent } from "react";
import {
  createWorkItem,
  desktopWriteError,
  getWorkItemEditSnapshot,
  previewCreateWorkItem,
  previewUpdateWorkItem,
  updateWorkItem,
  type DesktopWriteError,
  type WorkItemCreateInput,
  type WorkItemEditSnapshot,
  type WorkItemStatus,
  type WorkItemUpdatePatch,
  type WorkItemWritePreview,
} from "./desktop";
import { WriteDiffReview } from "./WriteDiffReview";

interface WorkItemEditorProps {
  mode: "create" | "edit";
  workItemId?: string;
  onClose: () => void;
  onSaved: (workItemId: string) => Promise<void>;
}

interface EditorDraft {
  id: string;
  projectId: string;
  title: string;
  status: WorkItemStatus;
  objective: string;
  desiredOutcomes: string;
  initiativeId: string;
  workTypes: string;
  tags: string;
  currentState: string;
  decisions: string;
  verificationCompleted: string;
  verificationPending: string;
  nextSteps: string;
  risks: string;
}

type PendingCommit =
  | { kind: "create"; input: WorkItemCreateInput; now: string }
  | {
      kind: "update";
      workItemId: string;
      expected: WorkItemEditSnapshot["revisions"];
      patch: WorkItemUpdatePatch;
      now: string;
    };

const statusOptions: Array<{ value: WorkItemStatus; label: string }> = [
  { value: "planned", label: "계획" },
  { value: "in_progress", label: "진행 중" },
  { value: "blocked", label: "막힘" },
  { value: "completed", label: "완료" },
  { value: "cancelled", label: "취소" },
];

function emptyDraft(): EditorDraft {
  return {
    id: "",
    projectId: "",
    title: "",
    status: "planned",
    objective: "",
    desiredOutcomes: "",
    initiativeId: "",
    workTypes: "",
    tags: "",
    currentState: "",
    decisions: "",
    verificationCompleted: "",
    verificationPending: "",
    nextSteps: "",
    risks: "",
  };
}

function formatLines(values: string[]): string {
  return values.join("\n");
}

function splitLines(value: string): string[] {
  return value
    .split("\n")
    .map((item) => item.trim())
    .filter(Boolean);
}

function splitCommaList(value: string): string[] {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function sameStrings(left: string[], right: string[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function draftFromSnapshot(snapshot: WorkItemEditSnapshot): EditorDraft {
  return {
    id: snapshot.work_item.id,
    projectId: snapshot.work_item.project_id,
    title: snapshot.work_item.title,
    status: snapshot.work_item.status,
    objective: snapshot.work_item.objective,
    desiredOutcomes: formatLines(snapshot.work_item.desired_outcomes),
    initiativeId: snapshot.work_item.classification.initiative_id ?? "",
    workTypes: snapshot.work_item.classification.work_types.join(", "),
    tags: snapshot.work_item.classification.tags.join(", "),
    currentState: snapshot.context.current_state,
    decisions: formatLines(snapshot.context.decisions),
    verificationCompleted: formatLines(snapshot.context.verification.completed),
    verificationPending: formatLines(snapshot.context.verification.pending),
    nextSteps: formatLines(snapshot.context.next_steps),
    risks: formatLines(snapshot.context.risks),
  };
}

function createInput(draft: EditorDraft): WorkItemCreateInput {
  return {
    id: draft.id.trim(),
    project_id: draft.projectId.trim(),
    title: draft.title.trim(),
    status: draft.status,
    objective: draft.objective.trim(),
    desired_outcomes: splitLines(draft.desiredOutcomes),
    classification: {
      initiative_id: draft.initiativeId.trim() || null,
      work_types: splitCommaList(draft.workTypes),
      tags: splitCommaList(draft.tags),
    },
    context: {
      current_state: draft.currentState.trim(),
      decisions: splitLines(draft.decisions),
      verification: {
        completed: splitLines(draft.verificationCompleted),
        pending: splitLines(draft.verificationPending),
      },
      next_steps: splitLines(draft.nextSteps),
      risks: splitLines(draft.risks),
    },
  };
}

function updatePatch(draft: EditorDraft, snapshot: WorkItemEditSnapshot): WorkItemUpdatePatch {
  const patch: WorkItemUpdatePatch = {};
  const title = draft.title.trim();
  const objective = draft.objective.trim();
  const desiredOutcomes = splitLines(draft.desiredOutcomes);
  const initiativeId = draft.initiativeId.trim() || null;
  const workTypes = splitCommaList(draft.workTypes);
  const tags = splitCommaList(draft.tags);

  if (title !== snapshot.work_item.title) patch.title = title;
  if (draft.status !== snapshot.work_item.status) patch.status = draft.status;
  if (objective !== snapshot.work_item.objective) patch.objective = objective;
  if (!sameStrings(desiredOutcomes, snapshot.work_item.desired_outcomes)) {
    patch.desired_outcomes = desiredOutcomes;
  }
  if (
    initiativeId !== snapshot.work_item.classification.initiative_id ||
    !sameStrings(workTypes, snapshot.work_item.classification.work_types) ||
    !sameStrings(tags, snapshot.work_item.classification.tags)
  ) {
    patch.classification = {
      initiative_id: initiativeId,
      work_types: workTypes,
      tags,
    };
  }

  const context: NonNullable<WorkItemUpdatePatch["context"]> = {};
  const currentState = draft.currentState.trim();
  const decisions = splitLines(draft.decisions);
  const completed = splitLines(draft.verificationCompleted);
  const pending = splitLines(draft.verificationPending);
  const nextSteps = splitLines(draft.nextSteps);
  const risks = splitLines(draft.risks);

  if (currentState !== snapshot.context.current_state) context.current_state = currentState;
  if (!sameStrings(decisions, snapshot.context.decisions)) context.decisions = decisions;
  if (
    !sameStrings(completed, snapshot.context.verification.completed) ||
    !sameStrings(pending, snapshot.context.verification.pending)
  ) {
    context.verification = { completed, pending };
  }
  if (!sameStrings(nextSteps, snapshot.context.next_steps)) context.next_steps = nextSteps;
  if (!sameStrings(risks, snapshot.context.risks)) context.risks = risks;
  if (Object.keys(context).length > 0) patch.context = context;

  return patch;
}

function errorTitle(error: DesktopWriteError): string {
  switch (error.kind) {
    case "revision_conflict":
      return "파일이 다른 곳에서 변경되었습니다.";
    case "create_conflict":
      return "같은 ID의 업무 파일이 이미 있습니다.";
    case "lock_busy":
      return "다른 writer가 저장 중입니다.";
    case "validation":
      return "입력 내용을 확인해 주세요.";
    case "root_required":
      return "먼저 데이터 폴더를 선택해 주세요.";
    case "not_found":
      return "업무 파일을 찾을 수 없습니다.";
    default:
      return "파일을 저장하지 못했습니다.";
  }
}

export function WorkItemEditor({ mode, workItemId, onClose, onSaved }: WorkItemEditorProps) {
  const [draft, setDraft] = useState<EditorDraft>(emptyDraft);
  const [snapshot, setSnapshot] = useState<WorkItemEditSnapshot | null>(null);
  const [preview, setPreview] = useState<WorkItemWritePreview | null>(null);
  const [pendingCommit, setPendingCommit] = useState<PendingCommit | null>(null);
  const [error, setError] = useState<DesktopWriteError | null>(null);
  const [loading, setLoading] = useState(mode === "edit");
  const [saving, setSaving] = useState(false);
  const [loadVersion, setLoadVersion] = useState(0);

  const patch = mode === "edit" && snapshot ? updatePatch(draft, snapshot) : null;
  const hasChanges = mode === "create" ? true : Boolean(patch && Object.keys(patch).length > 0);
  const isDirty =
    mode === "edit"
      ? hasChanges
      : Object.entries(draft).some(([key, value]) => key !== "status" && value.trim().length > 0);

  useEffect(() => {
    let cancelled = false;

    setError(null);
    setPreview(null);
    setPendingCommit(null);

    if (mode === "create") {
      setSnapshot(null);
      setDraft(emptyDraft());
      setLoading(false);
      return () => {
        cancelled = true;
      };
    }

    if (!workItemId) {
      setLoading(false);
      setError({ kind: "not_found", message: "work item id is required" });
      return () => {
        cancelled = true;
      };
    }

    setLoading(true);
    getWorkItemEditSnapshot(workItemId)
      .then((nextSnapshot) => {
        if (cancelled) return;
        setSnapshot(nextSnapshot);
        setDraft(draftFromSnapshot(nextSnapshot));
      })
      .catch((nextError: unknown) => {
        if (!cancelled) setError(desktopWriteError(nextError));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [loadVersion, mode, workItemId]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key !== "Escape" || saving) return;
      if (isDirty && !window.confirm("저장하지 않은 변경 사항을 버릴까요?")) return;
      onClose();
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isDirty, onClose, saving]);

  function requestClose() {
    if (saving) return;
    if (isDirty && !window.confirm("저장하지 않은 변경 사항을 버릴까요?")) return;
    onClose();
  }

  async function reviewChanges(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    const now = new Date().toISOString();
    try {
      if (mode === "create") {
        const input = createInput(draft);
        const nextPreview = await previewCreateWorkItem(input, now);
        setPendingCommit({ kind: "create", input, now });
        setPreview(nextPreview);
        return;
      }

      if (!snapshot || !patch || Object.keys(patch).length === 0) return;
      const nextPreview = await previewUpdateWorkItem(
        snapshot.work_item.id,
        snapshot.revisions,
        patch,
        now,
      );
      setPendingCommit({
        kind: "update",
        workItemId: snapshot.work_item.id,
        expected: snapshot.revisions,
        patch,
        now,
      });
      setPreview(nextPreview);
    } catch (nextError) {
      setError(desktopWriteError(nextError));
    }
  }

  async function commitChanges() {
    if (!pendingCommit) return;
    setSaving(true);
    setError(null);

    try {
      const result =
        pendingCommit.kind === "create"
          ? await createWorkItem(pendingCommit.input, pendingCommit.now)
          : await updateWorkItem(
              pendingCommit.workItemId,
              pendingCommit.expected,
              pendingCommit.patch,
              pendingCommit.now,
            );
      await onSaved(result.work_item.id);
      onClose();
    } catch (nextError) {
      const writeError = desktopWriteError(nextError);
      setError(writeError);
      if (writeError.kind === "revision_conflict") {
        setPreview(null);
        setPendingCommit(null);
      }
    } finally {
      setSaving(false);
    }
  }

  function reloadLatest() {
    setError(null);
    setLoadVersion((version) => version + 1);
  }

  return (
    <div className="editor-backdrop">
      <section
        className="editor-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="work-item-editor-title"
      >
        <header className="editor-header">
          <div>
            <span className="eyebrow">{mode === "create" ? "새 업무" : "업무 편집"}</span>
            <h2 id="work-item-editor-title">
              {mode === "create" ? "업무 항목 만들기" : snapshot?.work_item.title ?? workItemId}
            </h2>
          </div>
          <button type="button" className="icon-button" onClick={requestClose} aria-label="편집기 닫기">
            ×
          </button>
        </header>

        <div className="editor-body">
          {error ? (
            <div className={`editor-alert ${error.kind}`} role="alert">
              <div>
                <strong>{errorTitle(error)}</strong>
                <p>
                  {error.kind === "revision_conflict"
                    ? "현재 입력을 최신 파일 내용으로 교체한 뒤 다시 편집할 수 있습니다. 자동 덮어쓰기는 하지 않습니다."
                    : error.kind === "lock_busy"
                      ? "잠시 후 변경 검토 또는 저장을 다시 시도해 주세요."
                      : error.message}
                </p>
                {(error.kind === "revision_conflict" || error.kind === "lock_busy") && (
                  <details>
                    <summary>기술 상세</summary>
                    <code>{error.message}</code>
                  </details>
                )}
              </div>
              {error.kind === "revision_conflict" && (
                <button type="button" className="secondary-button" onClick={reloadLatest}>
                  최신 내용 다시 불러오기
                </button>
              )}
            </div>
          ) : null}

          {loading ? (
            <div className="editor-loading" role="status">
              편집 snapshot을 불러오는 중…
            </div>
          ) : preview ? (
            <WriteDiffReview
              eyebrow="저장 전 검토"
              title={preview.work_item.title}
              identity={preview.work_item.id}
              status={preview.work_item.status}
              files={preview.files}
              saving={saving}
              onBack={() => {
                setPreview(null);
                setPendingCommit(null);
              }}
              onCommit={commitChanges}
            />
          ) : (
            <form className="editor-form" onSubmit={reviewChanges} onChange={() => setError(null)}>
              <section className="editor-section" aria-labelledby="editor-identity-title">
                <div className="editor-section-heading">
                  <div>
                    <span className="eyebrow">Identity</span>
                    <h3 id="editor-identity-title">업무 정의</h3>
                  </div>
                  {mode === "edit" ? <span className="editor-preserved">ID와 프로젝트는 유지됩니다</span> : null}
                </div>
                <div className="editor-grid">
                  <label className="editor-field">
                    <span>업무 ID</span>
                    <input
                      required
                      minLength={2}
                      maxLength={64}
                      pattern="[A-Za-z0-9][A-Za-z0-9._-]+"
                      value={draft.id}
                      disabled={mode === "edit"}
                      placeholder="desktop-editor"
                      onChange={(event) => setDraft({ ...draft, id: event.target.value })}
                    />
                  </label>
                  <label className="editor-field">
                    <span>프로젝트 ID</span>
                    <input
                      required
                      value={draft.projectId}
                      disabled={mode === "edit"}
                      placeholder="work-harvest"
                      onChange={(event) => setDraft({ ...draft, projectId: event.target.value })}
                    />
                  </label>
                  <label className="editor-field full">
                    <span>제목</span>
                    <input
                      required
                      maxLength={160}
                      value={draft.title}
                      placeholder="무엇을 달성하려는 업무인가요?"
                      onChange={(event) => setDraft({ ...draft, title: event.target.value })}
                    />
                  </label>
                  <label className="editor-field">
                    <span>상태</span>
                    <select
                      value={draft.status}
                      onChange={(event) =>
                        setDraft({ ...draft, status: event.target.value as WorkItemStatus })
                      }
                    >
                      {statusOptions.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="editor-field">
                    <span>Initiative ID</span>
                    <input
                      value={draft.initiativeId}
                      placeholder="선택 사항"
                      onChange={(event) => setDraft({ ...draft, initiativeId: event.target.value })}
                    />
                  </label>
                  <label className="editor-field full">
                    <span>목표</span>
                    <textarea
                      required
                      rows={3}
                      value={draft.objective}
                      placeholder="완료했을 때 달라져야 하는 상태를 적어 주세요."
                      onChange={(event) => setDraft({ ...draft, objective: event.target.value })}
                    />
                  </label>
                  <label className="editor-field full">
                    <span>원하는 결과</span>
                    <textarea
                      rows={3}
                      value={draft.desiredOutcomes}
                      placeholder={"한 줄에 하나씩 입력\n예: GUI에서 안전하게 업무를 수정할 수 있다"}
                      onChange={(event) => setDraft({ ...draft, desiredOutcomes: event.target.value })}
                    />
                  </label>
                  <label className="editor-field">
                    <span>업무 유형</span>
                    <input
                      value={draft.workTypes}
                      placeholder="feature, docs"
                      onChange={(event) => setDraft({ ...draft, workTypes: event.target.value })}
                    />
                    <small>쉼표로 구분합니다.</small>
                  </label>
                  <label className="editor-field">
                    <span>태그</span>
                    <input
                      value={draft.tags}
                      placeholder="desktop, tauri"
                      onChange={(event) => setDraft({ ...draft, tags: event.target.value })}
                    />
                    <small>쉼표로 구분합니다.</small>
                  </label>
                </div>
              </section>

              <section className="editor-section" aria-labelledby="editor-context-title">
                <div className="editor-section-heading">
                  <div>
                    <span className="eyebrow">Context</span>
                    <h3 id="editor-context-title">이어가기 위한 맥락</h3>
                  </div>
                  {mode === "edit" ? (
                    <span className="editor-preserved">파일·Git·저장소·링크는 그대로 보존됩니다</span>
                  ) : null}
                </div>
                <div className="editor-grid">
                  <label className="editor-field full">
                    <span>현재 상태</span>
                    <textarea
                      required
                      rows={4}
                      value={draft.currentState}
                      placeholder="지금 어디까지 왔고 무엇이 중요한가요?"
                      onChange={(event) => setDraft({ ...draft, currentState: event.target.value })}
                    />
                  </label>
                  <label className="editor-field full">
                    <span>결정</span>
                    <textarea
                      rows={3}
                      value={draft.decisions}
                      placeholder="한 줄에 하나씩 입력"
                      onChange={(event) => setDraft({ ...draft, decisions: event.target.value })}
                    />
                  </label>
                  <label className="editor-field">
                    <span>검증 완료</span>
                    <textarea
                      rows={3}
                      value={draft.verificationCompleted}
                      placeholder="통과한 검증을 한 줄에 하나씩"
                      onChange={(event) =>
                        setDraft({ ...draft, verificationCompleted: event.target.value })
                      }
                    />
                  </label>
                  <label className="editor-field">
                    <span>검증 대기</span>
                    <textarea
                      rows={3}
                      value={draft.verificationPending}
                      placeholder="남은 검증을 한 줄에 하나씩"
                      onChange={(event) =>
                        setDraft({ ...draft, verificationPending: event.target.value })
                      }
                    />
                  </label>
                  <label className="editor-field">
                    <span>다음 단계</span>
                    <textarea
                      rows={4}
                      value={draft.nextSteps}
                      placeholder="한 줄에 하나씩 입력"
                      onChange={(event) => setDraft({ ...draft, nextSteps: event.target.value })}
                    />
                  </label>
                  <label className="editor-field">
                    <span>위험·막힘</span>
                    <textarea
                      rows={4}
                      value={draft.risks}
                      placeholder="한 줄에 하나씩 입력"
                      onChange={(event) => setDraft({ ...draft, risks: event.target.value })}
                    />
                  </label>
                </div>
              </section>

              <footer className="editor-footer">
                <div className="editor-footer-copy" aria-live="polite">
                  {mode === "edit" && !hasChanges
                    ? "변경 사항이 없습니다."
                    : "먼저 실제 파일 diff를 생성해 검토합니다."}
                </div>
                <button type="button" className="ghost-button" onClick={requestClose}>
                  취소
                </button>
                <button type="submit" className="primary-button" disabled={!hasChanges}>
                  변경 검토
                </button>
              </footer>
            </form>
          )}
        </div>
      </section>
    </div>
  );
}
