import { RefreshCw, RotateCcw, Trash2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import {
  listTrashedWorkItems,
  restoreWorkItem,
  type TrashedWorkItem,
} from "../../desktop";
import { Button } from "../../ui/Button";
import { friendlyError } from "./presentation";

interface TrashPageProps {
  onWorkspaceRefresh: () => Promise<void>;
}

const trashedAtFormatter = new Intl.DateTimeFormat("ko-KR", {
  dateStyle: "medium",
  timeStyle: "short",
});

function formatTrashedAt(value: string) {
  return trashedAtFormatter.format(new Date(value));
}

export function TrashPage({ onWorkspaceRefresh }: TrashPageProps) {
  const [items, setItems] = useState<TrashedWorkItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [restoringId, setRestoringId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setItems(await listTrashedWorkItems());
    } catch (nextError) {
      setError(friendlyError(nextError));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function restore(item: TrashedWorkItem) {
    if (!window.confirm(`“${item.title}” 업무와 연결된 체크포인트를 복구할까요?`)) return;
    setRestoringId(item.work_item_id);
    setError(null);
    try {
      await restoreWorkItem(item.work_item_id);
      setItems((current) => current.filter((entry) => entry.work_item_id !== item.work_item_id));
      await onWorkspaceRefresh();
    } catch (nextError) {
      setError(friendlyError(nextError));
    } finally {
      setRestoringId(null);
    }
  }

  return (
    <section className="trash-page" aria-labelledby="trash-page-title">
      <div className="trash-page-heading">
        <div>
          <p className="section-label">복구 가능한 기록</p>
          <h2 id="trash-page-title">업무 휴지통</h2>
          <p>업무와 연결된 Context·체크포인트를 함께 보관합니다.</p>
        </div>
        <Button disabled={loading} size="sm" variant="secondary" onClick={() => void load()}>
          <RefreshCw aria-hidden="true" size={14} strokeWidth={1.8} />
          새로고침
        </Button>
      </div>

      {error ? (
        <div className="alert error compact-alert" role="alert">
          {error}
        </div>
      ) : null}

      {loading ? (
        <div className="trash-page-state" role="status">
          휴지통을 확인하는 중입니다…
        </div>
      ) : error ? null : items.length === 0 ? (
        <div className="trash-page-empty">
          <span className="trash-page-empty-icon">
            <Trash2 aria-hidden="true" size={24} strokeWidth={1.6} />
          </span>
          <h3>휴지통이 비어 있습니다</h3>
          <p>삭제한 업무는 완전히 지우지 않고 이곳에 보관됩니다.</p>
        </div>
      ) : (
        <div className="trash-page-list" aria-label={`휴지통 업무 ${items.length}개`}>
          {items.map((item) => (
            <article className="trash-page-item" key={item.work_item_id}>
              <div className="trash-page-item-copy">
                <span className="trash-page-item-icon">
                  <Trash2 aria-hidden="true" size={17} strokeWidth={1.7} />
                </span>
                <span>
                  <strong>{item.title}</strong>
                  <small>
                    {item.work_item_id} · {formatTrashedAt(item.trashed_at)} · 파일 {item.paths.length}개
                  </small>
                </span>
              </div>
              <Button
                disabled={restoringId !== null}
                size="sm"
                variant="secondary"
                onClick={() => void restore(item)}
              >
                <RotateCcw aria-hidden="true" size={14} strokeWidth={1.8} />
                {restoringId === item.work_item_id ? "복구 중…" : "복구"}
              </Button>
            </article>
          ))}
        </div>
      )}

      <p className="trash-page-note">
        현재는 영구 삭제를 지원하지 않습니다. 복구할 때 원래 위치에 같은 파일이 있으면 덮어쓰지 않고
        중단합니다.
      </p>
    </section>
  );
}
