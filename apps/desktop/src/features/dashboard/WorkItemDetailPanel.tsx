import { CheckpointDetails } from "../../CheckpointDetails";
import type { WorkItemDetail } from "../../desktop";
import { DetailList } from "./DetailList";
import { formatTimestamp } from "./presentation";

interface WorkItemDetailPanelProps {
  actionError: string | null;
  detail: WorkItemDetail | null;
  detailError: string | null;
  detailLoading: boolean;
  emptyMessage?: string;
  onAddCheckpoint: (workItemId: string) => void;
  onCreatePerformanceNote: (workItemId: string) => void;
  onEdit: (workItemId: string) => void;
  onOpenCheckpoint: (checkpointId: string) => void;
  onOpenContext: (workItemId: string) => void;
  onReveal: (workItemId: string) => void;
}

export function WorkItemDetailPanel({
  actionError,
  detail,
  detailError,
  detailLoading,
  emptyMessage = "업무를 선택하면 지금 상태와 이어서 할 일을 보여줍니다.",
  onAddCheckpoint,
  onCreatePerformanceNote,
  onEdit,
  onOpenCheckpoint,
  onOpenContext,
  onReveal,
}: WorkItemDetailPanelProps) {
  return (
    <article className="panel detail-panel" aria-busy={detailLoading}>
      {detailLoading && <p className="muted" role="status">업무 상세를 불러오는 중…</p>}
      {detailError && <div className="alert error" role="alert">{detailError}</div>}
      {!detailLoading && !detail && !detailError && (
        <div className="detail-empty">
          <p className="muted">{emptyMessage}</p>
        </div>
      )}
      {!detailLoading && detail && (
        <div className="detail-content">
          <header className="detail-header">
            <div>
              <p className="section-label">
                {detail.project_id} · {detail.id}
              </p>
              <h2>{detail.title}</h2>
            </div>
            <span className={`status status-${detail.status}`}>{detail.status}</span>
          </header>

          <div className="detail-toolbar">
            <button
              className="inline-action checkpoint-action"
              onClick={() => onAddCheckpoint(detail.id)}
              type="button"
            >
              작업 기록 추가
            </button>
            <button
              className="inline-action report-action"
              onClick={() => onCreatePerformanceNote(detail.id)}
              type="button"
            >
              성과 노트 만들기
            </button>
            <button className="inline-action" onClick={() => onEdit(detail.id)} type="button">
              업무 편집
            </button>
            <button className="inline-action" onClick={() => onReveal(detail.id)} type="button">
              Finder에서 보기
            </button>
            <button className="inline-action" onClick={() => onOpenContext(detail.id)} type="button">
              Context.md 열기
            </button>
          </div>
          {actionError && <div className="alert error compact-alert" role="alert">{actionError}</div>}

          <p className="detail-objective">{detail.objective}</p>
          <div className="tag-row">
            {detail.classification.work_types.map((type) => (
              <span className="tag" key={type}>
                {type}
              </span>
            ))}
            {detail.classification.tags.map((tag) => (
              <span className="tag subtle" key={tag}>
                #{tag}
              </span>
            ))}
          </div>

          <div className="detail-card-grid">
            <section className="detail-card current-state-card">
              <p className="section-label">현재 상태</p>
              <p>{detail.context?.current_state ?? "현재 Context가 없습니다."}</p>
            </section>
            <section className="detail-card">
              <p className="section-label">다음 작업</p>
              <DetailList
                empty="등록된 다음 작업이 없습니다."
                items={detail.context?.next_steps ?? []}
              />
            </section>
          </div>

          <section className="detail-section">
            <div className="detail-section-heading">
              <h3>목표 결과</h3>
              <span>{detail.desired_outcomes.length}개</span>
            </div>
            <DetailList empty="등록된 목표 결과가 없습니다." items={detail.desired_outcomes} />
          </section>

          <section className="detail-section">
            <div className="detail-section-heading">
              <h3>체크포인트 타임라인</h3>
              <span>{detail.checkpoints.length}개</span>
            </div>
            {detail.checkpoints.length === 0 ? (
              <p className="muted">아직 연결된 체크포인트가 없습니다.</p>
            ) : (
              <div className="timeline">
                {detail.checkpoints.map((checkpoint) => (
                  <article className="timeline-item" key={checkpoint.id}>
                    <div className="timeline-marker" />
                    <div className="timeline-body">
                      <div className="timeline-meta">
                        <span>{formatTimestamp(checkpoint.captured_at)}</span>
                        <span>{checkpoint.kind}</span>
                        <span>{checkpoint.status_after}</span>
                      </div>
                      <h4>{checkpoint.title}</h4>
                      <p>{checkpoint.summary}</p>
                      {checkpoint.verifications.length > 0 && (
                        <div className="verification-row">
                          {checkpoint.verifications.map((verification) => (
                            <span
                              className={`verification verification-${verification.status}`}
                              key={`${checkpoint.id}-${verification.kind}-${verification.description}`}
                            >
                              {verification.kind} · {verification.status}
                            </span>
                          ))}
                        </div>
                      )}
                      <CheckpointDetails
                        checkpoint={checkpoint}
                        onOpenMarkdown={onOpenCheckpoint}
                      />
                    </div>
                  </article>
                ))}
              </div>
            )}
          </section>

          <p className="detail-updated">마지막 업무 갱신 {formatTimestamp(detail.updated_at)}</p>
        </div>
      )}
    </article>
  );
}
