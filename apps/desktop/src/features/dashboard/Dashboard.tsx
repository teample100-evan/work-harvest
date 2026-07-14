import { Button } from "../../ui/Button";
import { EditorHost } from "./EditorHost";
import { SystemOverview } from "./SystemOverview";
import { WorkItemBrowser } from "./WorkItemBrowser";
import { WorkItemDetailPanel } from "./WorkItemDetailPanel";
import type { WorkspaceController } from "./useWorkspaceController";

interface DashboardProps {
  controller: WorkspaceController;
}

export function Dashboard({ controller }: DashboardProps) {
  const { snapshot } = controller;
  const errorCount = snapshot?.issues.filter((issue) => issue.severity === "error").length ?? 0;
  const hasVisibleWorkItem = controller.filteredItems.some(
    (item) => item.id === controller.selectedWorkItemId,
  );

  return (
    <main className="app-shell">
      <header className="topbar">
        <div>
          <p className="eyebrow">LOCAL WORK RECORDS</p>
          <h1>Work Harvest</h1>
        </div>
        <div className="topbar-actions">
          {snapshot ? (
            <>
              <Button variant="primary" onClick={() => controller.setEditor({ mode: "create" })}>
                새 업무
              </Button>
              <Button variant="secondary" onClick={() => void controller.refresh()}>
                다시 검사
              </Button>
              <Button variant="ghost" onClick={() => void controller.chooseRoot()}>
                폴더 변경
              </Button>
            </>
          ) : (
            <Button variant="primary" onClick={() => void controller.chooseRoot()}>
              데이터 폴더 선택
            </Button>
          )}
        </div>
      </header>

      {controller.loading && (
        <section className="panel empty-state" role="status">
          데이터 확인 중…
        </section>
      )}

      {!controller.loading && !snapshot && (
        <section className="panel empty-state">
          <div className="empty-mark">WH</div>
          <h2>기록이 있는 폴더를 연결하세요</h2>
          <p>
            기본 위치는 <code>~/work-records</code>입니다. 앱은 선택한 폴더를 읽고 외부 변경을
            감시합니다.
          </p>
          <Button variant="primary" onClick={() => void controller.chooseRoot()}>
            폴더 선택
          </Button>
        </section>
      )}

      {controller.error && (
        <section className="alert error recovery-alert" role="alert">
          <span>{controller.error}</span>
          <button
            className="inline-action"
            onClick={() => void controller.chooseRoot()}
            type="button"
          >
            폴더 다시 선택
          </button>
        </section>
      )}

      {snapshot && (
        <>
          <section className="root-summary" aria-label="현재 작업 공간">
            <div>
              <p className="section-label">현재 작업 공간</p>
              <p className="root-path">{snapshot.root}</p>
            </div>
            <div
              className={`health ${errorCount > 0 ? "unhealthy" : "healthy"}`}
              role="status"
            >
              <span className="health-dot" />
              {errorCount > 0 ? `오류 ${errorCount}개` : "파일 검증 정상"}
            </div>
          </section>

          <section className="workspace-grid" aria-label="업무 작업 공간">
            <WorkItemBrowser
              items={controller.filteredItems}
              query={controller.query}
              selectedWorkItemId={controller.selectedWorkItemId}
              statusFilter={controller.statusFilter}
              statusOptions={controller.statusOptions}
              onQueryChange={controller.setQuery}
              onSelect={controller.setSelectedWorkItemId}
              onStatusFilterChange={controller.setStatusFilter}
            />
            <WorkItemDetailPanel
              actionError={controller.actionError}
              detail={hasVisibleWorkItem ? controller.detail : null}
              detailError={hasVisibleWorkItem ? controller.detailError : null}
              detailLoading={hasVisibleWorkItem && controller.detailLoading}
              emptyMessage={
                controller.filteredItems.length === 0
                  ? "검색어나 상태 조건을 바꾸면 업무 상세를 다시 볼 수 있습니다."
                  : undefined
              }
              onAddCheckpoint={(workItemId) =>
                controller.setEditor({ mode: "checkpoint", workItemId })
              }
              onCreatePerformanceNote={(workItemId) =>
                controller.setEditor({ mode: "performance-note", workItemId })
              }
              onEdit={(workItemId) => controller.setEditor({ mode: "edit", workItemId })}
              onOpenCheckpoint={(checkpointId) => void controller.openCheckpoint(checkpointId)}
              onOpenContext={(workItemId) => void controller.openContext(workItemId)}
              onReveal={(workItemId) => void controller.revealWorkItem(workItemId)}
            />
          </section>

          <SystemOverview
            indexActivity={controller.indexActivity}
            lastUpdatedAt={controller.lastUpdatedAt}
            notificationError={controller.notificationError}
            notificationState={controller.notificationState}
            snapshot={snapshot}
            onEnableNotifications={controller.enableNotifications}
          />
        </>
      )}

      <EditorHost controller={controller} />
    </main>
  );
}
