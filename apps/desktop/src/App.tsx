import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  getWorkItemDetail,
  inspectDataRoot,
  openCheckpointMarkdown,
  openContextMarkdown,
  revealWorkItem,
  setDataRoot,
  type DataRootChange,
  type DataRootSnapshot,
  type WorkItemDetail,
} from "./desktop";
import { AlwaysOnStatus } from "./AlwaysOnStatus";
import { CheckpointDetails } from "./CheckpointDetails";
import { useSnapshotNotifications } from "./useSnapshotNotifications";
import { WorkItemEditor } from "./WorkItemEditor";

const DATA_ROOT_KEY = "work-harvest:data-root";

interface IndexActivity {
  revision: number;
  reloadedFiles: number;
  eventCount: number | null;
  pathCount: number | null;
  fullRescan: boolean;
}

type EditorState =
  | { mode: "create" }
  | { mode: "edit"; workItemId: string };

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function friendlyError(error: unknown) {
  const message = errorMessage(error);
  if (/permission denied|operation not permitted/i.test(message)) {
    return "폴더 또는 파일 접근 권한이 없습니다. Finder에서 권한을 확인하거나 데이터 폴더를 다시 선택하세요.";
  }
  if (/not found|does not exist/i.test(message)) {
    return "연결된 파일을 찾을 수 없습니다. 외부에서 이동하거나 삭제했는지 확인한 뒤 다시 검사하세요.";
  }
  if (/could not open|could not reveal/i.test(message)) {
    return "Finder 또는 기본 Markdown 앱으로 파일을 열지 못했습니다. 연결 프로그램 설정을 확인하세요.";
  }
  return message;
}

function issueGuidance(code: string) {
  if (code === "schema_validation") {
    return "해당 JSON 필드를 공통 스키마에 맞게 수정한 뒤 다시 검사하세요.";
  }
  if (code === "invalid_json") {
    return "JSON 문법 오류를 수정하면 파일 감지가 자동으로 다시 검사합니다.";
  }
  if (code === "read_failed" || code === "scan_failed") {
    return "Finder에서 파일 권한을 확인하거나 데이터 폴더를 다시 선택하세요.";
  }
  if (code.startsWith("missing_")) {
    return "원본 JSON과 파생 Markdown 생성 상태를 확인하세요.";
  }
  if (code.includes("mismatch") || code.startsWith("unknown_")) {
    return "업무·프로젝트·체크포인트 ID의 연결 관계를 확인하세요.";
  }
  return "파일 경로를 확인하고 원본 기록을 수정한 뒤 다시 검사하세요.";
}

