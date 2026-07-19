import { useEffect, useState, type FormEvent } from "react";
import {
  CheckCircle2,
  ChevronRight,
  ClipboardList,
  PlayCircle,
  SlidersHorizontal,
  type LucideIcon,
} from "lucide-react";
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
  type WorkItemReportingMode,
  type WorkItemScope,
  type WorkItemStatus,
  type WorkItemUpdatePatch,
  type WorkItemWritePreview,
} from "./desktop";
import { WriteDiffReview } from "./WriteDiffReview";
import { Button } from "./ui/Button";
import { EditorDialog } from "./ui/EditorDialog";
import { ProjectCombobox } from "./ui/ProjectCombobox";
import { SelectMenu } from "./ui/SelectMenu";
import { TokenInput } from "./ui/TokenInput";
import { clearControlValidation, validateControls } from "./ui/formValidation";

interface WorkItemEditorProps {
  mode: "create" | "edit";
  projectOptions: string[];
  workItemId?: string;
  onClose: () => void;
  onSaved: (workItemId: string) => Promise<void>;
}

type CreateScenario = "planned" | "in_progress" | "completed" | "custom";

interface CreateScenarioOption {
  description: string;
  icon: LucideIcon;
  label: string;
  status: WorkItemStatus;
  value: CreateScenario;
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
  scope: WorkItemScope;
  reportingMode: WorkItemReportingMode;
  exclusionReason: string;
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

const scopeOptions: Array<{ value: WorkItemScope; label: string }> = [
  { value: "unclassified", label: "선택 필요" },
  { value: "company", label: "회사 업무" },
  { value: "personal", label: "개인 업무" },
];

const reportingModeOptions: Array<{ value: WorkItemReportingMode; label: string }> = [
  { value: "primary", label: "주요 업무 · 보고서에 한 줄로 포함" },
  { value: "supporting", label: "지원 활동 · 기록만 유지" },
  { value: "excluded", label: "보고 제외" },
];

const createScenarioOptions: CreateScenarioOption[] = [
  {
    value: "planned",
    label: "계획 중",
    description: "시작 전 목표를 정리합니다.",
    status: "planned",
    icon: ClipboardList,
  },
  {
    value: "in_progress",
    label: "진행 중",
    description: "현재 상태와 다음 단계를 기록합니다.",
    status: "in_progress",
    icon: PlayCircle,
  },
  {
    value: "completed",
    label: "완료 기록",
    description: "수행 내용과 결과를 남깁니다.",
    status: "completed",
    icon: CheckCircle2,
  },
  {
    value: "custom",
    label: "직접 구성",
    description: "정해진 흐름 없이 작성합니다.",
    status: "planned",
    icon: SlidersHorizontal,
  },
];

const scenarioCopy: Record<
  CreateScenario,
  {
    currentLabel: string;
    currentPlaceholder: string;
    objectiveLabel: string;
    objectivePlaceholder: string;
  }
> = {
  planned: {
    objectiveLabel: "목표",
    objectivePlaceholder: "이 업무로 이루려는 결과는 무엇인가요?",
    currentLabel: "현재 상태",
    currentPlaceholder: "현재 공유할 맥락을 적어 주세요.",
  },
  in_progress: {
    objectiveLabel: "목표",
    objectivePlaceholder: "진행 이유나 완료 기준을 적어 주세요.",
    currentLabel: "현재 상태",
    currentPlaceholder: "지금 어디까지 진행됐나요?",
  },
  completed: {
    objectiveLabel: "기록 목적",
    objectivePlaceholder: "어떤 문제나 요청을 해결한 업무였나요?",
    currentLabel: "수행한 내용",
    currentPlaceholder: "실제로 진행한 작업을 요약해 주세요.",
  },
  custom: {
    objectiveLabel: "목표",
    objectivePlaceholder: "이 업무의 목적을 적어 주세요.",
    currentLabel: "현재 상태 (선택)",
    currentPlaceholder: "지금 공유해야 할 핵심 맥락을 적어 주세요.",
  },
};

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
    scope: "unclassified",
    reportingMode: "primary",
    exclusionReason: "",
    currentState: "",
    decisions: "",
    verificationCompleted: "",
    verificationPending: "",
    nextSteps: "",
    risks: "",
  };
}

