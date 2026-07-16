import type { WorkItemSummary } from "../../desktop";
import { groupWorkItemDates } from "./workItemDates";

interface WorkDateNavigationProps {
  items: WorkItemSummary[];
  selectedDateKey: string | null;
  onSelect: (dateKey: string) => void;
}

export function WorkDateNavigation({
  items,
  selectedDateKey,
  onSelect,
}: WorkDateNavigationProps) {
  const monthGroups = groupWorkItemDates(items);

  return (
    <nav className="work-date-navigation" aria-label="작업 기록 날짜 탐색">
      {monthGroups.map((month) => (
        <section className="date-month-group" key={month.key}>
          <h2>{month.label}</h2>
          <div className="date-list">
            {month.dates.map((date) => (
              <button
                aria-current={selectedDateKey === date.key ? "date" : undefined}
                className={`date-button ${selectedDateKey === date.key ? "selected" : ""}`}
                key={date.key}
                onClick={() => onSelect(date.key)}
                type="button"
              >
                <span className="date-label">
                  <strong>{date.dayLabel}</strong>
                  {date.weekdayLabel && <span>{date.weekdayLabel}</span>}
                </span>
                <span className="date-count" aria-label={`업무 ${date.count}개`}>
                  {date.count}
                </span>
              </button>
            ))}
          </div>
        </section>
      ))}
    </nav>
  );
}
