import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  inspectDataRoot,
  setDataRoot,
  type DataRootSnapshot,
} from "./desktop";

const DATA_ROOT_KEY = "work-harvest:data-root";

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export function App() {
  const [snapshot, setSnapshot] = useState<DataRootSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdatedAt, setLastUpdatedAt] = useState<Date | null>(null);
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
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void listen("data-root-changed", () => {
      if (refreshTimer.current !== null) {
        window.clearTimeout(refreshTimer.current);
      }
      refreshTimer.current = window.setTimeout(() => {
        void refresh();
      }, 350);
    }).then((stopListening) => {
      if (disposed) {
        stopListening();
      } else {
        unlisten = stopListening;
      }
    });

    return () => {
      disposed = true;
      unlisten?.();
      if (refreshTimer.current !== null) {
        window.clearTimeout(refreshTimer.current);
      }
    };
  }, [refresh]);

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
              {hasErrors ? `오류 ${snapshot.issues.length}개` : "기본 구조 정상"}
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

          <section className="content-grid">
            <article className="panel">
              <div className="panel-heading">
                <div>
                  <p className="section-label">최근 갱신 순</p>
                  <h2>업무 항목</h2>
                </div>
                <span>{snapshot.work_items.length}개</span>
              </div>
              {snapshot.work_items.length === 0 ? (
                <p className="muted">아직 확인할 수 있는 업무 항목이 없습니다.</p>
              ) : (
                <div className="work-item-list">
                  {snapshot.work_items.slice(0, 8).map((item) => (
                    <div className="work-item-row" key={item.id}>
                      <div>
                        <div className="work-item-title">
                          <strong>{item.id}</strong>
                          <span>{item.title}</span>
                        </div>
                        <p>{item.current_state ?? "현재 상태가 기록되지 않았습니다."}</p>
                      </div>
                      <span className={`status status-${item.status}`}>{item.status}</span>
                    </div>
                  ))}
                </div>
              )}
            </article>

            <article className="panel">
              <div className="panel-heading">
                <div>
                  <p className="section-label">읽기 전용 검사</p>
                  <h2>발견한 문제</h2>
                </div>
                <span>{snapshot.issues.length}개</span>
              </div>
              {snapshot.issues.length === 0 ? (
                <p className="muted">기본 파일 구조와 JSON 읽기 검사를 통과했습니다.</p>
              ) : (
                <div className="issue-list">
                  {snapshot.issues.slice(0, 10).map((issue) => (
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
          </section>

          <footer>
            외부 파일 변경을 감시하고 있습니다.
            {lastUpdatedAt && ` 마지막 확인 ${lastUpdatedAt.toLocaleTimeString("ko-KR")}`}
          </footer>
        </>
      )}
    </main>
  );
}