function formatIdSuffix(date: Date): string {
  const twoDigits = (value: number) => String(value).padStart(2, "0");
  return `${String(date.getFullYear()).slice(-2)}${twoDigits(date.getMonth() + 1)}${twoDigits(
    date.getDate(),
  )}-${twoDigits(date.getHours())}${twoDigits(date.getMinutes())}`;
}

function suggestWorkItemId(title: string, projectId: string, suffix: string): string {
  const normalize = (value: string) =>
    value
      .normalize("NFKD")
      .toLowerCase()
      .replace(/[^a-z0-9._-]+/g, "-")
      .replace(/^[._-]+|[._-]+$/g, "")
      .slice(0, 40);
  const seed = normalize(title) || normalize(projectId) || "work";
  return `${seed}-${suffix}`.slice(0, 64);
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
    scope: snapshot.work_item.scope,
    reportingMode: snapshot.work_item.reporting.mode,
    exclusionReason: snapshot.work_item.reporting.exclusion_reason ?? "",
    currentState: snapshot.context.current_state,
    decisions: formatLines(snapshot.context.decisions),
    verificationCompleted: formatLines(snapshot.context.verification.completed),
    verificationPending: formatLines(snapshot.context.verification.pending),
    nextSteps: formatLines(snapshot.context.next_steps),
    risks: formatLines(snapshot.context.risks),
  };
}

