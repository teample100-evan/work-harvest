import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  getWorkItemDetail,
  inspectDataRoot,
  openCheckpointMarkdown,
  openContextMarkdown,
  openExternalUrl,
  openPerformanceNoteMarkdown,
  openWeeklyReportMarkdown,
  revealWorkItem,
  setDataRoot,
  trashWorkItem,
  type DataRootChange,
  type DataRootSnapshot,
  type WorkItemDetail,
} from "../../desktop";
import { useSnapshotNotifications } from "../../useSnapshotNotifications";
import { friendlyError } from "./presentation";
import { workItemMatchesDate, workItemPrimaryDateKey } from "./workItemDates";

const DATA_ROOT_KEY = "work-harvest:data-root";

export interface IndexActivity {
  revision: number;
  reloadedFiles: number;
  eventCount: number | null;
  pathCount: number | null;
  fullRescan: boolean;
}

export type EditorState =
  | { mode: "create" }
  | { mode: "edit"; workItemId: string }
  | { mode: "checkpoint"; workItemId: string }
  | { mode: "performance-note"; workItemId: string }
  | { mode: "weekly-report"; startDate: string; endDate: string };

export function useWorkspaceController() {
  const [snapshot, setSnapshot] = useState<DataRootSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdatedAt, setLastUpdatedAt] = useState<Date | null>(null);
  const [selectedWorkItemId, setSelectedWorkItemId] = useState<string | null>(null);
  const [detail, setDetail] = useState<WorkItemDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [dateFilter, setDateFilter] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [detailRevision, setDetailRevision] = useState(0);
  const [indexActivity, setIndexActivity] = useState<IndexActivity | null>(null);
  const [editor, setEditor] = useState<EditorState | null>(null);
  const selectedWorkItemIdRef = useRef<string | null>(null);
  const requestGeneration = useRef(0);
  const {
    enableNotifications,
    notificationError,
    notificationState,
    observeSnapshot,
  } = useSnapshotNotifications();

  const applyRoot = useCallback(async (root: string) => {
    const generation = ++requestGeneration.current;
    setLoading(true);
    setError(null);
    try {
      const nextSnapshot = await setDataRoot(root);
      if (generation !== requestGeneration.current) return;
      localStorage.setItem(DATA_ROOT_KEY, root);
      observeSnapshot(nextSnapshot, false);
      setSnapshot(nextSnapshot);
      setLastUpdatedAt(new Date());
      setDetailRevision((revision) => revision + 1);
      setIndexActivity({
        revision: 1,
        reloadedFiles:
          nextSnapshot.counts.work_items +
          nextSnapshot.counts.contexts +
          nextSnapshot.counts.checkpoints,
        eventCount: null,
        pathCount: null,
        fullRescan: true,
      });
    } catch (nextError) {
      if (generation !== requestGeneration.current) return;
      setError(friendlyError(nextError));
    } finally {
      if (generation === requestGeneration.current) setLoading(false);
    }
  }, [observeSnapshot]);

  const refresh = useCallback(async () => {
    const generation = ++requestGeneration.current;
    try {
      const update = await inspectDataRoot();
      if (generation !== requestGeneration.current) return;
      const nextSnapshot = update.snapshot;
      observeSnapshot(nextSnapshot, true);
      setSnapshot(nextSnapshot);
      setError(null);
      setLastUpdatedAt(new Date());
      setDetailRevision((revision) => revision + 1);
      setIndexActivity({
        revision: update.revision,
        reloadedFiles: update.reloaded_files,
        eventCount: null,
        pathCount: null,
        fullRescan: update.full_rescan,
      });
    } catch (nextError) {
      if (generation !== requestGeneration.current) return;
      setError(friendlyError(nextError));
    }
  }, [observeSnapshot]);

  useEffect(() => {
    const savedRoot = localStorage.getItem(DATA_ROOT_KEY);
    if (savedRoot) {
      void applyRoot(savedRoot);
    } else {
      setLoading(false);
    }
  }, [applyRoot]);

  useEffect(() => {
    selectedWorkItemIdRef.current = selectedWorkItemId;
  }, [selectedWorkItemId]);

  useEffect(() => {
    let disposed = false;
    let unlisten: Array<() => void> = [];
    void Promise.allSettled([
      listen<DataRootChange>("data-root-updated", (event) => {
        const update = event.payload;
        observeSnapshot(update.snapshot, true);
        setSnapshot(update.snapshot);
        setError(null);
        setLastUpdatedAt(new Date());
        setIndexActivity({
          revision: update.revision,
          reloadedFiles: update.reloaded_files,
          eventCount: update.event_count,
          pathCount: update.paths.length,
          fullRescan: update.full_rescan,
        });
        const selectedId = selectedWorkItemIdRef.current;
        if (
          update.full_rescan ||
          (selectedId !== null && update.changed_work_item_ids.includes(selectedId))
        ) {
          setDetailRevision((revision) => revision + 1);
        }
      }),
      listen<string>("data-root-watch-error", (event) => {
        setError(`파일 변경 감시에 실패했습니다: ${event.payload}`);
      }),
      listen<string>("tray-work-item-selected", (event) => {
        setDateFilter(null);
        setQuery("");
        setStatusFilter("all");
        setSelectedWorkItemId(event.payload);
      }),
      listen<string>("always-on-error", (event) => {
        setActionError(`메뉴바 상태를 갱신하지 못했습니다: ${event.payload}`);
      }),
    ]).then((results) => {
      const stopListening = results.flatMap((result) =>
        result.status === "fulfilled" ? [result.value] : [],
      );
      const failure = results.find((result) => result.status === "rejected");
      if (failure?.status === "rejected" && !disposed) {
        setError(`파일 변경 연결에 실패했습니다: ${friendlyError(failure.reason)}`);
      }
      if (disposed) {
        stopListening.forEach((stop) => stop());
      } else {
        unlisten = stopListening;
      }
    });

    return () => {
      disposed = true;
      unlisten.forEach((stop) => stop());
    };
  }, [observeSnapshot]);

  useEffect(() => {
    if (!snapshot || snapshot.work_items.length === 0) {
      setDateFilter(null);
      setSelectedWorkItemId(null);
      return;
    }

    const selectedItem = snapshot.work_items.find((item) => item.id === selectedWorkItemId);
    const nextSelectedItem = selectedItem ?? snapshot.work_items[0];
    if (!selectedItem) {
      setSelectedWorkItemId(nextSelectedItem.id);
    }

    const hasSelectedDate =
      dateFilter !== null &&
      snapshot.work_items.some((item) => workItemMatchesDate(item, dateFilter));
    if (!hasSelectedDate) {
      setDateFilter(workItemPrimaryDateKey(nextSelectedItem));
    }
  }, [dateFilter, selectedWorkItemId, snapshot]);

  useEffect(() => {
    if (!selectedWorkItemId) {
      setDetail(null);
      setDetailError(null);
      return;
    }

    let disposed = false;
    setDetailLoading(true);
    setDetailError(null);
    setActionError(null);
    setDetail(null);
    void getWorkItemDetail(selectedWorkItemId)
      .then((nextDetail) => {
        if (!disposed) setDetail(nextDetail);
      })
      .catch((nextError: unknown) => {
        if (!disposed) setDetailError(friendlyError(nextError));
      })
      .finally(() => {
        if (!disposed) setDetailLoading(false);
      });

    return () => {
      disposed = true;
    };
  }, [detailRevision, selectedWorkItemId]);

  const statusOptions = useMemo(
    () => [...new Set(snapshot?.work_items.map((item) => item.status) ?? [])].sort(),
    [snapshot],
  );
  const filteredItems = useMemo(() => {
    const normalizedQuery = query.trim().toLocaleLowerCase("ko-KR");
    return (snapshot?.work_items ?? []).filter((item) => {
      const matchesDate =
        dateFilter === null || workItemMatchesDate(item, dateFilter);
      const matchesStatus = statusFilter === "all" || item.status === statusFilter;
      const matchesQuery =
        normalizedQuery.length === 0 ||
        `${item.id} ${item.title} ${item.project_id} ${item.current_state ?? ""}`
          .toLocaleLowerCase("ko-KR")
          .includes(normalizedQuery);
      return matchesDate && matchesStatus && matchesQuery;
    });
  }, [dateFilter, query, snapshot, statusFilter]);

  useEffect(() => {
    if (
      filteredItems.length > 0 &&
      !filteredItems.some((item) => item.id === selectedWorkItemId)
    ) {
      setSelectedWorkItemId(filteredItems[0].id);
    }
  }, [filteredItems, selectedWorkItemId]);

  async function chooseRoot() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Work Harvest 데이터 폴더 선택",
      });
      if (typeof selected === "string") await applyRoot(selected);
    } catch (nextError) {
      setError(friendlyError(nextError));
    }
  }

  async function runExternalAction(action: () => Promise<void>) {
    setActionError(null);
    try {
      await action();
    } catch (nextError) {
      setActionError(friendlyError(nextError));
    }
  }

  async function handleWorkItemSaved(workItemId: string) {
    setDateFilter(null);
    setQuery("");
    setStatusFilter("all");
    selectedWorkItemIdRef.current = workItemId;
    setSelectedWorkItemId(workItemId);
    await refresh();
  }

  async function handleTrashWorkItem(workItemId: string) {
    setActionError(null);
    try {
      await trashWorkItem(workItemId, new Date().toISOString());
      selectedWorkItemIdRef.current = null;
      setSelectedWorkItemId(null);
      setDetail(null);
      await refresh();
      return true;
    } catch (nextError) {
      setActionError(friendlyError(nextError));
      return false;
    }
  }

  function handlePerformanceNoteCreated(report: string) {
    setEditor(null);
    void runExternalAction(() => openPerformanceNoteMarkdown(report));
  }

  function handleWeeklyReportCreated(report: string) {
    setEditor(null);
    void runExternalAction(() => openWeeklyReportMarkdown(report));
  }

  return {
    actionError,
    chooseRoot,
    dateFilter,
    detail,
    detailError,
    detailLoading,
    editor,
    enableNotifications,
    error,
    filteredItems,
    handlePerformanceNoteCreated,
    handleWeeklyReportCreated,
    handleWorkItemSaved,
    handleTrashWorkItem,
    indexActivity,
    lastUpdatedAt,
    loading,
    notificationError,
    notificationState,
    openCheckpoint: (checkpointId: string) =>
      runExternalAction(() => openCheckpointMarkdown(checkpointId)),
    openContext: (workItemId: string) =>
      runExternalAction(() => openContextMarkdown(workItemId)),
    openExternalUrl: (url: string) => runExternalAction(() => openExternalUrl(url)),
    query,
    refresh,
    revealWorkItem: (workItemId: string) =>
      runExternalAction(() => revealWorkItem(workItemId)),
    selectedWorkItemId,
    setEditor,
    setDateFilter,
    setQuery,
    setSelectedWorkItemId,
    setStatusFilter,
    snapshot,
    statusFilter,
    statusOptions,
  };
}

export type WorkspaceController = ReturnType<typeof useWorkspaceController>;
