import type { DocumentFacts, Project } from "./shared/contracts";

export const factLabels = {
  title: ["论文标题", "Manuscript title"],
  abstractText: ["摘要", "Abstract"],
  keywords: ["关键词", "Keywords"],
  language: ["语言", "Language"],
  articleType: ["文章类型", "Article type"],
  authors: ["作者", "Authors"],
  affiliations: ["作者单位", "Affiliations"],
  correspondingEmail: ["通讯邮箱", "Corresponding email"],
  conflictOfInterest: ["利益冲突声明", "Competing-interest statement"],
  funding: ["经费声明", "Funding statement"],
  dataAvailability: ["数据可用性声明", "Data-availability statement"],
  ethicsStatement: ["伦理声明", "Ethics statement"],
  highlights: ["研究亮点", "Highlights"],
  creditContributions: ["作者贡献", "Author contributions"],
  generativeAiDisclosure: ["生成式 AI 使用声明", "Generative-AI disclosure"],
} as const;
export type FactField = keyof typeof factLabels;
export type FactConflict = { field: FactField; local: DocumentFacts[FactField]; saved: DocumentFacts[FactField] };
const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

// Preserve local edits while incorporating unrelated changes. Overlapping edits
// and edits from a replaced source require an explicit choice before saving.
export function reconcileFactDraft(base: Project, draft: DocumentFacts, latest: Project) {
  const facts = { ...latest.facts };
  const conflicts: FactConflict[] = [];
  for (const field of Object.keys(factLabels) as FactField[]) {
    if (same(draft[field], base.facts[field])) continue;
    if (base.activeSource.sha256 !== latest.activeSource.sha256 ||
      (!same(latest.facts[field], base.facts[field]) && !same(latest.facts[field], draft[field]))) {
      conflicts.push({ field, local: draft[field], saved: latest.facts[field] });
    } else Object.assign(facts, { [field]: draft[field] });
  }
  return { facts, conflicts };
}
