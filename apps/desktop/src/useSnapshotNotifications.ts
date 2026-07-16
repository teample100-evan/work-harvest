import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { useCallback, useEffect, useRef, useState } from "react";
import type { DataIssue, DataRootSnapshot } from "./desktop";

export type NotificationState = "checking" | "prompt" | "granted" | "denied" | "error";

const NOTIFICATIONS_ENABLED_KEY = "work-harvest:notifications-enabled";

function issueKey(issue: DataIssue) {
  return `${issue.severity}\u0000${issue.path}\u0000${issue.code}\u0000${issue.message}`;
}

function notificationBody(value: string) {
  const maxLength = 180;
  return value.length > maxLength ? `${value.slice(0, maxLength - 1)}…` : value;
}

function announceSnapshotChanges(previous: DataRootSnapshot, next: DataRootSnapshot) {
  const previousCheckpointIds = new Set(previous.checkpoint_ids);
  const newCheckpointIds = next.checkpoint_ids.filter((id) => !previousCheckpointIds.has(id));

  if (newCheckpointIds.length > 0) {
    const relatedWorkItem = next.work_items.find(
      (item) =>
        item.last_checkpoint_id !== null && newCheckpointIds.includes(item.last_checkpoint_id),
    );
    const subject = relatedWorkItem?.title ?? newCheckpointIds[0];
    sendNotification({
      title:
        newCheckpointIds.length === 1
          ? "새 체크포인트를 감지했습니다"
          : `새 체크포인트 ${newCheckpointIds.length}개를 감지했습니다`,
      body: notificationBody(`${subject} · 기록이 대시보드에 반영되었습니다.`),
      group: "work-harvest-checkpoints",
    });
  }

  const previousIssueKeys = new Set(
    previous.issues.filter((issue) => issue.severity === "error").map(issueKey),
  );
  const newErrors = next.issues.filter(
    (issue) => issue.severity === "error" && !previousIssueKeys.has(issueKey(issue)),
  );

  if (newErrors.length > 0) {
    const firstError = newErrors[0];
    sendNotification({
      title:
        newErrors.length === 1
          ? "새 검증 오류를 발견했습니다"
          : `새 검증 오류 ${newErrors.length}개를 발견했습니다`,
      body: notificationBody(`${firstError.path} · ${firstError.message}`),
      group: "work-harvest-validation",
    });
  }
}

export function useSnapshotNotifications() {
  const baseline = useRef<DataRootSnapshot | null>(null);
  const permissionGranted = useRef(false);
  const [state, setState] = useState<NotificationState>("checking");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (localStorage.getItem(NOTIFICATIONS_ENABLED_KEY) !== "true") {
      setState("prompt");
      return;
    }

    let disposed = false;
    void isPermissionGranted()
      .then((granted) => {
        if (disposed) return;
        permissionGranted.current = granted;
        setState(granted ? "granted" : "prompt");
      })
      .catch((nextError: unknown) => {
        if (disposed) return;
        setError(String(nextError));
        setState("error");
      });
    return () => {
      disposed = true;
    };
  }, []);

  const enableNotifications = useCallback(async () => {
    setError(null);
    try {
      const alreadyGranted = await isPermissionGranted();
      const permission = alreadyGranted ? "granted" : await requestPermission();
      const granted = permission === "granted";
      permissionGranted.current = granted;
      setState(granted ? "granted" : "denied");
      if (granted) {
        localStorage.setItem(NOTIFICATIONS_ENABLED_KEY, "true");
        sendNotification({
          title: "Work Harvest 알림이 켜졌습니다",
          body: "새 체크포인트와 검증 오류만 한 번씩 알려드립니다.",
        });
      }
    } catch (nextError) {
      setError(String(nextError));
      setState("error");
    }
  }, []);

  const observeSnapshot = useCallback(
    (next: DataRootSnapshot, announceChanges: boolean) => {
      const previous = baseline.current;
      baseline.current = next;
      if (announceChanges && previous && permissionGranted.current) {
        announceSnapshotChanges(previous, next);
      }
    },
    [],
  );

  return {
    enableNotifications,
    notificationError: error,
    notificationState: state,
    observeSnapshot,
  };
}
