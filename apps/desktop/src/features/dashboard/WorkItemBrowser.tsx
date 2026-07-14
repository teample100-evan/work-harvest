import type { WorkItemSummary } from "../../desktop";

interface WorkItemBrowserProps {
  items: WorkItemSummary[];
  query: string;
  selectedWorkItemId: string | null;
  statusFilter: string;
  statusOptions: string[];
  onQueryChange: (query: string) => void;
  onSelect: (workItemId: string) => void;
  onStatusFilterChange: (status: string) => void;
}

export function WorkItemBrowser({
  items,
  query,
  selectedWorkItemId,
  statusFilter,
  statusOptions,
  onQueryChange,
  onSelect,
  onStatusFilterChange,
}: WorkItemBrowserProps) {
  return (
    <article className="panel work-browser">
      <div className="panel-heading">
        <div>
          <p className="section-label">업무 탐색</p>
          <h2>이어갈 업무</h2>
        </div>
        <span>{items.length}개</span>
      </div>
      <div className="filter-row">
        <input
          aria-label="업무 항목 검색"
          onChange={(event) => onQueryChange(event.target.value)}
          placeholder="ID, 제목, 현재 상태 검색"
          type="search"
          value={query}
        />
        <select
          aria-label="업무 상태 필터"
          onChange={(event) => onStatusFilterChange(event.target.value)}
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
      {items.length === 0 ? (
        <p className="muted">조건에 맞는 업무 항목이 없습니다.</p>
      ) : (
        <div className="work-item-list">
          {items.map((item) => (
            <button
              aria-current={selectedWorkItemId === item.id ? "true" : undefined}
              className={`work-item-button ${selectedWorkItemId === item.id ? "selected" : ""}`}
              key={item.id}
              onClick={() => onSelect(item.id)}
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
  );
}
