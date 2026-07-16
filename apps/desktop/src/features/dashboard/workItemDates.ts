import type { WorkItemSummary } from "../../desktop";

export const UNDATED_KEY = "undated";

function localDateParts(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return null;
  return {
    date,
    key: [date.getFullYear(), date.getMonth() + 1, date.getDate()]
      .map((part, index) => (index === 0 ? String(part) : String(part).padStart(2, "0")))
      .join("-"),
  };
}

export function workItemDateKey(value: string) {
  return localDateParts(value)?.key ?? UNDATED_KEY;
}

export function formatWorkDateLong(key: string) {
  if (key === UNDATED_KEY) return "날짜 없는 업무";
  const date = new Date(`${key}T00:00:00`);
  return date.toLocaleDateString("ko-KR", {
    year: "numeric",
    month: "long",
    day: "numeric",
    weekday: "long",
  });
}

export interface WorkDateEntry {
  key: string;
  dayLabel: string;
  weekdayLabel: string;
  count: number;
}

export interface WorkMonthGroup {
  key: string;
  label: string;
  dates: WorkDateEntry[];
}

export function groupWorkItemDates(items: WorkItemSummary[]) {
  const dateCounts = new Map<string, number>();
  for (const item of items) {
    const key = workItemDateKey(item.updated_at);
    dateCounts.set(key, (dateCounts.get(key) ?? 0) + 1);
  }

  const orderedKeys = [...dateCounts.keys()].sort((left, right) => {
    if (left === UNDATED_KEY) return 1;
    if (right === UNDATED_KEY) return -1;
    return right.localeCompare(left);
  });
  const groups = new Map<string, WorkMonthGroup>();

  for (const key of orderedKeys) {
    if (key === UNDATED_KEY) {
      groups.set(UNDATED_KEY, {
        key: UNDATED_KEY,
        label: "기타",
        dates: [{ key, dayLabel: "날짜 없음", weekdayLabel: "", count: dateCounts.get(key) ?? 0 }],
      });
      continue;
    }

    const date = new Date(`${key}T00:00:00`);
    const monthKey = key.slice(0, 7);
    const existing = groups.get(monthKey);
    const entry: WorkDateEntry = {
      key,
      dayLabel: key.replaceAll("-", "/"),
      weekdayLabel: date.toLocaleDateString("ko-KR", { weekday: "short" }),
      count: dateCounts.get(key) ?? 0,
    };

    if (existing) {
      existing.dates.push(entry);
    } else {
      groups.set(monthKey, {
        key: monthKey,
        label: date.toLocaleDateString("ko-KR", { year: "numeric", month: "long" }),
        dates: [entry],
      });
    }
  }

  return [...groups.values()];
}
