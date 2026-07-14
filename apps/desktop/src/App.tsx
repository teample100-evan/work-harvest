import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  getWorkItemDetail,
  inspectDataRoot,
  setDataRoot,
  type DataRootSnapshot,
  type WorkItemDetail,
} from "./desktop";

const DATA_ROOT_KEY = "work-harvest:data-root";

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
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
      {items.map((item) => (
        <li key={item}>{item}</li>
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
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const refreshTimer = useRef<number | null>(null);

  const applyRoot = useCallback(async (root: string) => {
    setLoading(true);
    setError(null);
    try {
      const nextSnapshot = await setDataRoot(root);
      localStorage.setItem(DATA_ROOT_KEY, root);
      setSnapshot(nextSnapshot);
      setLastUpdatedAt(new Date());
    } catch (nextError) {
      setError(errorMessage(nextError));
    } finally {
      setLoading(false);
    }
  }, []);

  const refresh = useCallback(async () => {
    try {
      const nextSnapshot = await inspectDataRoot();
      setSnapshot(nextSnapshot);
      setError(null);
      setLastUpdatedAt(new Date());
    } catch (nextError) {
      setError(errorMessage(nextError));
    }
  }, []);

  useEffect(() => {
    const savedRoot = localStorage.getItem(DATA_ROOT_KEY);
    if (savedRoot) {
      void applyRoot(savedRoot);
    } else {
      setLoading(false);
    }
  }, [applyRoot]);

  useEffect(() => {
    let disposed = false;
    let unlisten: Array<() => void> = [];
    void Promise.all([
      listen("data-root-changed", () => {
        if (refreshTimer.current !== null) {
          window.clearTimeout(refreshTimer.current);
        }
        refreshTimer.current = window.setTimeout(() => {
          void refresh();
        }, 350);
      }),
      listen<string>("data-root-watch-error", (event) => {
        setError(`파일 변경 감시에 실패했습니다: ${event.payload}`);
      }),
    ]).then((stopListening) => {
      if (disposed) {
        stopListening.forEach((stop) => stop());
      } else {
        unlisten = stopListening;
      }
    });

    return () => {
      disposed = true;
      unlisten.forEach((stop) => stop());
      if (refreshTimer.current !== null) {
        window.clearTimeout(refreshTimer.current);
      }
    };
  }, [refresh]);

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
    setDetail(null);
    void getWorkItemDetail(selectedWorkItemId)
      .then((nextDetail) => {
        if (!disposed) setDetail(nextDetail);
      })
      .catch((nextError: unknown) => {
        if (!disposed) setDetailError(errorMessage(nextError));
      })
      .finally(() => {
        if (!disposed) setDetailLoading(false);
      });

    return () => {
      disposed = true;
    };
  }, [lastUpdatedAt, selectedWorkItemId]);

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
      setError(errorMessage(nextError));
    }
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
            <button className="button secondary" onClick={() => void refresh()}>
              다시 검사
            </button>
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

      {error && <section className="alert error">{error}</section>}

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
                  </div>
                ))}
              </div>
            )}
          </article>

          <footer>
            외부 파일 변경을 감시하고 있습니다.
            {lastUpdatedAt && ` 마지막 확인 ${lastUpdatedAt.toLocaleTimeString("ko-KR")}`}
          </footer>
        </>
      )}
    </main>
  );
}
