import { AlwaysOnStatus } from "../../AlwaysOnStatus";
import type { DataRootSnapshot } from "../../desktop";
import type { NotificationState } from "../../useSnapshotNotifications";
import { issueGuidance } from "./presentation";
import type { IndexActivity } from "./useWorkspaceController";

interface SystemOverviewProps {
  indexActivity: IndexActivity | null;
  lastUpdatedAt: Date | null;
  notificationError: string | null;
  notificationState: NotificationState;
  snapshot: DataRootSnapshot;
  onEnableNotifications: () => Promise<void>;
}

export function SystemOverview({
  indexActivity,
  lastUpdatedAt,
  notificationError,
  notificationState,
  snapshot,
  onEnableNotifications,
}: SystemOverviewProps) {
  return (
    <section className="system-overview" aria-labelledby="system-overview-title">
      <div className="system-overview-heading">
        <div>
          <p className="section-label">시스템 상태</p>
          <h2 id="system-overview-title">저장소와 파일 감시</h2>
        </div>
        <p>업무 흐름과 분리해 필요할 때 확인하는 운영 정보입니다.</p>
      </div>

      <div className="system-overview-grid">
        <section className="metric-grid" aria-label="데이터 수량">
          <article className="metric-card">
            <span>업무</span>
            <strong>{snapshot.counts.work_items}</strong>
          </article>
          <article className="metric-card">
            <span>Context</span>
            <strong>{snapshot.counts.contexts}</strong>
          </article>
          <article className="metric-card">
            <span>체크포인트</span>
            <strong>{snapshot.counts.checkpoints}</strong>
          </article>
        </section>

        <AlwaysOnStatus
          notificationError={notificationError}
          notificationState={notificationState}
          onEnableNotifications={onEnableNotifications}
        />
      </div>

      {snapshot.issues.length > 0 && (
        <article className="panel issue-panel">
          <div className="panel-heading">
            <div>
              <p className="section-label">Draft 2020-12 · 파일 관계 검사</p>
              <h2>확인이 필요한 문제</h2>
            </div>
            <span>{snapshot.issues.length}개</span>
          </div>
          <div className="issue-list">
            {snapshot.issues.slice(0, 20).map((issue) => (
              <div className="issue" key={`${issue.path}-${issue.code}-${issue.message}`}>
                <strong>{issue.message}</strong>
                <code>{issue.path}</code>
                <p>{issueGuidance(issue.code)}</p>
              </div>
            ))}
          </div>
        </article>
      )}

      <footer className="watcher-status" role="status">
        외부 파일 변경 감시 중
        {lastUpdatedAt && ` · 마지막 확인 ${lastUpdatedAt.toLocaleTimeString("ko-KR")}`}
        {indexActivity &&
          ` · 인덱스 r${indexActivity.revision} · ${
            indexActivity.fullRescan ? "전체" : "증분"
          } 검사 · JSON ${indexActivity.reloadedFiles}개${
            indexActivity.eventCount === null
              ? ""
              : ` · 이벤트 ${indexActivity.eventCount}건 → 경로 ${indexActivity.pathCount}개`
          }`}
      </footer>
    </section>
  );
}