function formatTimestamp(value: string) {
  if (!value) return "기록 없음";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("ko-KR", {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function DetailList({ items, empty }: { items: string[]; empty: string }) {
  if (items.length === 0) return <p className="muted compact">{empty}</p>;
  return (
    <ul className="detail-list">
      {items.map((item, index) => (
        <li key={`${item}-${index}`}>{item}</li>
      ))}
    </ul>
  );
}

export function App() {
  const [snapshot, setSnapshot] = useState<DataRootSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdatedAt, setLastUpdatedAt] = useState<Date | null>(null);
  const [selectedWorkItemId, setSelectedWorkItemId] = useState<string | null>(null);
  const [detail, setDetail] = useState<WorkItemDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
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
      setSelectedWorkItemId(null);
      return;
    }
    if (!snapshot.work_items.some((item) => item.id === selectedWorkItemId)) {
      setSelectedWorkItemId(snapshot.work_items[0].id);
    }
  }, [selectedWorkItemId, snapshot]);

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
      const matchesStatus = statusFilter === "all" || item.status === statusFilter;
      const matchesQuery =
        normalizedQuery.length === 0 ||
        `${item.id} ${item.title} ${item.project_id} ${item.current_state ?? ""}`
          .toLocaleLowerCase("ko-KR")
          .includes(normalizedQuery);
      return matchesStatus && matchesQuery;
    });
  }, [query, snapshot, statusFilter]);

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
      if (typeof selected === "string") {
        await applyRoot(selected);
      }
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
    setQuery("");
    setStatusFilter("all");
    selectedWorkItemIdRef.current = workItemId;
    setSelectedWorkItemId(workItemId);
    await refresh();
  }

  const hasErrors = snapshot?.issues.some((issue) => issue.severity === "error");

  return (
    <main className="app-shell">
      <header className="topbar">
        <div>
          <p className="eyebrow">LOCAL WORK RECORDS</p>
          <h1>Work Harvest</h1>
        </div>
        <div className="topbar-actions">
          {snapshot && (
            <>
              <button className="button secondary" onClick={() => setEditor({ mode: "create" })}>
                새 업무
              </button>
              <button className="button secondary" onClick={() => void refresh()}>
                다시 검사
              </button>
            </>
          )}
          <button className="button primary" onClick={() => void chooseRoot()}>
            {snapshot ? "폴더 변경" : "데이터 폴더 선택"}
          </button>
        </div>
      </header>

      {loading && <section className="panel empty-state">데이터를 확인하는 중입니다.</section>}

      {!loading && !snapshot && (
        <section className="panel empty-state">
          <div className="empty-mark">WH</div>
          <h2>기록이 있는 폴더를 연결하세요</h2>
          <p>
            기본 위치는 <code>~/work-records</code>입니다. 앱은 선택한 폴더를 읽고 외부 변경을 감시합니다.
          </p>
          <button className="button primary" onClick={() => void chooseRoot()}>
            폴더 선택
          </button>
        </section>
      )}

      {error && (
        <section className="alert error recovery-alert">
          <span>{error}</span>
          <button className="inline-action" onClick={() => void chooseRoot()} type="button">
            폴더 다시 선택
          </button>
        </section>
      )}

      {snapshot && (
        <>
          <section className="root-summary">
            <div>
              <p className="section-label">데이터 루트</p>
              <p className="root-path">{snapshot.root}</p>
            </div>
            <div className={`health ${hasErrors ? "unhealthy" : "healthy"}`}>
              <span className="health-dot" />
              {hasErrors ? `오류 ${snapshot.issues.length}개` : "전체 스키마 정상"}
            </div>
          </section>

          <AlwaysOnStatus
            notificationError={notificationError}
            notificationState={notificationState}
            onEnableNotifications={enableNotifications}
          />

          <section className="metric-grid" aria-label="데이터 수량">
            <article className="metric-card">
              <span>업무 항목</span>
              <strong>{snapshot.counts.work_items}</strong>
            </article>
            <article className="metric-card">
              <span>현재 Context</span>
              <strong>{snapshot.counts.contexts}</strong>
            </article>
            <article className="metric-card">
              <span>체크포인트</span>
              <strong>{snapshot.counts.checkpoints}</strong>
            </article>
          </section>

          <section className="workspace-grid">
            <article className="panel work-browser">
              <div className="panel-heading">
                <div>
                  <p className="section-label">업무 탐색</p>
                  <h2>업무 항목</h2>
                </div>
                <span>{filteredItems.length}개</span>
              </div>
              <div className="filter-row">
                <input
                  aria-label="업무 항목 검색"
                  onChange={(event) => setQuery(event.target.value)}
                  placeholder="ID, 제목, 상태 검색"
                  type="search"
                  value={query}
                />
                <select
                  aria-label="업무 상태 필터"
                  onChange={(event) => setStatusFilter(event.target.value)}
                  value={statusFilter}
                >
                  <option value="all">모든 상태</option>
                  {statusOptions.map((status) => (
                    <option key={status} value={status}>
                      {status}
                    </option>
                  ))}
                </select>
              </div>
              {filteredItems.length === 0 ? (
                <p className="muted">조건에 맞는 업무 항목이 없습니다.</p>
              ) : (
                <div className="work-item-list">
                  {filteredItems.map((item) => (
                    <button
                      className={`work-item-button ${selectedWorkItemId === item.id ? "selected" : ""}`}
                      key={item.id}
                      onClick={() => setSelectedWorkItemId(item.id)}
                      type="button"
                    >
                      <div className="work-item-row">
                        <div>
                          <div className="work-item-title">
                            <strong>{item.id}</strong>
                            <span>{item.title}</span>
                          </div>
                          <p>{item.current_state ?? "현재 상태가 기록되지 않았습니다."}</p>
                        </div>
                        <span className={`status status-${item.status}`}>{item.status}</span>
                      </div>
                    </button>
                  ))}
                </div>
              )}
            </article>

            <article className="panel detail-panel">
              {detailLoading && <p className="muted">업무 상세를 불러오는 중입니다.</p>}
              {detailError && <div className="alert error">{detailError}</div>}
              {!detailLoading && !detail && !detailError && (
                <div className="detail-empty">
                  <p className="muted">업무 항목을 선택하면 현재 상태와 기록 흐름을 보여줍니다.</p>
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
                      className="inline-action"
                      onClick={() => setEditor({ mode: "edit", workItemId: detail.id })}
                      type="button"
                    >
                      업무 편집
                    </button>
                    <button
                      className="inline-action"
                      onClick={() =>
                        void runExternalAction(() => revealWorkItem(detail.id))
                      }
                      type="button"
                    >
                      Finder에서 보기
                    </button>
                    <button
                      className="inline-action"
                      onClick={() =>
                        void runExternalAction(() => openContextMarkdown(detail.id))
                      }
                      type="button"
                    >
                      Context.md 열기
                    </button>
                  </div>
                  {actionError && <div className="alert error compact-alert">{actionError}</div>}

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
                                onOpenMarkdown={(checkpointId) =>
                                  void runExternalAction(() =>
                                    openCheckpointMarkdown(checkpointId),
                                  )
                                }
                              />
                            </div>
                          </article>
                        ))}
                      </div>
                    )}
                  </section>

                  <p className="detail-updated">
                    마지막 업무 갱신 {formatTimestamp(detail.updated_at)}
                  </p>
                </div>
              )}
            </article>
          </section>

          <article className="panel issue-panel">
            <div className="panel-heading">
              <div>
                <p className="section-label">Draft 2020-12 · 파일 관계 검사</p>
                <h2>발견한 문제</h2>
              </div>
              <span>{snapshot.issues.length}개</span>
            </div>
            {snapshot.issues.length === 0 ? (
              <p className="muted">JSON Schema와 파일 간 관계 검사를 통과했습니다.</p>
            ) : (
              <div className="issue-list">
                {snapshot.issues.slice(0, 20).map((issue) => (
                  <div
                    className="issue"
                    key={`${issue.path}-${issue.code}-${issue.message}`}
                  >
                    <strong>{issue.message}</strong>
                    <code>{issue.path}</code>
                    <p>{issueGuidance(issue.code)}</p>
                  </div>
                ))}
              </div>
            )}
          </article>

          <footer>
            외부 파일 변경을 감시하고 있습니다.
            {lastUpdatedAt && ` 마지막 확인 ${lastUpdatedAt.toLocaleTimeString("ko-KR")}`}
            {indexActivity &&
              ` · 인덱스 r${indexActivity.revision} · ${
                indexActivity.fullRescan ? "전체" : "증분"
              } 검사 · JSON ${indexActivity.reloadedFiles}개${
                indexActivity.eventCount === null
                  ? ""
                  : ` · 이벤트 ${indexActivity.eventCount}건 → 경로 ${indexActivity.pathCount}개`
              }`}
          </footer>
        </>
      )}

      {editor ? (
        <WorkItemEditor
          mode={editor.mode}
          workItemId={editor.mode === "edit" ? editor.workItemId : undefined}
          onClose={() => setEditor(null)}
          onSaved={handleWorkItemSaved}
        />
      ) : null}
    </main>
  );
}
