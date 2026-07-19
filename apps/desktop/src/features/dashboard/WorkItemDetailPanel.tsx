import { Menu } from "@base-ui/react/menu";
import { CheckpointDetails } from "../../CheckpointDetails";
import type { WorkItemDetail } from "../../desktop";
import { DetailList } from "./DetailList";
import {
  formatTimestamp,
  formatCheckpointKind,
  formatConfidentiality,
  formatVerificationKind,
  formatVerificationStatus,
  formatWorkItemStatus,
  needsWorkItemStatusBadge,
} from "./presentation";

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
  onOpenExternalUrl: (url: string) => void;
  onReveal: (workItemId: string) => void;
  onTrash: (workItemId: string) => void;
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
  onOpenExternalUrl,
  onReveal,
  onTrash,
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
        <div className="detail-content typeset-work-detail">
          <header className="detail-header">
            <div>
              <h2>{detail.title}</h2>
              <p className="detail-meta">
                <span>{detail.project_id}</span>
                <span>{formatWorkItemStatus(detail.status)}</span>
                <span>갱신 {formatTimestamp(detail.updated_at)}</span>
              </p>
              <div className="detail-classification">
                <span className="detail-id">ID {detail.id}</span>
                <span>
                  {detail.scope === "company"
                    ? "회사 업무"
                    : detail.scope === "personal"
                      ? "개인 업무"
                      : "미분류"}
                </span>
                <span>
                  {detail.reporting.mode === "primary"
                    ? "주요 업무"
                    : detail.reporting.mode === "supporting"
                      ? "지원 활동"
                      : "보고 제외"}
                </span>
                {detail.classification.work_types.map((type) => (
                  <span key={type}>{type}</span>
                ))}
                {detail.classification.tags.map((tag) => (
                  <span key={tag}>#{tag}</span>
                ))}
              </div>
            </div>
            {needsWorkItemStatusBadge(detail.status) && (
              <span className={`status status-${detail.status}`}>
                {formatWorkItemStatus(detail.status)}
              </span>
            )}
          </header>

          <div className="detail-toolbar">
            <button
              className="inline-action checkpoint-action"
              onClick={() => onAddCheckpoint(detail.id)}
              type="button"
            >
              작업 기록 추가
            </button>
            <button className="inline-action" onClick={() => onEdit(detail.id)} type="button">
              업무 편집
            </button>
            <Menu.Root modal={false}>
              <Menu.Trigger aria-label="기타 업무 작업" className="detail-actions-trigger">
                ···
              </Menu.Trigger>
              <Menu.Portal>
                <Menu.Positioner
                  align="start"
                  className="detail-actions-positioner"
                  collisionPadding={8}
                  sideOffset={6}
                >
                  <Menu.Popup className="detail-actions-popover">
                    <Menu.Item
                      className="detail-actions-item"
                      onClick={() => onCreatePerformanceNote(detail.id)}
                    >
                      성과 노트 만들기
                    </Menu.Item>
                    <Menu.Item
                      className="detail-actions-item"
                      onClick={() => onOpenContext(detail.id)}
                    >
                      Context.md 열기
                    </Menu.Item>
                    <Menu.Item
                      className="detail-actions-item"
                      onClick={() => onReveal(detail.id)}
                    >
                      Finder에서 보기
                    </Menu.Item>
                    <Menu.Item
                      className="detail-actions-item danger"
                      onClick={() => {
                        if (
                          window.confirm(
                            "이 업무와 연결된 체크포인트를 휴지통으로 이동할까요? 작업 환경 메뉴에서 복구할 수 있습니다.",
                          )
                        ) {
                          onTrash(detail.id);
                        }
                      }}
                    >
                      휴지통으로 이동
                    </Menu.Item>
                  </Menu.Popup>
                </Menu.Positioner>
              </Menu.Portal>
            </Menu.Root>
          </div>
          {actionError && <div className="alert error compact-alert" role="alert">{actionError}</div>}

          <section className="detail-introduction" aria-labelledby="detail-objective-heading">
            <p className="section-label" id="detail-objective-heading">업무 설명</p>
            <p className="detail-objective">{detail.objective}</p>
          </section>

          <section className="detail-section detail-context-section">
            <div className="detail-context-block">
              <p className="section-label">현재 상태</p>
              <p>{detail.context?.current_state ?? "현재 기록된 상태가 없습니다."}</p>
            </div>
            <div className="detail-context-block">
              <p className="section-label">다음 작업</p>
              <DetailList
                empty="등록된 다음 작업이 없습니다."
                items={detail.context?.next_steps ?? []}
              />
            </div>
          </section>

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
                        <span>{formatCheckpointKind(checkpoint.kind)}</span>
                        <span>{formatWorkItemStatus(checkpoint.status_after)}</span>
                        {checkpoint.confidentiality !== "normal" ? (
                          <span className={`confidentiality confidentiality-${checkpoint.confidentiality}`}>
                            {formatConfidentiality(checkpoint.confidentiality)}
                          </span>
                        ) : null}
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
                              {formatVerificationKind(verification.kind)} · {formatVerificationStatus(verification.status)}
                            </span>
                          ))}
                        </div>
                      )}
                      <CheckpointDetails
                        checkpoint={checkpoint}
                        onOpenMarkdown={onOpenCheckpoint}
                        onOpenUrl={onOpenExternalUrl}
                      />
                    </div>
                  </article>
                ))}
              </div>
            )}
          </section>

        </div>
      )}
    </article>
  );
}
