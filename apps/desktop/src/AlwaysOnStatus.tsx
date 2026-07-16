import type { NotificationState } from "./useSnapshotNotifications";

interface AlwaysOnStatusProps {
  notificationError: string | null;
  notificationState: NotificationState;
  onEnableNotifications: () => Promise<void>;
}

function notificationLabel(state: NotificationState) {
  if (state === "checking") return "알림 확인 중";
  if (state === "granted") return "알림 사용 중";
  if (state === "denied") return "알림 권한 거부됨";
  if (state === "error") return "알림 확인 실패";
  return "알림 꺼짐";
}

export function AlwaysOnStatus({
  notificationError,
  notificationState,
  onEnableNotifications,
}: AlwaysOnStatusProps) {
  const canEnable = notificationState !== "checking" && notificationState !== "granted";

  return (
    <section className="always-on-status" aria-label="상시 실행 상태">
      <div className="always-on-copy">
        <span className="always-on-dot" />
        <div>
          <strong>메뉴바에서 계속 실행 중</strong>
          <p>창을 닫아도 파일 감시는 유지됩니다. 알림은 선택해 켤 수 있습니다.</p>
        </div>
      </div>
      <div className="notification-control">
        <span className={`notification-state notification-state-${notificationState}`}>
          {notificationLabel(notificationState)}
        </span>
        {canEnable && (
          <button
            className="inline-action"
            onClick={() => void onEnableNotifications()}
            type="button"
          >
            {notificationState === "error" ? "다시 시도" : "알림 켜기"}
          </button>
        )}
        {notificationState === "granted" && <small>macOS 알림 설정을 따릅니다.</small>}
        {notificationError && <small title={notificationError}>시스템 알림 설정 확인 필요</small>}
      </div>
    </section>
  );
}
