import { useEffect, useRef, useState, type FormEvent } from "react";
import {
  captureCheckpoint,
  desktopWriteError,
  getWorkItemEditSnapshot,
  previewCaptureCheckpoint,
  type CheckpointInput,
  type CheckpointKind,
  type CheckpointWritePreview,
  type DesktopWriteError,
  type WorkItemEditSnapshot,
  type WorkItemStatus,
} from "./desktop";
import { WriteDiffReview } from "./WriteDiffReview";
import {
  CheckpointStepper,
  type CheckpointStep,
} from "./features/checkpoint/CheckpointStepper";
import { Button } from "./ui/Button";
import { EditorDialog } from "./ui/EditorDialog";

interface CheckpointEditorProps {
  workItemId: string;
  onClose: () => void;
  onSaved: (workItemId: string) => Promise<void>;
}

interface CheckpointDraft {
  kind: CheckpointKind;
  capturedAt: string;
  timezone: string;
  workStart: string;
  workEnd: string;
  title: string;
  summary: string;
  statusAfter: WorkItemStatus;
  activities: string;
  decisionSummary: string;
  decisionRationale: string;
  decisionStatus: "proposed" | "accepted" | "superseded";
  verificationType: "test" | "build" | "lint" | "manual" | "measurement" | "review" | "other";
  verificationDescription: string;
  verificationStatus: "passed" | "failed" | "partial" | "not_run";
  verificationCommand: string;
  verificationEvidence: string;
  outcomes: string;
  outcomeImpact: string;
  blockers: string;
  nextSteps: string;
  evidenceCommits: string;
  evidencePullRequests: string;
  evidenceIssues: string;
  evidenceFiles: string;
  evidenceCommands: string;
  evidenceUrls: string;
  correctionOf: string;
  confidentiality: "normal" | "sensitive" | "restricted";
  currentState: string;
  contextDecisions: string;
  contextVerificationCompleted: string;
  contextVerificationPending: string;
  contextRisks: string;
}

interface PendingCheckpointCommit {
  input: CheckpointInput;
  expected: WorkItemEditSnapshot["revisions"];
  now: string;
}

const checkpointSteps: CheckpointStep[] = [
  { label: "요약", description: "범위와 진행한 작업" },
  { label: "결과·검증", description: "결정과 확인 결과" },
  { label: "근거", description: "커밋·파일·URL" },
  { label: "Context", description: "다음 작업과 handoff" },
];

const finalStep = checkpointSteps.length - 1;

function pad(value: number): string {
  return String(value).padStart(2, "0");
}

