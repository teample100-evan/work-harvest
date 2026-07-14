import type { CheckpointSummary } from "./desktop";

interface CheckpointDetailsProps {
  checkpoint: CheckpointSummary;
  onOpenMarkdown: (checkpointId: string) => void;
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

function EvidenceGroup({ label, items }: { label: string; items: string[] }) {
  if (items.length === 0) return null;
  return (
    <div className="evidence-group">
      <strong>{label}</strong>
      <RecordList items={items} />
    </div>
  );
}

export function CheckpointDetails({
  checkpoint,
  onOpenMarkdown,
}: CheckpointDetailsProps) {
  const hasEvidence = Object.values(checkpoint.evidence).some((items) => items.length > 0);

  return (
    <div className="checkpoint-actions">
      <details className="checkpoint-details">
        <summary>전체 기록 펼치기</summary>
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
                    <span>{decision.status}</span>
                  </article>
                ))}
              </div>
            </section>
          )}

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
                        {verification.kind} · {verification.status}
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

          {hasEvidence && (
            <section className="record-section">
              <h5>근거</h5>
              <div className="evidence-grid">
                <EvidenceGroup label="커밋" items={checkpoint.evidence.commits} />
                <EvidenceGroup label="PR" items={checkpoint.evidence.pull_requests} />
                <EvidenceGroup label="이슈" items={checkpoint.evidence.issues} />
                <EvidenceGroup label="파일" items={checkpoint.evidence.files} />
                <EvidenceGroup label="명령" items={checkpoint.evidence.commands} />
                <EvidenceGroup label="URL" items={checkpoint.evidence.urls} />
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
      <button
        className="inline-action"
        onClick={() => onOpenMarkdown(checkpoint.id)}
        type="button"
      >
        Markdown 열기
      </button>
    </div>
  );
}
