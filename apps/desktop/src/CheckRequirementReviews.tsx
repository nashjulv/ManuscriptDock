import type { SubmissionMaterialCatalog } from "./App";
import { RequirementDecision } from "./RequirementDecision";
import { useI18n } from "./i18n";
import { useEffect, useRef } from "react";
import { highlightGuidanceTarget } from "./ActionGuidance";

export function confirmationReviewed(catalog: SubmissionMaterialCatalog | null, ruleId: string) {
  return Boolean(catalog?.checklist.some(item => item.id === `check-${ruleId}` && (item.status === "passed" || item.status === "not_applicable")));
}

export function CheckRequirementReviews({ workspaceId, catalog, onUpdated, disabled = false, focusItemId }: {
  workspaceId: string;
  catalog: SubmissionMaterialCatalog | null;
  onUpdated: (catalog: SubmissionMaterialCatalog) => void;
  disabled?: boolean;
  focusItemId?: string | null;
}) {
  const { locale, text } = useI18n();
  const root = useRef<HTMLElement>(null);
  useEffect(() => {
    const row = Array.from(root.current?.querySelectorAll<HTMLDetailsElement>("details[data-review-id]") ?? []).find(row => row.dataset.reviewId === focusItemId);
    if (row) { row.open = true; row.scrollIntoView?.({ block: "center" }); row.focus(); return highlightGuidanceTarget(row); }
  }, [focusItemId]);
  const items = catalog?.checklist.filter(item => item.confirmable) ?? [];
  if (!items.length) return null;
  return <section ref={root} className="materials-checklist" aria-label={text("在此完成核验", "Complete reviews here")}>
    <h3>{text("在此完成核验", "Complete reviews here")}</h3>
    <p>{text("无需返回资料页。保存后自动更新本地检查；缺失文件仍需补齐。", "No need to return to Materials. Saving refreshes local checks automatically; missing files still need to be supplied.")}</p>
    {items.map(item => <details data-review-id={item.id} tabIndex={-1} key={item.id} open={item.status !== "passed" && item.status !== "not_applicable"}>
      <summary>{locale === "en" ? item.labelEn : item.label} · {item.status === "passed" ? text("已完成", "Complete") : item.status === "not_applicable" ? text("不适用", "Not applicable") : text("待核验", "Review needed")}</summary>
      {item.id.startsWith("check-") ? <p>{text("请对照原稿核实此项；如不适用，请说明依据。", "Review this item against the manuscript; explain your basis if it does not apply.")}</p> : item.evidenceExcerpt ? <blockquote>{item.evidenceExcerpt}</blockquote> : null}
      {item.sourceUrl ? <a href={item.sourceUrl} target="_blank" rel="noreferrer">{text("查看官方依据", "View official evidence")}</a> : null}
      <RequirementDecision key={`${item.id}-${item.decision}-${item.decisionReason}`} workspaceId={workspaceId} item={item} onUpdated={onUpdated} disabled={disabled} />
    </details>)}
  </section>;
}
