import { ArrowLeft, CalendarDays, Plus } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Button } from "../../ui/Button";
import { EditorHost } from "./EditorHost";
import { WorkDateNavigation } from "./WorkDateNavigation";
import { WorkItemBrowser } from "./WorkItemBrowser";
import { WorkItemDetailPanel } from "./WorkItemDetailPanel";
import { WorkspaceEnvironmentMenu } from "./WorkspaceEnvironmentMenu";
import { formatWorkDateLong } from "./workItemDates";
import type { WorkspaceController } from "./useWorkspaceController";

interface DashboardProps {
  controller: WorkspaceController;
}

export function Dashboard({ controller }: DashboardProps) {
  const mainScrollRef = useRef<HTMLDivElement>(null);
  const listScrollTopRef = useRef(0);
  const [mainView, setMainView] = useState<"list" | "detail">("list");
  const { snapshot } = controller;
  const errorCount = snapshot?.issues.filter((issue) => issue.severity === "error").length ?? 0;
  const hasVisibleWorkItem = controller.filteredItems.some(
    (item) => item.id === controller.selectedWorkItemId,
  );

  useEffect(() => {
    if (mainView === "detail") {
      mainScrollRef.current?.scrollTo({ top: 0 });
      return;
    }

    requestAnimationFrame(() => {
      mainScrollRef.current?.scrollTo({ top: listScrollTopRef.current });
    });
  }, [mainView]);

  function handleDateSelect(dateKey: string) {
    controller.setQuery("");
    controller.setStatusFilter("all");
    controller.setDateFilter(dateKey);
    setMainView("list");
  }

  function handleWorkItemSelect(workItemId: string) {
    listScrollTopRef.current = mainScrollRef.current?.scrollTop ?? 0;
    controller.setSelectedWorkItemId(workItemId);
    setMainView("detail");
  }

  return (
    <div className={`app-shell ${snapshot ? "workspace-layout" : "onboarding-layout"}`}>
      {snapshot ? (
        <>
          <aside className="app-sidebar">
            <header className="sidebar-brand">
              <div>
                <p className="eyebrow">LOCAL WORK RECORDS</p>
                <h1>Work Harvest</h1>
              </div>
            </header>

            <Button
              className="sidebar-create-button"
              variant="ghost"
              onClick={() => controller.setEditor({ mode: "create" })}
            >
              <Plus aria-hidden="true" size={16} strokeWidth={1.9} />
              새 업무
            </Button>

            <WorkDateNavigation
              items={snapshot.work_items}
              selectedDateKey={controller.dateFilter}
              onSelect={handleDateSelect}
            />

            <WorkspaceEnvironmentMenu
              errorCount={errorCount}
              notificationError={controller.notificationError}
              notificationState={controller.notificationState}
              snapshot={snapshot}
              onChooseRoot={() => void controller.chooseRoot()}
              onEnableNotifications={controller.enableNotifications}
              onRefresh={() => void controller.refresh()}
            />
          </aside>

          <main className={`main-pane ${mainView === "detail" ? "detail-view" : "list-view"}`}>
            <header className="main-topbar">
              {mainView === "list" ? (
                <>
                  <div className="main-topbar-title">
                    <CalendarDays aria-hidden="true" size={17} strokeWidth={1.8} />
                    <div>
                      <h2>
                        {controller.dateFilter
                          ? formatWorkDateLong(controller.dateFilter)
                          : "날짜를 선택하세요"}
                      </h2>
                    </div>
                  </div>
                </>
              ) : (
                <button className="main-back-button" onClick={() => setMainView("list")} type="button">
                  <ArrowLeft aria-hidden="true" size={17} strokeWidth={1.8} />
                  <span>
                    <small>
                      {controller.dateFilter ? formatWorkDateLong(controller.dateFilter) : "업무 목록"}
                    </small>
                    <strong>업무 목록으로 돌아가기</strong>
                  </span>
                </button>
              )}
            </header>

            <div className="main-scroll" ref={mainScrollRef}>
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

              {mainView === "list" ? (
                <section className="daily-workspace" aria-label="선택한 날짜의 업무 목록">
                  <WorkItemBrowser
                    items={controller.filteredItems}
                    query={controller.query}
                    statusFilter={controller.statusFilter}
                    statusOptions={controller.statusOptions}
                    onQueryChange={controller.setQuery}
                    onSelect={handleWorkItemSelect}
                    onStatusFilterChange={controller.setStatusFilter}
                  />
                </section>
              ) : (
                <section className="detail-stage detail-page" aria-label="선택한 업무 상세">
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
                    onOpenCheckpoint={(checkpointId) =>
                      void controller.openCheckpoint(checkpointId)
                    }
                    onOpenExternalUrl={(url) => void controller.openExternalUrl(url)}
                    onOpenContext={(workItemId) => void controller.openContext(workItemId)}
                    onReveal={(workItemId) => void controller.revealWorkItem(workItemId)}
                  />
                </section>
              )}
            </div>
          </main>
        </>
      ) : (
        <main className="onboarding-main">
          <header className="topbar">
            <div>
              <p className="eyebrow">LOCAL WORK RECORDS</p>
              <h1>Work Harvest</h1>
            </div>
            <Button variant="primary" onClick={() => void controller.chooseRoot()}>
              데이터 폴더 선택
            </Button>
          </header>

          {controller.loading && (
            <section className="panel empty-state" role="status">
              데이터 확인 중…
            </section>
          )}

          {!controller.loading && (
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
        </main>
      )}

      <EditorHost controller={controller} />
    </div>
  );
}
