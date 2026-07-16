import { useState, type FormEvent } from "react";
import {
  createPerformanceNote,
  desktopWriteError,
  previewPerformanceNote,
  type DesktopWriteError,
  type PerformanceNoteInput,
  type PerformanceNoteSourceRevision,
  type PerformanceNoteWritePreview,
} from "./desktop";
import { WriteDiffReview } from "./WriteDiffReview";
import { Button } from "./ui/Button";
import { EditorDialog } from "./ui/EditorDialog";

interface PerformanceNoteEditorProps {
  workItemId: string;
  onClose: () => void;
  onCreated: (report: string) => void;
}

interface PendingPerformanceNote {
  input: PerformanceNoteInput;
  expected: PerformanceNoteSourceRevision[];
  generatedAt: string;
}

function errorTitle(error: DesktopWriteError): string {
  switch (error.kind) {
    case "revision_conflict":
      return "성과 노트 원본이 검토 도중 변경되었습니다.";
    case "create_conflict":
      return "같은 경로의 성과 노트가 이미 있습니다.";
    case "lock_busy":
      return "다른 writer가 저장 중입니다.";
    case "validation":
      return "저장 경로 또는 원본 기록을 확인해 주세요.";
    case "not_found":
      return "업무 기록을 찾을 수 없습니다.";
    default:
      return "성과 노트를 만들지 못했습니다.";
  }
}

export function PerformanceNoteEditor({
  workItemId,
  onClose,
  onCreated,
}: PerformanceNoteEditorProps) {
  const [output, setOutput] = useState("");
  const [preview, setPreview] = useState<PerformanceNoteWritePreview | null>(null);
  const [pending, setPending] = useState<PendingPerformanceNote | null>(null);
  const [reviewedMarkdown, setReviewedMarkdown] = useState("");
  const [error, setError] = useState<DesktopWriteError | null>(null);
  const [saving, setSaving] = useState(false);
  const isDirty = output.trim().length > 0 || preview !== null;

  function requestClose() {
    if (saving) return;
    if (isDirty && !window.confirm("검토 중인 성과 노트를 닫을까요?")) return;
    onClose();
  }

  async function reviewChanges(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    const generatedAt = new Date().toISOString();
    const input: PerformanceNoteInput = {
      work_item_id: workItemId,
      output: output.trim() || null,
    };
    try {
      const nextPreview = await previewPerformanceNote(input, generatedAt);
      setPreview(nextPreview);
      setPending({
        input: { ...input, output: nextPreview.paths.report },
        expected: nextPreview.source_revisions,
        generatedAt,
      });
      setReviewedMarkdown(nextPreview.files[0]?.after ?? "");
    } catch (nextError) {
      setError(desktopWriteError(nextError));
    }
  }

  async function commitChanges() {
    if (!pending) return;
    setSaving(true);
    setError(null);
    try {
      const result = await createPerformanceNote(
        { ...pending.input, markdown: reviewedMarkdown },
        pending.expected,
        pending.generatedAt,
      );
      onCreated(result.paths.report);
    } catch (nextError) {
      const writeError = desktopWriteError(nextError);
      setError(writeError);
      if (writeError.kind === "revision_conflict" || writeError.kind === "create_conflict") {
        setPreview(null);
        setPending(null);
      }
      setSaving(false);
    }
  }

  return (
    <EditorDialog
      eyebrow="Performance note"
      title="성과 노트 초안 만들기"
      titleId="performance-note-editor-title"
      closeLabel="성과 노트 생성기 닫기"
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
                    ? "업무·Context·체크포인트 중 하나가 바뀌어 파일을 만들지 않았습니다. 최신 원본으로 다시 검토해 주세요."
                    : error.kind === "create_conflict"
                      ? "기존 초안을 보호했습니다. 다른 Markdown 경로를 입력해 새 초안을 만들 수 있습니다."
                      : error.kind === "lock_busy"
                        ? "잠시 후 변경 검토 또는 생성을 다시 시도해 주세요."
                        : error.message}
                </p>
                <details>
                  <summary>기술 상세</summary>
                  <code>{error.message}</code>
                </details>
              </div>
              {error.kind === "revision_conflict" ? (
                <Button size="sm" variant="secondary" onClick={() => setError(null)}>
                  최신 원본 다시 검토
                </Button>
              ) : null}
            </div>
          ) : null}

          {preview ? (
            <WriteDiffReview
              eyebrow="성과 노트 생성 전 검토"
              title={preview.work_item.title}
              identity={preview.paths.report}
              status={[
                `포함 ${preview.checkpoint_count}개`,
                preview.redacted_checkpoint_count > 0
                  ? `민감 ${preview.redacted_checkpoint_count}개 세부 정보 생략`
                  : null,
                preview.excluded_checkpoint_count > 0
                  ? `제한 ${preview.excluded_checkpoint_count}개 제외`
                  : null,
              ].filter(Boolean).join(" · ")}
              files={preview.files}
              saving={saving}
              defaultView="document"
              documentValue={reviewedMarkdown}
              onDocumentChange={setReviewedMarkdown}
              commitLabel="성과 노트 생성 후 열기"
              onBack={() => {
                setPreview(null);
                setPending(null);
                setReviewedMarkdown("");
              }}
              onCommit={commitChanges}
            />
          ) : (
            <form className="editor-form" onSubmit={reviewChanges} onChange={() => setError(null)}>
              <section className="editor-section" aria-labelledby="performance-note-source-title">
                <div className="editor-section-heading">
                  <div>
                    <span className="eyebrow">Source</span>
                    <h3 id="performance-note-source-title">기록에서 초안 생성</h3>
                  </div>
                  <span className="editor-preserved">기존 성과 노트는 덮어쓰지 않습니다</span>
                </div>
                <p className="editor-helper">
                  <code>{workItemId}</code>의 업무 메타데이터, 현재 Context와 모든 체크포인트를 합쳐 13개 섹션의 Markdown 초안을 만듭니다. 근거가 없는 내용은 미확인으로 남깁니다.
                </p>
                <p className="editor-policy-note">
                  민감 체크포인트는 요약·결과만 포함하고 세부 근거를 생략합니다. 제한 체크포인트는 초안에서 완전히 제외합니다.
                </p>
                <div className="editor-grid">
                  <label className="editor-field full">
                    <span>저장 경로 · 선택</span>
                    <input
                      value={output}
                      placeholder={`reports/performance-notes/${workItemId}-<마지막 작업일>.md`}
                      onChange={(event) => setOutput(event.target.value)}
                    />
                    <small>비워 두면 마지막 체크포인트 작업일로 기본 경로를 생성합니다. 데이터 폴더 내부의 .md 경로만 허용합니다.</small>
                  </label>
                </div>
              </section>

              <footer className="editor-footer">
                <div className="editor-footer-copy" aria-live="polite">
                  저장 전 실제 Markdown 전체와 원본 revision을 함께 검토합니다.
                </div>
                <Button size="sm" variant="ghost" onClick={requestClose}>
                  취소
                </Button>
                <Button type="submit" size="sm" variant="primary">
                  Markdown 초안 검토
                </Button>
              </footer>
            </form>
          )}
        </div>
    </EditorDialog>
  );
}
