import { useI18n } from "./i18n";
import type { Recommendation } from "./shared/contracts";

export function JournalTargetMap({ items, selected, onSelect }: { items: Recommendation[]; selected: string | null; onSelect: (id: string) => void }) {
  const { text } = useI18n();
  const roles = { ambitious_option: text("冲刺方向", "Ambitious option"), best_overall_fit: text("综合匹配", "Overall fit"), less_preparation_needed: text("准备更省力", "Less preparation") };
  return <section className="journal-target-map" aria-label={text("目标期刊同心圆", "Journal target map")}>
    <div className="journal-target-plot">
      <svg viewBox="0 0 100 100" aria-hidden="true">
        <circle cx="50" cy="50" r="46" /><circle cx="50" cy="50" r="33" /><circle cx="50" cy="50" r="19" />
        <path d="M4 50H96M50 4V96" />
      </svg>
      <span className="target-center">{text("论文", "Paper")}</span>
      {items.map((item, index) => {
        const radius = item.role === "ambitious_option" ? 13 : item.role === "best_overall_fit" ? 28 : 42;
        const angle = (-90 + index * 360 / items.length) * Math.PI / 180;
        return <button key={item.journal.id} className="target-point" style={{ left: `${50 + Math.cos(angle) * radius}%`, top: `${50 + Math.sin(angle) * radius}%` }}
          aria-label={`${roles[item.role]} · ${item.journal.displayName}`} title={`${roles[item.role]} · ${item.journal.displayName}`}
          aria-pressed={selected === item.journal.id} onClick={() => onSelect(item.journal.id)}>{index + 1}</button>;
      })}
    </div>
    <div className="target-legend">
      <h3>{text("为这篇论文选择目标", "Choose a target for this paper")}</h3>
      <p>{text("点击圆点或期刊查看对应候选。圆环表示推荐角色，不代表录用概率。", "Select a point or journal to inspect a candidate. Rings indicate recommendation roles, not acceptance probabilities.")}</p>
      {items.map((item, index) => <button key={item.journal.id} aria-pressed={selected === item.journal.id} onClick={() => onSelect(item.journal.id)}><b>{index + 1} · {roles[item.role]}</b><span>{item.journal.displayName}</span></button>)}
    </div>
  </section>;
}
