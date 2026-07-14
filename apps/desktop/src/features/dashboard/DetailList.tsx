interface DetailListProps {
  items: string[];
  empty: string;
}

export function DetailList({ items, empty }: DetailListProps) {
  if (items.length === 0) return <p className="muted compact">{empty}</p>;
  return (
    <ul className="detail-list">
      {items.map((item, index) => (
        <li key={`${item}-${index}`}>{item}</li>
      ))}
    </ul>
  );
}