function localDateTime(date: Date): string {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function splitLines(value: string): string[] {
  return value
    .split("\n")
    .map((item) => item.trim())
    .filter(Boolean);
}

function formatLines(values: string[]): string {
  return values.join("\n");
}

function initialDraft(snapshot: WorkItemEditSnapshot): CheckpointDraft {
  const now = new Date();
  const capturedAt = localDateTime(now);
  const workDate = capturedAt.slice(0, 10);
  return {
    kind: "progress",
    capturedAt,
    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone || "Asia/Seoul",
    workStart: workDate,
    workEnd: workDate,
    title: "",
    summary: "",
    statusAfter: snapshot.work_item.status,
    activities: "",
    decisionSummary: "",
    decisionRationale: "",
    decisionStatus: "accepted",
    verificationType: "test",
    verificationDescription: "",
    verificationStatus: "passed",
    verificationCommand: "",
    verificationEvidence: "",
    outcomes: "",
    outcomeImpact: "",
    blockers: "",
    nextSteps: formatLines(snapshot.context.next_steps),
    evidenceCommits: "",
    evidencePullRequests: "",
    evidenceIssues: "",
    evidenceFiles: "",
    evidenceCommands: "",
    evidenceUrls: "",
    correctionOf: "",
    confidentiality: "normal",
    currentState: snapshot.context.current_state,
    contextDecisions: formatLines(snapshot.context.decisions),
    contextVerificationCompleted: formatLines(snapshot.context.verification.completed),
    contextVerificationPending: formatLines(snapshot.context.verification.pending),
    contextRisks: formatLines(snapshot.context.risks),
  };
}

function checkpointInput(draft: CheckpointDraft, workItemId: string): CheckpointInput {
  const command = draft.verificationCommand.trim();
  const evidenceCommands = splitLines(draft.evidenceCommands);
  if (command && !evidenceCommands.includes(command)) evidenceCommands.push(command);
  const decisionSummary = draft.decisionSummary.trim();
  const decisionRationale = draft.decisionRationale.trim();
  const verificationDescription = draft.verificationDescription.trim();
  const impact = draft.outcomeImpact.trim();
  const outcomeEvidence = splitLines(draft.verificationEvidence);

  return {
    work_item_id: workItemId,
    kind: draft.kind,
    source: {
      agent: "manual",
      surface: "desktop",
      task_title: draft.title.trim(),
      session_ref: null,
    },
    captured_at: new Date(draft.capturedAt).toISOString(),
    work_period: {
      start: draft.workStart,
      end: draft.workEnd,
      precision: draft.workStart === draft.workEnd ? "day" : "range",
      basis: draft.kind === "backfill" ? ["checkpoint", "user"] : ["checkpoint"],
      timezone: draft.timezone.trim(),
    },
    title: draft.title.trim(),
    summary: draft.summary.trim(),
    status_after: draft.kind === "final" ? "completed" : draft.statusAfter,
    activities: splitLines(draft.activities),
    decisions:
      decisionSummary || decisionRationale
        ? [
            {
              summary: decisionSummary,
              rationale: decisionRationale,
              status: draft.decisionStatus,
            },
          ]
        : [],
    verifications: verificationDescription
      ? [
          {
            type: draft.verificationType,
            description: verificationDescription,
            status: draft.verificationStatus,
            command: command || null,
            evidence_refs: splitLines(draft.verificationEvidence),
          },
        ]
      : [],
    outcomes: splitLines(draft.outcomes).map((description) => ({
      description,
      impact: impact || null,
      evidence_refs: outcomeEvidence,
    })),
    blockers: splitLines(draft.blockers),
    next_steps: splitLines(draft.nextSteps),
    evidence: {
      commits: splitLines(draft.evidenceCommits),
      pull_requests: splitLines(draft.evidencePullRequests),
      issues: splitLines(draft.evidenceIssues),
      files: splitLines(draft.evidenceFiles),
      commands: evidenceCommands,
      urls: splitLines(draft.evidenceUrls),
    },
    related_checkpoint_ids: [],
    correction_of: draft.kind === "correction" ? draft.correctionOf.trim() || null : null,
    confidentiality: draft.confidentiality,
    context_update: {
      current_state: draft.currentState.trim(),
      decisions: splitLines(draft.contextDecisions),
      verification: {
        completed: splitLines(draft.contextVerificationCompleted),
        pending: splitLines(draft.contextVerificationPending),
      },
      next_steps: splitLines(draft.nextSteps),
      risks: splitLines(draft.contextRisks),
    },
  };
}

function errorTitle(error: DesktopWriteError): string {
  switch (error.kind) {
    case "revision_conflict":
      return "업무 파일이 기록 도중 변경되었습니다.";
    case "create_conflict":
      return "같은 ID의 체크포인트가 이미 있습니다.";
    case "lock_busy":
      return "다른 writer가 저장 중입니다.";
    case "validation":
      return "기록 내용을 확인해 주세요.";
    case "not_found":
      return "업무 파일을 찾을 수 없습니다.";
    default:
      return "체크포인트를 저장하지 못했습니다.";
  }
}

export function CheckpointEditor({ workItemId, onClose, onSaved }: CheckpointEditorProps) {
  const [snapshot, setSnapshot] = useState<WorkItemEditSnapshot | null>(null);
  const [draft, setDraft] = useState<CheckpointDraft | null>(null);
  const [initialDraftJson, setInitialDraftJson] = useState("");
  const [preview, setPreview] = useState<CheckpointWritePreview | null>(null);
  const [pendingCommit, setPendingCommit] = useState<PendingCheckpointCommit | null>(null);
  const [error, setError] = useState<DesktopWriteError | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [loadVersion, setLoadVersion] = useState(0);
  const [activeStep, setActiveStep] = useState(0);
  const [furthestStep, setFurthestStep] = useState(0);
  const activeStepRef = useRef<HTMLDivElement>(null);

  const isDirty = draft !== null && JSON.stringify(draft) !== initialDraftJson;

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    setPreview(null);
    setPendingCommit(null);
    setActiveStep(0);
    setFurthestStep(0);
    getWorkItemEditSnapshot(workItemId)
      .then((nextSnapshot) => {
        if (cancelled) return;
        const nextDraft = initialDraft(nextSnapshot);
        setSnapshot(nextSnapshot);
        setDraft(nextDraft);
        setInitialDraftJson(JSON.stringify(nextDraft));
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
  }, [loadVersion, workItemId]);

  function requestClose() {
    if (saving) return;
    if (isDirty && !window.confirm("저장하지 않은 체크포인트를 버릴까요?")) return;
    onClose();
  }

  function validateActiveStep() {
    const controls = activeStepRef.current?.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>(
      "input, select, textarea",
    );
    if (!controls) return true;
    for (const control of controls) {
      if (!control.checkValidity()) {
        control.reportValidity();
        control.focus();
        return false;
      }
    }
    return true;
  }

  function advanceStep() {
    if (!validateActiveStep()) return;
    const nextStep = Math.min(activeStep + 1, finalStep);
    setActiveStep(nextStep);
    setFurthestStep((current) => Math.max(current, nextStep));
  }

  function selectStep(step: number) {
    if (step > activeStep && !validateActiveStep()) return;
    setActiveStep(step);
  }

  async function reviewChanges(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!draft || !snapshot) return;
    if (activeStep < finalStep) {
      advanceStep();
      return;
    }
    setError(null);
    const now = new Date().toISOString();
    try {
      const input = checkpointInput(draft, workItemId);
      const nextPreview = await previewCaptureCheckpoint(input, snapshot.revisions, now);
      setPreview(nextPreview);
      setPendingCommit({
        input: { ...input, id: nextPreview.checkpoint.id },
        expected: snapshot.revisions,
        now,
      });
    } catch (nextError) {
      setError(desktopWriteError(nextError));
    }
  }

  async function commitChanges() {
    if (!pendingCommit) return;
    setSaving(true);
    setError(null);
    try {
      const result = await captureCheckpoint(
        pendingCommit.input,
        pendingCommit.expected,
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

  return (
    <EditorDialog
      eyebrow="작업 기록"
      title={snapshot ? `${snapshot.work_item.title} · 체크포인트` : "체크포인트 추가"}
      titleId="checkpoint-editor-title"
      closeLabel="기록기 닫기"
      closeDisabled={saving}
      onRequestClose={requestClose}
      wide
    >
        <div className="editor-body">
          {error ? (
            <div className={`editor-alert ${error.kind}`} role="alert">
              <div>
                <strong>{errorTitle(error)}</strong>
                <p>
                  {error.kind === "revision_conflict"
                    ? "현재 입력을 최신 Context로 교체한 뒤 다시 기록할 수 있습니다. 기존 파일은 덮어쓰지 않았습니다."
                    : error.kind === "lock_busy"
                      ? "잠시 후 변경 검토 또는 저장을 다시 시도해 주세요."
                      : error.message}
                </p>
                <details>
                  <summary>기술 상세</summary>
                  <code>{error.message}</code>
                </details>
              </div>
              {error.kind === "revision_conflict" ? (
                <Button size="sm" variant="secondary" onClick={() => setLoadVersion((value) => value + 1)}>
                  최신 Context 다시 불러오기
                </Button>
              ) : null}
            </div>
          ) : null}

          {loading || !draft ? (
            <div className="editor-loading" role="status">
              기록할 업무 Context를 불러오는 중…
            </div>
          ) : preview ? (
            <WriteDiffReview
              eyebrow="체크포인트 저장 전 검토"
              title={preview.checkpoint.title}
              identity={preview.checkpoint.id}
              status={preview.checkpoint.status_after}
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
              <CheckpointStepper
                activeStep={activeStep}
                furthestStep={furthestStep}
                steps={checkpointSteps}
                onSelect={selectStep}
              />
              <div className="checkpoint-step-content" ref={activeStepRef}>
              {activeStep === 0 ? (
                <section className="editor-section" aria-labelledby="checkpoint-summary-title">
                <div className="editor-section-heading">
                  <div>
                    <span className="eyebrow">Checkpoint</span>
                    <h3 id="checkpoint-summary-title">기록 범위와 요약</h3>
                  </div>
                  <span className="editor-preserved">ID는 preview에서 안전하게 생성됩니다</span>
                </div>
                <div className="editor-grid">
                  <label className="editor-field">
                    <span>기록 유형</span>
                    <select
                      value={draft.kind}
                      onChange={(event) => {
                        const kind = event.target.value as CheckpointKind;
                        setDraft({ ...draft, kind });
                      }}
                    >
                      <option value="progress">진행 기록</option>
                      <option value="final">최종 완료</option>
                      <option value="backfill">사후 기록</option>
                      <option value="correction">정정 기록</option>
                    </select>
                  </label>
                  <label className="editor-field">
                    <span>기록 후 상태</span>
                    <select
                      value={draft.kind === "final" ? "completed" : draft.statusAfter}
                      disabled={draft.kind === "final"}
                      onChange={(event) => setDraft({ ...draft, statusAfter: event.target.value as WorkItemStatus })}
                    >
                      <option value="planned">계획</option>
                      <option value="in_progress">진행 중</option>
                      <option value="blocked">막힘</option>
                      <option value="completed">완료</option>
                      <option value="cancelled">취소</option>
                    </select>
                  </label>
                  <label className="editor-field">
                    <span>기록 시각</span>
                    <input
                      required
                      type="datetime-local"
                      value={draft.capturedAt}
                      onChange={(event) => setDraft({ ...draft, capturedAt: event.target.value })}
                    />
                  </label>
                  <label className="editor-field">
                    <span>Timezone</span>
                    <input
                      required
                      value={draft.timezone}
                      placeholder="Asia/Seoul"
                      onChange={(event) => setDraft({ ...draft, timezone: event.target.value })}
                    />
                  </label>
                  <label className="editor-field">
                    <span>작업 시작일</span>
                    <input required type="date" value={draft.workStart} onChange={(event) => setDraft({ ...draft, workStart: event.target.value })} />
                  </label>
                  <label className="editor-field">
                    <span>작업 종료일</span>
                    <input required type="date" value={draft.workEnd} onChange={(event) => setDraft({ ...draft, workEnd: event.target.value })} />
                  </label>
                  {draft.kind === "correction" ? (
                    <label className="editor-field full">
                      <span>정정 대상 체크포인트 ID</span>
                      <input required value={draft.correctionOf} onChange={(event) => setDraft({ ...draft, correctionOf: event.target.value })} />
                    </label>
                  ) : null}
                  <label className="editor-field full">
                    <span>제목</span>
                    <input required maxLength={160} value={draft.title} placeholder="이번 작업 단계를 한 문장으로" onChange={(event) => setDraft({ ...draft, title: event.target.value })} />
                  </label>
                  <label className="editor-field full">
                    <span>요약</span>
                    <textarea required rows={3} value={draft.summary} placeholder="무엇이 달라졌고 현재 어디까지 왔나요?" onChange={(event) => setDraft({ ...draft, summary: event.target.value })} />
                  </label>
                  <label className="editor-field full">
                    <span>진행한 작업</span>
                    <textarea required rows={4} value={draft.activities} placeholder="한 줄에 하나씩 입력" onChange={(event) => setDraft({ ...draft, activities: event.target.value })} />
                  </label>
                </div>
                </section>
              ) : null}

              {activeStep === 1 ? (
                <section className="editor-section" aria-labelledby="checkpoint-evidence-title">
                <div className="editor-section-heading">
                  <div>
                    <span className="eyebrow">Evidence</span>
                    <h3 id="checkpoint-evidence-title">결정·검증·결과</h3>
                  </div>
                  <span className="editor-preserved">비어 있는 구조화 항목은 기록에서 제외됩니다</span>
                </div>
                <div className="editor-grid">
                  <label className="editor-field">
                    <span>결정</span>
                    <input value={draft.decisionSummary} placeholder="결정한 내용" onChange={(event) => setDraft({ ...draft, decisionSummary: event.target.value })} />
                  </label>
                  <label className="editor-field">
                    <span>결정 상태</span>
                    <select value={draft.decisionStatus} onChange={(event) => setDraft({ ...draft, decisionStatus: event.target.value as CheckpointDraft["decisionStatus"] })}>
                      <option value="accepted">채택</option>
                      <option value="proposed">제안</option>
                      <option value="superseded">대체됨</option>
                    </select>
                  </label>
                  <label className="editor-field full">
                    <span>결정 이유</span>
                    <textarea rows={2} required={draft.decisionSummary.trim().length > 0} value={draft.decisionRationale} placeholder="왜 이 결정을 내렸나요?" onChange={(event) => setDraft({ ...draft, decisionRationale: event.target.value })} />
                  </label>
                  <label className="editor-field">
                    <span>검증 유형</span>
                    <select value={draft.verificationType} onChange={(event) => setDraft({ ...draft, verificationType: event.target.value as CheckpointDraft["verificationType"] })}>
                      <option value="test">테스트</option>
                      <option value="build">빌드</option>
                      <option value="lint">Lint</option>
                      <option value="manual">수동 확인</option>
                      <option value="measurement">측정</option>
                      <option value="review">리뷰</option>
                      <option value="other">기타</option>
                    </select>
                  </label>
                  <label className="editor-field">
                    <span>검증 상태</span>
                    <select value={draft.verificationStatus} onChange={(event) => setDraft({ ...draft, verificationStatus: event.target.value as CheckpointDraft["verificationStatus"] })}>
                      <option value="passed">통과</option>
                      <option value="failed">실패</option>
                      <option value="partial">부분 통과</option>
                      <option value="not_run">미실행</option>
                    </select>
                  </label>
                  <label className="editor-field full">
                    <span>검증 설명</span>
                    <input value={draft.verificationDescription} placeholder="어떤 검증을 수행했나요?" onChange={(event) => setDraft({ ...draft, verificationDescription: event.target.value })} />
                  </label>
                  <label className="editor-field">
                    <span>검증 명령</span>
                    <input value={draft.verificationCommand} placeholder="pnpm check:all" onChange={(event) => setDraft({ ...draft, verificationCommand: event.target.value })} />
                  </label>
                  <label className="editor-field">
                    <span>검증 근거 경로</span>
                    <textarea rows={2} value={draft.verificationEvidence} placeholder="한 줄에 하나씩" onChange={(event) => setDraft({ ...draft, verificationEvidence: event.target.value })} />
                  </label>
                  <label className="editor-field full">
                    <span>결과</span>
                    <textarea required={draft.kind === "final"} rows={3} value={draft.outcomes} placeholder="한 줄에 하나씩 입력 · 최종 완료에는 하나 이상 필요" onChange={(event) => setDraft({ ...draft, outcomes: event.target.value })} />
                  </label>
                  <label className="editor-field full">
                    <span>결과의 영향</span>
                    <input value={draft.outcomeImpact} placeholder="모든 결과에 공통으로 적용할 영향 설명" onChange={(event) => setDraft({ ...draft, outcomeImpact: event.target.value })} />
                  </label>
                  <label className="editor-field">
                    <span>차단 요소</span>
                    <textarea rows={3} value={draft.blockers} placeholder="한 줄에 하나씩" onChange={(event) => setDraft({ ...draft, blockers: event.target.value })} />
                  </label>
                  <label className="editor-field">
                    <span>다음 작업</span>
                    <textarea rows={3} value={draft.nextSteps} placeholder="체크포인트와 Context에 함께 반영" onChange={(event) => setDraft({ ...draft, nextSteps: event.target.value })} />
                  </label>
                </div>
                </section>
              ) : null}

              {activeStep === 2 ? (
                <section className="editor-section" aria-labelledby="checkpoint-proof-title">
                <div className="editor-section-heading">
                  <div>
                    <span className="eyebrow">References</span>
                    <h3 id="checkpoint-proof-title">연결할 근거</h3>
                  </div>
                  <span className="editor-preserved">각 항목을 한 줄에 하나씩 입력합니다</span>
                </div>
                <div className="editor-grid checkpoint-evidence-grid">
                  <label className="editor-field"><span>커밋</span><textarea rows={2} value={draft.evidenceCommits} onChange={(event) => setDraft({ ...draft, evidenceCommits: event.target.value })} /></label>
                  <label className="editor-field"><span>PR</span><textarea rows={2} value={draft.evidencePullRequests} onChange={(event) => setDraft({ ...draft, evidencePullRequests: event.target.value })} /></label>
                  <label className="editor-field"><span>이슈</span><textarea rows={2} value={draft.evidenceIssues} onChange={(event) => setDraft({ ...draft, evidenceIssues: event.target.value })} /></label>
                  <label className="editor-field"><span>파일</span><textarea rows={2} value={draft.evidenceFiles} onChange={(event) => setDraft({ ...draft, evidenceFiles: event.target.value })} /></label>
                  <label className="editor-field"><span>명령</span><textarea rows={2} value={draft.evidenceCommands} onChange={(event) => setDraft({ ...draft, evidenceCommands: event.target.value })} /></label>
                  <label className="editor-field"><span>URL</span><textarea rows={2} value={draft.evidenceUrls} onChange={(event) => setDraft({ ...draft, evidenceUrls: event.target.value })} /></label>
                </div>
                </section>
              ) : null}

              {activeStep === 3 ? (
                <section className="editor-section" aria-labelledby="checkpoint-context-title">
                <div className="editor-section-heading">
                  <div>
                    <span className="eyebrow">Handoff</span>
                    <h3 id="checkpoint-context-title">저장 후 Context</h3>
                  </div>
                  <span className="editor-preserved">파일·Git 기준점은 현재 값이 보존됩니다</span>
                </div>
                <div className="editor-grid">
                  <label className="editor-field full"><span>현재 상태</span><textarea required rows={4} value={draft.currentState} onChange={(event) => setDraft({ ...draft, currentState: event.target.value })} /></label>
                  <label className="editor-field full"><span>누적 결정</span><textarea rows={3} value={draft.contextDecisions} placeholder="Context에 유지할 결정 · 한 줄에 하나씩" onChange={(event) => setDraft({ ...draft, contextDecisions: event.target.value })} /></label>
                  <label className="editor-field"><span>검증 완료</span><textarea rows={3} value={draft.contextVerificationCompleted} onChange={(event) => setDraft({ ...draft, contextVerificationCompleted: event.target.value })} /></label>
                  <label className="editor-field"><span>검증 대기</span><textarea rows={3} value={draft.contextVerificationPending} onChange={(event) => setDraft({ ...draft, contextVerificationPending: event.target.value })} /></label>
                  <label className="editor-field full"><span>리스크</span><textarea rows={3} value={draft.contextRisks} onChange={(event) => setDraft({ ...draft, contextRisks: event.target.value })} /></label>
                  <label className="editor-field">
                    <span>기밀 수준</span>
                    <select value={draft.confidentiality} onChange={(event) => setDraft({ ...draft, confidentiality: event.target.value as CheckpointDraft["confidentiality"] })}>
                      <option value="normal">일반</option>
                      <option value="sensitive">민감</option>
                      <option value="restricted">제한</option>
                    </select>
                  </label>
                </div>
                </section>
              ) : null}
              </div>

              <footer className="editor-footer">
                <div className="editor-footer-copy" aria-live="polite">
                  단계 {activeStep + 1}/{checkpointSteps.length} · {checkpointSteps[activeStep].description}
                </div>
                <Button type="button" size="sm" variant="ghost" onClick={requestClose}>취소</Button>
                {activeStep > 0 ? (
                  <Button type="button" size="sm" variant="ghost" onClick={() => setActiveStep((step) => step - 1)}>
                    이전
                  </Button>
                ) : null}
                {activeStep < finalStep ? (
                  <Button type="button" size="sm" variant="primary" onClick={advanceStep}>
                    다음 단계
                  </Button>
                ) : (
                  <Button type="submit" size="sm" variant="primary">5개 파일 변경 검토</Button>
                )}
              </footer>
            </form>
          )}
        </div>
    </EditorDialog>
  );
}
