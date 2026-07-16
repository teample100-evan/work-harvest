import type { CheckpointSummary } from "./desktop";
import { ChevronRight, ExternalLink } from "lucide-react";
import { useState } from "react";
import {
  formatDecisionStatus,
  formatVerificationKind,
  formatVerificationStatus,
} from "./features/dashboard/presentation";

interface CheckpointDetailsProps {
  checkpoint: CheckpointSummary;
  onOpenMarkdown: (checkpointId: string) => void;
  onOpenUrl: (url: string) => void;
}

function RecordList({ items }: { items: string[] }) {
  return (
    <ul className="record-list">
      {items.map((item, index) => (
        <li key={`${item}-${index}`}>{item}</li>
      ))}
    </ul>
  );
}

function EvidenceGroup({
  label,
  items,
  onOpenUrl,
}: {
  label: string;
  items: string[];
  onOpenUrl: (url: string) => void;
}) {
  if (items.length === 0) return null;
  return (
    <div className="evidence-group">
      <strong>{label}</strong>
      {label === "URL" ? (
        <ul className="record-list evidence-link-list">
          {items.map((url) => (
            <li key={url}>
              <button onClick={() => onOpenUrl(url)} type="button">
                <span>{url}</span>
                <ExternalLink aria-hidden="true" size={13} strokeWidth={1.8} />
              </button>
            </li>
          ))}
        </ul>
      ) : (
        <RecordList items={items} />
      )}
    </div>
  );
}

export function CheckpointDetails({
  checkpoint,
  onOpenMarkdown,
  onOpenUrl,
}: CheckpointDetailsProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const hasEvidence = Object.values(checkpoint.evidence).some((items) => items.length > 0);
  const hasSupportingDetails = checkpoint.verifications.length > 0 || hasEvidence || Boolean(checkpoint.git);

  return (
    <div className="checkpoint-actions">
      <div className="checkpoint-action-row">
        <button
          aria-expanded={isExpanded}
          className="checkpoint-expand-action"
          onClick={() => setIsExpanded((current) => !current)}
          type="button"
        >
          <ChevronRight
            aria-hidden="true"
            className={isExpanded ? "disclosure-chevron expanded" : "disclosure-chevron"}
            size={14}
            strokeWidth={2}
          />
          전체 기록
        </button>
        <button
          className="checkpoint-markdown-action"
          onClick={() => onOpenMarkdown(checkpoint.id)}
          type="button"
        >
          Markdown 열기
        </button>
      </div>
      {isExpanded && (
        <div className="checkpoint-detail-content">
          <div className="record-grid">
            <section>
              <h5>진행한 작업</h5>
              {checkpoint.activities.length > 0 ? (
                <RecordList items={checkpoint.activities} />
              ) : (
                <p className="record-empty">기록 없음</p>
              )}
            </section>
            <section>
              <h5>결과</h5>
              {checkpoint.outcomes.length > 0 ? (
                <RecordList items={checkpoint.outcomes} />
              ) : (
                <p className="record-empty">기록 없음</p>
              )}
            </section>
          </div>

          {checkpoint.decisions.length > 0 && (
            <section className="record-section">
              <h5>결정과 이유</h5>
              <div className="decision-list">
                {checkpoint.decisions.map((decision) => (
                  <article key={`${decision.summary}-${decision.status}`}>
                    <strong>{decision.summary}</strong>
                    <p>{decision.rationale}</p>
                    <span>{formatDecisionStatus(decision.status)}</span>
                  </article>
                ))}
              </div>
            </section>
          )}

          {(checkpoint.blockers.length > 0 || checkpoint.next_steps.length > 0) && (
            <div className="record-grid record-section">
              <section>
                <h5>차단 요소</h5>
                {checkpoint.blockers.length > 0 ? (
                  <RecordList items={checkpoint.blockers} />
                ) : (
                  <p className="record-empty">없음</p>
                )}
              </section>
              <section>
                <h5>다음 작업</h5>
                {checkpoint.next_steps.length > 0 ? (
                  <RecordList items={checkpoint.next_steps} />
                ) : (
                  <p className="record-empty">없음</p>
                )}
              </section>
            </div>
          )}

          {hasSupportingDetails && (
            <details className="checkpoint-supporting-details">
              <summary>
                <ChevronRight
                  aria-hidden="true"
                  className="disclosure-chevron"
                  size={14}
                  strokeWidth={2}
                />
                근거 및 검증
              </summary>
              <div className="checkpoint-supporting-content">
                {checkpoint.verifications.length > 0 && (
                  <section className="record-section">
                    <h5>검증 상세</h5>
                    <div className="verification-detail-list">
                      {checkpoint.verifications.map((verification) => (
                        <article
                          key={`${verification.kind}-${verification.description}-${verification.status}`}
                        >
                          <div>
                            <strong>{verification.description}</strong>
                            <span className={`verification verification-${verification.status}`}>
                              {formatVerificationKind(verification.kind)} · {formatVerificationStatus(verification.status)}
                            </span>
                          </div>
                          {verification.command && <code>{verification.command}</code>}
                          {verification.evidence_refs.length > 0 && (
                            <RecordList items={verification.evidence_refs} />
                          )}
                        </article>
                      ))}
                    </div>
                  </section>
                )}

                {hasEvidence && (
                  <section className="record-section">
                    <h5>근거</h5>
                    <div className="evidence-grid">
                      <div className="evidence-cluster">
                        <p>추적 정보</p>
                        <EvidenceGroup label="커밋" items={checkpoint.evidence.commits} onOpenUrl={onOpenUrl} />
                        <EvidenceGroup label="PR" items={checkpoint.evidence.pull_requests} onOpenUrl={onOpenUrl} />
                        <EvidenceGroup label="이슈" items={checkpoint.evidence.issues} onOpenUrl={onOpenUrl} />
                      </div>
                      <div className="evidence-cluster">
                        <p>재현 정보</p>
                        <EvidenceGroup label="파일" items={checkpoint.evidence.files} onOpenUrl={onOpenUrl} />
                        <EvidenceGroup label="명령" items={checkpoint.evidence.commands} onOpenUrl={onOpenUrl} />
                      </div>
                      <div className="evidence-cluster">
                        <p>바로가기</p>
                        <EvidenceGroup label="URL" items={checkpoint.evidence.urls} onOpenUrl={onOpenUrl} />
                      </div>
                    </div>
                  </section>
                )}

                {checkpoint.git && (
                  <section className="record-section git-record">
                    <h5>Git 기준점</h5>
                    <code>
                      {checkpoint.git.repository}
                      {checkpoint.git.branch ? ` · ${checkpoint.git.branch}` : ""}
                      {checkpoint.git.head_after ? ` · ${checkpoint.git.head_after}` : ""}
                    </code>
                  </section>
                )}
              </div>
            </details>
          )}
        </div>
      )}
    </div>
  );
}
