import { Popover } from "@base-ui/react/popover";
import { Bell, ChevronRight, FolderOpen, RefreshCw, Settings2, TimerReset } from "lucide-react";
import { useEffect, useState } from "react";
import {
  getBuildInfo,
  type BuildInfo,
  type DataRootSnapshot,
} from "../../desktop";
import type { NotificationState } from "../../useSnapshotNotifications";
import { Button } from "../../ui/Button";

interface WorkspaceEnvironmentMenuProps {
  errorCount: number;
  notificationError: string | null;
  notificationState: NotificationState;
  snapshot: DataRootSnapshot;
  onChooseRoot: () => void;
  onEnableNotifications: () => Promise<void>;
  onRefresh: () => void;
}

function notificationLabel(state: NotificationState) {
  if (state === "checking") return "확인 중";
  if (state === "granted") return "켜짐";
  if (state === "denied") return "권한 필요";
  if (state === "error") return "확인 실패";
  return "꺼짐";
}

export function WorkspaceEnvironmentMenu({
  errorCount,
  notificationError,
  notificationState,
  snapshot,
  onChooseRoot,
  onEnableNotifications,
  onRefresh,
}: WorkspaceEnvironmentMenuProps) {
  const [buildInfo, setBuildInfo] = useState<BuildInfo | null>(null);
  const canEnableNotifications =
    notificationState !== "checking" && notificationState !== "granted";

  useEffect(() => {
    let disposed = false;
    void getBuildInfo()
      .then((nextBuildInfo) => {
        if (!disposed) setBuildInfo(nextBuildInfo);
      })
      .catch(() => {
        if (!disposed) setBuildInfo(null);
      });
    return () => {
      disposed = true;
    };
  }, []);

  return (
    <footer className="sidebar-footer">
      <Popover.Root modal={false}>
        <Popover.Trigger className="workspace-environment-trigger">
          <span>
            <Settings2 aria-hidden="true" size={15} strokeWidth={1.8} />
            작업 환경
          </span>
          <ChevronRight
            aria-hidden="true"
            className="disclosure-chevron environment-chevron"
            size={15}
            strokeWidth={1.8}
          />
        </Popover.Trigger>

        <Popover.Portal>
          <Popover.Positioner
            align="start"
            className="environment-popover-positioner"
            collisionPadding={10}
            side="top"
            sideOffset={8}
          >
            <Popover.Popup className="environment-popover">
              <div className="environment-heading">
                <div>
                  <p className="section-label">로컬 작업 공간</p>
                  <Popover.Title>작업 환경</Popover.Title>
                </div>
                <span className={`environment-health ${errorCount > 0 ? "unhealthy" : "healthy"}`}>
                  <span className="health-dot" />
                  {errorCount > 0 ? `오류 ${errorCount}개` : "정상"}
                </span>
              </div>

              <p className="environment-root" title={snapshot.root}>
                {snapshot.root}
              </p>

              <div className="environment-counts" aria-label="데이터 수량">
                <span>업무 {snapshot.counts.work_items}</span>
                <span>Context {snapshot.counts.contexts}</span>
                <span>체크포인트 {snapshot.counts.checkpoints}</span>
              </div>

              {buildInfo ? (
                <div className="environment-build" aria-label="앱 빌드 정보">
                  <div>
                    <strong>앱 빌드</strong>
                    <span>v{buildInfo.version} · {buildInfo.profile}</span>
                  </div>
                  <code title={buildInfo.commit}>
                    {buildInfo.commit}{buildInfo.dirty ? " · 수정 포함" : ""}
                  </code>
                  <small>
                    {buildInfo.built_at_unix > 0
                      ? new Date(buildInfo.built_at_unix * 1000).toLocaleString("ko-KR")
                      : "빌드 시각 미확인"}
                  </small>
                </div>
              ) : null}

              <div className="environment-settings" aria-label="백그라운드 실행 설정">
                <div className="environment-setting-row">
                  <span className="environment-setting-name">
                    <TimerReset aria-hidden="true" size={15} strokeWidth={1.8} />
                    <span>
                      <strong>백그라운드 실행</strong>
                      <small>창을 닫아도 감시 유지</small>
                    </span>
                  </span>
                  <span className="environment-setting-state active">실행 중</span>
                </div>

                <div className="environment-setting-row">
                  <span className="environment-setting-name">
                    <Bell aria-hidden="true" size={15} strokeWidth={1.8} />
                    <span>
                      <strong>알림</strong>
                      <small title={notificationError ?? undefined}>
                        {notificationError ? "시스템 설정 확인 필요" : "업무 변경 알림"}
                      </small>
                    </span>
                  </span>
                  {canEnableNotifications ? (
                    <button
                      className="environment-notification-action"
                      onClick={() => void onEnableNotifications()}
                      type="button"
                    >
                      {notificationState === "error" ? "재시도" : "켜기"}
                    </button>
                  ) : (
                    <span className="environment-setting-state">
                      {notificationLabel(notificationState)}
                    </span>
                  )}
                </div>
              </div>

              <div className="environment-actions">
                <Button size="sm" variant="secondary" onClick={onRefresh}>
                  <RefreshCw aria-hidden="true" size={14} strokeWidth={1.8} />
                  다시 검사
                </Button>
                <Button size="sm" variant="ghost" onClick={onChooseRoot}>
                  <FolderOpen aria-hidden="true" size={15} strokeWidth={1.8} />
                  폴더 변경
                </Button>
              </div>
            </Popover.Popup>
          </Popover.Positioner>
        </Popover.Portal>
      </Popover.Root>
    </footer>
  );
}