function createInput(draft: EditorDraft): WorkItemCreateInput {
  const currentState = draft.currentState.trim();
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
    scope: draft.scope,
    reporting: {
      mode: draft.reportingMode,
      exclusion_reason: draft.exclusionReason.trim() || null,
    },
    context: {
      ...(currentState ? { current_state: currentState } : {}),
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
  const exclusionReason = draft.exclusionReason.trim() || null;

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
  if (draft.scope !== snapshot.work_item.scope) patch.scope = draft.scope;
  if (
    draft.reportingMode !== snapshot.work_item.reporting.mode ||
    exclusionReason !== snapshot.work_item.reporting.exclusion_reason
  ) {
    patch.reporting = {
      mode: draft.reportingMode,
      exclusion_reason: exclusionReason,
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

export function WorkItemEditor({
  mode,
  projectOptions,
  workItemId,
  onClose,
  onSaved,
}: WorkItemEditorProps) {
  const [draft, setDraft] = useState<EditorDraft>(emptyDraft);
  const [createScenario, setCreateScenario] = useState<CreateScenario>("planned");
  const [idEdited, setIdEdited] = useState(false);
  const [idSuffix] = useState(() => formatIdSuffix(new Date()));
  const [snapshot, setSnapshot] = useState<WorkItemEditSnapshot | null>(null);
  const [preview, setPreview] = useState<WorkItemWritePreview | null>(null);
  const [pendingCommit, setPendingCommit] = useState<PendingCommit | null>(null);
  const [error, setError] = useState<DesktopWriteError | null>(null);
  const [loading, setLoading] = useState(mode === "edit");
  const [saving, setSaving] = useState(false);
  const [validationMessage, setValidationMessage] = useState<string | null>(null);
  const [loadVersion, setLoadVersion] = useState(0);

  const patch = mode === "edit" && snapshot ? updatePatch(draft, snapshot) : null;
  const hasChanges = mode === "create" ? true : Boolean(patch && Object.keys(patch).length > 0);
  const isDirty =
    mode === "edit"
      ? hasChanges
      : Object.entries(draft).some(
          ([key, value]) => key !== "status" && key !== "id" && value.trim().length > 0,
        );

  useEffect(() => {
    let cancelled = false;

    setError(null);
    setPreview(null);
    setPendingCommit(null);

    if (mode === "create") {
      setSnapshot(null);
      setDraft(emptyDraft());
      setCreateScenario("planned");
      setIdEdited(false);
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
    if (mode !== "create" || idEdited) return;
    const suggestedId = suggestWorkItemId(draft.title, draft.projectId, idSuffix);
    setDraft((current) =>
      current.id === suggestedId ? current : { ...current, id: suggestedId },
    );
  }, [draft.projectId, draft.title, idEdited, idSuffix, mode]);

  function changeCreateScenario(nextScenario: CreateScenario) {
    const option = createScenarioOptions.find((item) => item.value === nextScenario);
    setCreateScenario(nextScenario);
    if (option) setDraft((current) => ({ ...current, status: option.status }));
  }

  function requestClose() {
    if (saving) return;
    if (isDirty && !window.confirm("저장하지 않은 변경 사항을 버릴까요?")) return;
    onClose();
  }

  async function reviewChanges(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    const nextValidationMessage = validateControls(event.currentTarget);
    setValidationMessage(nextValidationMessage);
    if (nextValidationMessage) return;
    if (mode === "create" && draft.scope === "unclassified") {
      setValidationMessage("회사 업무인지 개인 업무인지 선택해 주세요.");
      return;
    }

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

  const activeScenarioCopy = scenarioCopy[createScenario];
  const showCurrentState = createScenario !== "planned";
  const currentStateRequired =
    createScenario === "in_progress" || createScenario === "completed";
  const showDesiredOutcomes = createScenario !== "in_progress";
  const showNextSteps = createScenario === "in_progress";
  const showRisks = createScenario === "in_progress";
  const showCompletedVerification = createScenario === "completed";

  return (
    <EditorDialog
      eyebrow={mode === "create" ? "새 업무" : "업무 편집"}
      title={mode === "create" ? "새 업무 만들기" : snapshot?.work_item.title ?? workItemId}
      titleId="work-item-editor-title"
      closeLabel="편집기 닫기"
      closeDisabled={saving}
      onRequestClose={requestClose}
    >
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
                <Button size="sm" variant="secondary" onClick={reloadLatest}>
                  최신 내용 다시 불러오기
                </Button>
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
            <form
              className="editor-form"
              noValidate
              onSubmit={reviewChanges}
              onChange={(event) => {
                clearControlValidation(event.target);
                setValidationMessage(null);
                setError(null);
              }}
            >
              {validationMessage ? (
                <div className="editor-inline-validation" role="alert">
                  {validationMessage}
                </div>
              ) : null}
              {mode === "create" ? (
                <>
                  <section className="editor-section editor-scenario-section" aria-labelledby="create-scenario-title">
                    <div className="editor-section-heading">
                      <div>
                        <span className="eyebrow">시작 단계</span>
                        <h3 id="create-scenario-title">업무는 어느 단계에 있나요?</h3>
                      </div>
                    </div>
                    <p className="editor-section-description">
                      현재 단계에 맞는 입력 항목만 보여드립니다.
                    </p>
                    <div className="editor-scenario-grid">
                      {createScenarioOptions.map((option) => {
                        const Icon = option.icon;
                        const selected = createScenario === option.value;
                        return (
                          <button
                            aria-pressed={selected}
                            className={selected ? "selected" : ""}
                            key={option.value}
                            onClick={() => changeCreateScenario(option.value)}
                            type="button"
                          >
                            <Icon aria-hidden="true" size={17} strokeWidth={1.8} />
                            <span>
                              <strong>{option.label}</strong>
                              <small>{option.description}</small>
                            </span>
                          </button>
                        );
                      })}
                    </div>
                  </section>

                  <section className="editor-section" aria-labelledby="editor-core-title">
                    <div className="editor-section-heading">
                      <div>
                        <span className="eyebrow">필수 정보</span>
                        <h3 id="editor-core-title">업무 기본 정보</h3>
                      </div>
                    </div>
                    <div className="editor-grid">
                      <label className="editor-field full">
                        <span>제목</span>
                        <input
                          required
                          maxLength={160}
                          value={draft.title}
                          placeholder="업무를 한 문장으로 적어 주세요."
                          onChange={(event) => setDraft({ ...draft, title: event.target.value })}
                        />
                      </label>
                      <div className="editor-field">
                        <span>프로젝트</span>
                        <ProjectCombobox
                          onChange={(projectId) => setDraft({ ...draft, projectId })}
                          options={projectOptions}
                          value={draft.projectId}
                        />
                        <small>기존 프로젝트를 선택하거나 새로 입력하세요.</small>
                      </div>
                      <div className="editor-field">
                        <span>상태</span>
                        <SelectMenu
                          ariaLabel="업무 상태"
                          className="editor-select-menu"
                          onChange={(status) => setDraft({ ...draft, status })}
                          options={statusOptions}
                          value={draft.status}
                        />
                        <small>시작 단계에 따라 자동 설정됩니다.</small>
                      </div>
                      <div className="editor-field">
                        <span>업무 범위</span>
                        <SelectMenu
                          ariaLabel="업무 범위"
                          className="editor-select-menu"
                          onChange={(scope) => setDraft({ ...draft, scope })}
                          options={scopeOptions}
                          value={draft.scope}
                        />
                        <small>회사 보고와 개인 기록을 분리하는 기준입니다.</small>
                      </div>
                      <div className="editor-field">
                        <span>주간 보고 처리</span>
                        <SelectMenu
                          ariaLabel="주간 보고 처리"
                          className="editor-select-menu"
                          onChange={(reportingMode) => setDraft({ ...draft, reportingMode })}
                          options={reportingModeOptions}
                          value={draft.reportingMode}
                        />
                        <small>지원 활동과 제외 항목은 기본 보고서의 독립 업무로 나오지 않습니다.</small>
                      </div>
                      {draft.reportingMode !== "primary" ? (
                        <label className="editor-field full">
                          <span>보고에서 분리하는 이유 · 선택</span>
                          <input
                            maxLength={240}
                            value={draft.exclusionReason}
                            placeholder="예: 기능 업무에 포함되는 브랜치 동기화 작업"
                            onChange={(event) =>
                              setDraft({ ...draft, exclusionReason: event.target.value })
                            }
                          />
                        </label>
                      ) : null}
                      <label className="editor-field full">
                        <span>{activeScenarioCopy.objectiveLabel}</span>
                        <textarea
                          required
                          rows={3}
                          value={draft.objective}
                          placeholder={activeScenarioCopy.objectivePlaceholder}
                          onChange={(event) => setDraft({ ...draft, objective: event.target.value })}
                        />
                      </label>
                      {showCurrentState && (
                        <label className="editor-field full">
                          <span>{activeScenarioCopy.currentLabel}</span>
                          <textarea
                            required={currentStateRequired}
                            rows={3}
                            value={draft.currentState}
                            placeholder={activeScenarioCopy.currentPlaceholder}
                            onChange={(event) =>
                              setDraft({ ...draft, currentState: event.target.value })
                            }
                          />
                        </label>
                      )}
                      {showDesiredOutcomes && (
                        <label className="editor-field full">
                          <span>{createScenario === "completed" ? "결과" : "원하는 결과"}</span>
                          <textarea
                            rows={3}
                            value={draft.desiredOutcomes}
                            placeholder={
                              createScenario === "completed"
                                ? "달라진 점이나 만들어진 결과를 한 줄에 하나씩 적어 주세요."
                                : "완료 여부를 판단할 결과를 한 줄에 하나씩 적어 주세요."
                            }
                            onChange={(event) =>
                              setDraft({ ...draft, desiredOutcomes: event.target.value })
                            }
                          />
                        </label>
                      )}
                      {showNextSteps && (
                        <label className="editor-field">
                          <span>다음 단계</span>
                          <textarea
                            rows={3}
                            value={draft.nextSteps}
                            placeholder="이어서 할 일을 한 줄에 하나씩"
                            onChange={(event) => setDraft({ ...draft, nextSteps: event.target.value })}
                          />
                        </label>
                      )}
                      {showRisks && (
                        <label className="editor-field">
                          <span>위험·막힘</span>
                          <textarea
                            rows={3}
                            value={draft.risks}
                            placeholder="진행을 막는 조건이나 우려 사항"
                            onChange={(event) => setDraft({ ...draft, risks: event.target.value })}
                          />
                        </label>
                      )}
                      {showCompletedVerification && (
                        <label className="editor-field full">
                          <span>확인한 내용</span>
                          <textarea
                            rows={3}
                            value={draft.verificationCompleted}
                            placeholder="완료를 확인한 방법이나 검증 결과를 한 줄에 하나씩"
                            onChange={(event) =>
                              setDraft({ ...draft, verificationCompleted: event.target.value })
                            }
                          />
                        </label>
                      )}
                    </div>
                  </section>

                  <details className="editor-disclosure">
                    <summary>
                      <ChevronRight
                        aria-hidden="true"
                        className="disclosure-chevron"
                        size={15}
                        strokeWidth={2}
                      />
                      <span>
                        <strong>추가 맥락</strong>
                        <small>분류·결정·검증 등 선택 항목</small>
                      </span>
                    </summary>
                    <div className="editor-disclosure-content editor-grid">
                      <label className="editor-field">
                        <span>업무 ID (자동)</span>
                        <input
                          required
                          minLength={2}
                          maxLength={64}
                          pattern="[A-Za-z0-9][A-Za-z0-9._-]+"
                          value={draft.id}
                          placeholder="work-260715-1300"
                          onChange={(event) => {
                            setIdEdited(true);
                            setDraft({ ...draft, id: event.target.value });
                          }}
                        />
                        <small>자동 생성되며 필요한 경우에만 수정하세요.</small>
                      </label>
                      <label className="editor-field">
                        <span>상위 업무 ID</span>
                        <input
                          value={draft.initiativeId}
                          placeholder="연결할 상위 업무가 있다면 입력"
                          onChange={(event) =>
                            setDraft({ ...draft, initiativeId: event.target.value })
                          }
                        />
                      </label>
                      <div className="editor-field">
                        <span>업무 유형</span>
                        <TokenInput
                          ariaLabel="업무 유형 추가"
                          onChange={(workTypes) => setDraft({ ...draft, workTypes })}
                          placeholder="예: feature"
                          value={draft.workTypes}
                        />
                        <small>Enter 또는 쉼표로 항목을 추가합니다.</small>
                      </div>
                      <div className="editor-field">
                        <span>태그</span>
                        <TokenInput
                          ariaLabel="태그 추가"
                          onChange={(tags) => setDraft({ ...draft, tags })}
                          placeholder="예: desktop"
                          value={draft.tags}
                        />
                        <small>검색이나 분류에 쓸 단어만 추가하세요.</small>
                      </div>
                      {!showDesiredOutcomes && (
                        <label className="editor-field full">
                          <span>원하는 결과</span>
                          <textarea
                            rows={3}
                            value={draft.desiredOutcomes}
                            placeholder="완료되었을 때 확인할 결과를 한 줄에 하나씩"
                            onChange={(event) =>
                              setDraft({ ...draft, desiredOutcomes: event.target.value })
                            }
                          />
                        </label>
                      )}
                      <label className="editor-field full">
                        <span>결정</span>
                        <textarea
                          rows={3}
                          value={draft.decisions}
                          placeholder="중요한 선택과 이유를 한 줄에 하나씩"
                          onChange={(event) => setDraft({ ...draft, decisions: event.target.value })}
                        />
                      </label>
                      {!showCompletedVerification && (
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
                      )}
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
                      {!showNextSteps && (
                        <label className="editor-field">
                          <span>다음 단계</span>
                          <textarea
                            rows={4}
                            value={draft.nextSteps}
                            placeholder="이어서 할 일을 한 줄에 하나씩"
                            onChange={(event) => setDraft({ ...draft, nextSteps: event.target.value })}
                          />
                        </label>
                      )}
                      {!showRisks && (
                        <label className="editor-field">
                          <span>위험·막힘</span>
                          <textarea
                            rows={4}
                            value={draft.risks}
                            placeholder="진행을 막는 조건이나 우려 사항"
                            onChange={(event) => setDraft({ ...draft, risks: event.target.value })}
                          />
                        </label>
                      )}
                    </div>
                  </details>
                </>
              ) : (
                <>
                  <section className="editor-section" aria-labelledby="editor-identity-title">
                    <div className="editor-section-heading">
                      <div>
                        <span className="eyebrow">기본 정보</span>
                        <h3 id="editor-identity-title">업무 정의</h3>
                      </div>
                      <span className="editor-preserved">ID와 프로젝트는 유지됩니다</span>
                    </div>
                    <div className="editor-grid">
                      <label className="editor-field">
                        <span>업무 ID</span>
                        <input disabled value={draft.id} />
                      </label>
                      <label className="editor-field">
                        <span>프로젝트</span>
                        <input disabled value={draft.projectId} />
                      </label>
                      <label className="editor-field full">
                        <span>제목</span>
                        <input
                          required
                          maxLength={160}
                          value={draft.title}
                          onChange={(event) => setDraft({ ...draft, title: event.target.value })}
                        />
                      </label>
                      <div className="editor-field">
                        <span>상태</span>
                        <SelectMenu
                          ariaLabel="업무 상태"
                          className="editor-select-menu"
                          onChange={(status) => setDraft({ ...draft, status })}
                          options={statusOptions}
                          value={draft.status}
                        />
                      </div>
                      <div className="editor-field">
                        <span>업무 범위</span>
                        <SelectMenu
                          ariaLabel="업무 범위"
                          className="editor-select-menu"
                          onChange={(scope) => setDraft({ ...draft, scope })}
                          options={scopeOptions}
                          value={draft.scope}
                        />
                      </div>
                      <div className="editor-field">
                        <span>주간 보고 처리</span>
                        <SelectMenu
                          ariaLabel="주간 보고 처리"
                          className="editor-select-menu"
                          onChange={(reportingMode) => setDraft({ ...draft, reportingMode })}
                          options={reportingModeOptions}
                          value={draft.reportingMode}
                        />
                      </div>
                      {draft.reportingMode !== "primary" ? (
                        <label className="editor-field">
                          <span>보고에서 분리하는 이유 · 선택</span>
                          <input
                            maxLength={240}
                            value={draft.exclusionReason}
                            placeholder="예: 기능 업무를 지원한 운영 작업"
                            onChange={(event) =>
                              setDraft({ ...draft, exclusionReason: event.target.value })
                            }
                          />
                        </label>
                      ) : null}
                      <label className="editor-field">
                        <span>Initiative ID</span>
                        <input
                          value={draft.initiativeId}
                          placeholder="선택 사항"
                          onChange={(event) =>
                            setDraft({ ...draft, initiativeId: event.target.value })
                          }
                        />
                      </label>
                      <label className="editor-field full">
                        <span>목표</span>
                        <textarea
                          required
                          rows={3}
                          value={draft.objective}
                          onChange={(event) => setDraft({ ...draft, objective: event.target.value })}
                        />
                      </label>
                      <label className="editor-field full">
                        <span>원하는 결과</span>
                        <textarea
                          rows={3}
                          value={draft.desiredOutcomes}
                          placeholder="한 줄에 하나씩 입력"
                          onChange={(event) =>
                            setDraft({ ...draft, desiredOutcomes: event.target.value })
                          }
                        />
                      </label>
                      <div className="editor-field">
                        <span>업무 유형</span>
                        <TokenInput
                          ariaLabel="업무 유형 추가"
                          onChange={(workTypes) => setDraft({ ...draft, workTypes })}
                          placeholder="예: feature"
                          value={draft.workTypes}
                        />
                      </div>
                      <div className="editor-field">
                        <span>태그</span>
                        <TokenInput
                          ariaLabel="태그 추가"
                          onChange={(tags) => setDraft({ ...draft, tags })}
                          placeholder="예: desktop"
                          value={draft.tags}
                        />
                      </div>
                    </div>
                  </section>

                  <section className="editor-section" aria-labelledby="editor-context-title">
                    <div className="editor-section-heading">
                      <div>
                        <span className="eyebrow">맥락</span>
                        <h3 id="editor-context-title">이어가기 위한 정보</h3>
                      </div>
                      <span className="editor-preserved">파일·Git·저장소·링크는 그대로 보존됩니다</span>
                    </div>
                    <div className="editor-grid">
                      <label className="editor-field full">
                        <span>현재 상태</span>
                        <textarea
                          required
                          rows={4}
                          value={draft.currentState}
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
                          onChange={(event) => setDraft({ ...draft, nextSteps: event.target.value })}
                        />
                      </label>
                      <label className="editor-field">
                        <span>위험·막힘</span>
                        <textarea
                          rows={4}
                          value={draft.risks}
                          onChange={(event) => setDraft({ ...draft, risks: event.target.value })}
                        />
                      </label>
                    </div>
                  </section>
                </>
              )}

              <footer className="editor-footer">
                <div className="editor-footer-copy" aria-live="polite">
                  {mode === "edit" && !hasChanges
                    ? "변경 사항이 없습니다."
                    : "저장 전에 변경 내용을 확인합니다."}
                </div>
                <Button size="sm" variant="ghost" onClick={requestClose}>
                  취소
                </Button>
                <Button type="submit" size="sm" variant="primary" disabled={!hasChanges}>
                  {mode === "create" ? "검토하기" : "변경 검토"}
                </Button>
              </footer>
            </form>
          )}
        </div>
    </EditorDialog>
  );
}
