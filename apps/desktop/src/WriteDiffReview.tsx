import type { WorkItemFileChange } from "./desktop";
import { Button } from "./ui/Button";
import { useState } from "react";

interface WriteDiffReviewProps {
  eyebrow: string;
  title: string;
  identity: string;
  status: string;
  files: WorkItemFileChange[];
  saving: boolean;
  defaultView?: "document" | "changes";
  documentValue?: string;
  onDocumentChange?: (value: string) => void;
  commitLabel?: string;
  onBack: () => void;
  onCommit: () => void;
}

function MarkdownDocumentPreview({ value }: { value: string }) {
  const withoutFrontmatter = value.replace(/^---\n[\s\S]*?\n---\n?/, "");
  return (
    <article className="markdown-document-preview">
      {withoutFrontmatter.split("\n").map((line, index) => {
        const key = `${index}-${line.slice(0, 24)}`;
        if (!line.trim()) return <span className="markdown-preview-space" key={key} />;
        if (line.startsWith("### ")) return <h4 key={key}>{line.slice(4)}</h4>;
        if (line.startsWith("## ")) return <h3 key={key}>{line.slice(3)}</h3>;
        if (line.startsWith("# ")) return <h2 key={key}>{line.slice(2)}</h2>;
        if (line.startsWith("> ")) return <blockquote key={key}>{line.slice(2)}</blockquote>;
        if (line.startsWith("- ")) return <p className="markdown-preview-list-item" key={key}>{line.slice(2)}</p>;
        return <p key={key}>{line}</p>;
      })}
    </article>
  );
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
  defaultView = "changes",
  documentValue,
  onDocumentChange,
  commitLabel,
  onBack,
  onCommit,
}: WriteDiffReviewProps) {
  const canPreviewDocument = files.length === 1 && files[0].path.endsWith(".md");
  const [view, setView] = useState<"document" | "edit" | "changes">(
    canPreviewDocument ? defaultView : "changes",
  );
  const effectiveDocument = documentValue ?? files[0]?.after ?? "";
  const reviewedFiles =
    canPreviewDocument && documentValue !== undefined
      ? [{ ...files[0], after: documentValue }]
      : files;

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

      <div className="review-view-heading">
        <p className="editor-helper">
          {view === "document"
            ? "저장될 문서를 읽기 편한 형태로 먼저 확인합니다. 정확한 파일 내용은 원시 변경에서 볼 수 있습니다."
            : view === "edit"
              ? "저장할 Markdown을 직접 다듬습니다. 기밀 수준 front matter는 낮출 수 없습니다."
              : "실제로 저장될 파일의 정확한 전후 비교입니다. 저장 시 preview의 시각과 revision을 그대로 사용합니다."}
        </p>
        {canPreviewDocument ? (
          <div className="review-view-switch" aria-label="검토 보기 방식">
            <button
              aria-pressed={view === "document"}
              onClick={() => setView("document")}
              type="button"
            >
              문서 미리보기
            </button>
            {onDocumentChange ? (
              <button
                aria-pressed={view === "edit"}
                onClick={() => setView("edit")}
                type="button"
              >
                내용 편집
              </button>
            ) : null}
            <button
              aria-pressed={view === "changes"}
              onClick={() => setView("changes")}
              type="button"
            >
              원시 변경
            </button>
          </div>
        ) : null}
      </div>

      {view === "document" && canPreviewDocument ? (
        <MarkdownDocumentPreview value={effectiveDocument} />
      ) : view === "edit" && onDocumentChange ? (
        <label className="markdown-document-editor">
          <span>저장할 Markdown</span>
          <textarea
            aria-label="저장할 성과 노트 Markdown"
            onChange={(event) => onDocumentChange(event.target.value)}
            spellCheck={false}
            value={effectiveDocument}
          />
        </label>
      ) : (
      <div className="diff-list">
        {reviewedFiles.map((file, index) => (
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
      )}

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
