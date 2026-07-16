import { Search } from "lucide-react";
import type { WorkItemSummary } from "../../desktop";
import { SelectMenu, type SelectMenuOption } from "../../ui/SelectMenu";
import { formatWorkItemStatus, needsWorkItemStatusBadge } from "./presentation";

interface WorkItemBrowserProps {
  items: WorkItemSummary[];
  query: string;
  statusFilter: string;
  statusOptions: string[];
  onQueryChange: (query: string) => void;
  onSelect: (workItemId: string) => void;
  onStatusFilterChange: (status: string) => void;
}

export function WorkItemBrowser({
  items,
  query,
  statusFilter,
  statusOptions,
  onQueryChange,
  onSelect,
  onStatusFilterChange,
}: WorkItemBrowserProps) {
  const filterOptions: Array<SelectMenuOption<string>> = ["all", ...statusOptions].map(
    (status) => ({
      value: status,
      label: status === "all" ? "모든 상태" : formatWorkItemStatus(status),
    }),
  );

  return (
    <section className="daily-work-browser" aria-labelledby="daily-work-list-title">
      <div className="daily-work-heading">
        <h2 id="daily-work-list-title">업무 목록</h2>
        <span>{items.length}개</span>
      </div>

      <div className="daily-filter-row">
        <label className="daily-search">
          <Search aria-hidden="true" size={15} strokeWidth={1.8} />
          <input
            aria-label="선택한 날짜의 업무 검색"
            onChange={(event) => onQueryChange(event.target.value)}
            placeholder="업무 검색"
            type="search"
            value={query}
          />
        </label>
        <SelectMenu
          ariaLabel="선택한 날짜의 업무 상태 필터"
          className="daily-status-menu"
          onChange={onStatusFilterChange}
          options={filterOptions}
          value={statusFilter}
        />
      </div>

      {items.length === 0 ? (
        <div className="daily-work-empty">
          <p>조건에 맞는 업무가 없습니다.</p>
          <span>검색어나 상태 조건을 변경해보세요.</span>
        </div>
      ) : (
        <div className="daily-work-items">
          {items.map((item) => (
            <button
              className="daily-work-button"
              key={item.id}
              onClick={() => onSelect(item.id)}
              title={item.current_state ?? item.title}
              type="button"
            >
              <span className="daily-work-title">{item.title}</span>
              <span className="daily-work-meta">
                <span>
                  {item.project_id} · {formatWorkItemStatus(item.status)}
                </span>
                {needsWorkItemStatusBadge(item.status) && (
                  <span className={`status status-${item.status}`}>
                    {formatWorkItemStatus(item.status)}
                  </span>
                )}
              </span>
            </button>
          ))}
        </div>
      )}
    </section>
  );
}
