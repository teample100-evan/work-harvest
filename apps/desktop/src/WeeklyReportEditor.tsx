import { useState, type FormEvent } from "react";
import {
  createWeeklyReport,
  desktopWriteError,
  previewWeeklyReport,
  type DesktopWriteError,
  type PerformanceNoteSourceRevision,
  type WeeklyReportInput,
  type WeeklyReportWritePreview,
} from "./desktop";
import { WriteDiffReview } from "./WriteDiffReview";
import { Button } from "./ui/Button";
import { EditorDialog } from "./ui/EditorDialog";
import { clearControlValidation, validateControls } from "./ui/formValidation";

interface WeeklyReportEditorProps {
  initialStartDate: string;
  initialEndDate: string;
  onClose: () => void;
  onCreated: (report: string) => void;
}

interface PendingWeeklyReport {
  input: WeeklyReportInput;
  expected: PerformanceNoteSourceRevision[];
  generatedAt: string;
}

function errorTitle(error: DesktopWriteError) {
  switch (error.kind) {
    case "revision_conflict":
      return "주간 보고서 원본이 검토 도중 변경되었습니다.";
    case "create_conflict":
      return "같은 기간의 주간 보고서가 이미 있습니다.";
    case "lock_busy":
      return "다른 writer가 저장 중입니다.";
    case "validation":
      return "기간 또는 저장 경로를 확인해 주세요.";
    default:
      return "주간 보고서를 만들지 못했습니다.";
  }
}

export function WeeklyReportEditor({
  initialStartDate,
  initialEndDate,
  onClose,
  onCreated,
}: WeeklyReportEditorProps) {
  const [startDate, setStartDate] = useState(initialStartDate);
  const [endDate, setEndDate] = useState(initialEndDate);
  const [output, setOutput] = useState("");
  const [preview, setPreview] = useState<WeeklyReportWritePreview | null>(null);
  const [pending, setPending] = useState<PendingWeeklyReport | null>(null);
  const [reviewedMarkdown, setReviewedMarkdown] = useState("");
  const [error, setError] = useState<DesktopWriteError | null>(null);
  const [dateError, setDateError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const isDirty = output.trim().length > 0 || preview !== null;

  function requestClose() {
    if (saving) return;
    if (isDirty && !window.confirm("검토 중인 주간 보고서를 닫을까요?")) return;
    onClose();
  }

  async function reviewChanges(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setDateError(null);
    if (validateControls(event.currentTarget)) return;
    if (startDate > endDate) {
      setDateError("시작일은 종료일보다 늦을 수 없습니다.");
      return;
    }

    const generatedAt = new Date().toISOString();
    const input: WeeklyReportInput = {
      start_date: startDate,
      end_date: endDate,
      output: output.trim() || null,
    };
    try {
      const nextPreview = await previewWeeklyReport(input, generatedAt);
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
      const result = await createWeeklyReport(
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

  const stats = preview?.stats;
  const previewStatus = stats
    ? [
        `업무 ${stats.work_item_count}개`,
        `기록 ${stats.checkpoint_count}개`,
        `Git 커밋 ${stats.git_commit_count}개`,
        `검증 ${stats.verification_count}개`,
        stats.redacted_checkpoint_count > 0
          ? `민감 ${stats.redacted_checkpoint_count}개 세부 정보 생략`
          : null,
        stats.excluded_checkpoint_count > 0
          ? `제한 ${stats.excluded_checkpoint_count}개 제외`
          : null,
        stats.unknown_period_checkpoint_count > 0
          ? `기간 미확인 ${stats.unknown_period_checkpoint_count}개 제외`
          : null,
      ]
        .filter(Boolean)
        .join(" · ")
    : "";

  return (
    <EditorDialog
      eyebrow="Weekly report"
      title="주간 성과보고서 만들기"
      titleId="weekly-report-editor-title"
      closeLabel="주간 성과보고서 생성기 닫기"
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
                  ? "검토한 뒤 체크포인트나 업무 상태가 바뀌어 파일을 만들지 않았습니다. 최신 원본으로 다시 검토해 주세요."
                  : error.kind === "create_conflict"
                    ? "기존 보고서를 보호했습니다. 다른 Markdown 경로를 입력해 새 초안을 만들 수 있습니다."
                    : error.message}
              </p>
            </div>
          </div>
        ) : null}

        {preview ? (
          <WriteDiffReview
            eyebrow="주간 성과보고서 저장 전 검토"
            title={`${preview.start_date} ~ ${preview.end_date}`}
            identity={preview.paths.report}
            status={previewStatus}
            files={preview.files}
            saving={saving}
            defaultView="document"
            documentValue={reviewedMarkdown}
            onDocumentChange={setReviewedMarkdown}
            commitLabel="보고서 저장 후 열기"
            onBack={() => {
              setPreview(null);
              setPending(null);
              setReviewedMarkdown("");
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
              setError(null);
              setDateError(null);
            }}
          >
            <section className="editor-section" aria-labelledby="weekly-report-source-title">
              <div className="editor-section-heading">
                <div>
                  <span className="eyebrow">Period</span>
                  <h3 id="weekly-report-source-title">보고할 기간 선택</h3>
                </div>
                <span className="editor-preserved">기존 보고서는 덮어쓰지 않습니다</span>
              </div>
              <p className="editor-helper">
                실제 작업 기간이 선택 범위와 겹치는 체크포인트를 업무별로 묶습니다. Git 커밋과 테스트·빌드·lint·수동 검증 결과도 기록에서 자동 집계합니다.
              </p>
              <p className="editor-policy-note">
                앱이 테스트를 임의로 실행하지는 않습니다. 민감 기록은 세부 근거를 숨기고 제한 기록은 보고서에서 제외합니다.
              </p>
              <div className="editor-grid">
                <label className="editor-field">
                  <span>시작일</span>
                  <input
                    type="date"
                    required
                    value={startDate}
                    onChange={(event) => setStartDate(event.target.value)}
                  />
                </label>
                <label className="editor-field">
                  <span>종료일</span>
                  <input
                    type="date"
                    required
                    value={endDate}
                    onChange={(event) => setEndDate(event.target.value)}
                  />
                </label>
                <label className="editor-field full">
                  <span>저장 경로 · 선택</span>
                  <input
                    value={output}
                    placeholder={`reports/weekly/${startDate.replaceAll("-", "")}_to_${endDate.replaceAll("-", "")}.md`}
                    onChange={(event) => setOutput(event.target.value)}
                  />
                  <small>비워 두면 기간으로 기본 경로를 생성합니다. 데이터 폴더 내부의 .md 경로만 허용합니다.</small>
                </label>
              </div>
              {dateError ? <p className="field-error" role="alert">{dateError}</p> : null}
            </section>

            <footer className="editor-footer">
              <div className="editor-footer-copy" aria-live="polite">
                저장 전에 집계 수치와 실제 Markdown 전체를 검토할 수 있습니다.
              </div>
              <Button size="sm" variant="ghost" onClick={requestClose}>
                취소
              </Button>
              <Button type="submit" size="sm" variant="primary">
                주간 보고서 검토
              </Button>
            </footer>
          </form>
        )}
      </div>
    </EditorDialog>
  );
}
