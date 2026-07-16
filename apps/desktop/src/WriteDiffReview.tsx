import type { WorkItemFileChange } from "./desktop";
import { Button } from "./ui/Button";

interface WriteDiffReviewProps {
  eyebrow: string;
  title: string;
  identity: string;
  status: string;
  files: WorkItemFileChange[];
  saving: boolean;
  commitLabel?: string;
  onBack: () => void;
  onCommit: () => void;
}

function lineCount(value: string | null): number {
  if (!value) return 0;
  return value.replace(/\n$/, "").split("\n").length;
}

export function WriteDiffReview({
  eyebrow,
  title,
  identity,
  status,
  files,
  saving,
  commitLabel,
  onBack,
  onCommit,
}: WriteDiffReviewProps) {
  return (
    <div className="editor-review">
      <section className="editor-section review-summary" aria-labelledby="review-summary-title">
        <div>
          <span className="eyebrow">{eyebrow}</span>
          <h3 id="review-summary-title">{title}</h3>
          <p>
            <code>{identity}</code> · {status} · 파일 {files.length}개
          </p>
        </div>
        <span className="review-ready-badge">검증 완료</span>
      </section>

      <p className="editor-helper">
        아래 내용은 실제로 저장될 파일의 정확한 전후 비교입니다. 저장 시 preview에 사용한 시각과 revision을 그대로 사용합니다.
      </p>

      <div className="diff-list">
        {files.map((file, index) => (
          <details className="diff-file" key={file.path} open={index === 0}>
            <summary>
              <span className={`diff-operation ${file.operation}`}>
                {file.operation === "create" ? "생성" : "수정"}
              </span>
              <code>{file.path}</code>
              <span className="diff-line-count">
                {lineCount(file.before)} → {lineCount(file.after)}줄
              </span>
            </summary>
            <div className="diff-columns">
              <section className="diff-pane" aria-label={`${file.path} 저장 전`}>
                <h4>저장 전</h4>
                <pre>{file.before ?? "(새 파일)"}</pre>
              </section>
              <section className="diff-pane after" aria-label={`${file.path} 저장 후`}>
                <h4>저장 후</h4>
                <pre>{file.after}</pre>
              </section>
            </div>
          </details>
        ))}
      </div>

      <footer className="editor-footer">
        <Button size="sm" variant="ghost" onClick={onBack} disabled={saving}>
          편집으로 돌아가기
        </Button>
        <Button size="sm" variant="primary" onClick={onCommit} disabled={saving}>
          {saving ? "저장 중…" : (commitLabel ?? `${files.length}개 파일 저장`)}
        </Button>
      </footer>
    </div>
  );
}
