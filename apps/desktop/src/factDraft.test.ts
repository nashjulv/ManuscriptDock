import { expect, it } from "vitest";
import { reconcileFactDraft } from "./factDraft";
import type { Project } from "./shared/contracts";

const base = { id: "synthetic", activeSource: { sha256: "source-a" }, facts: { sourceHash: "source-a", title: "Original", authors: ["Author A"], funding: null, authorConfirmedFields: [] } } as unknown as Project;

it("merges unrelated changes and preserves array edits without mutating inputs", () => {
  const draft = { ...base.facts, authors: ["Author A", "Author B"] };
  const latest = { ...base, facts: { ...base.facts, funding: "Saved funding" } };
  const merged = reconcileFactDraft(base, draft, latest);
  expect(merged.conflicts).toEqual([]);
  expect(merged.facts.authors).toEqual(["Author A", "Author B"]);
  expect(merged.facts.funding).toBe("Saved funding");
  expect(latest.facts.authors).toEqual(["Author A"]);
});

it("retains both overlapping edits and binds recovered inputs to the latest source", () => {
  const draft = { ...base.facts, title: "Local title" };
  const latest = { ...base, activeSource: { ...base.activeSource, sha256: "source-b" }, facts: { ...base.facts, sourceHash: "source-b" } };
  const merged = reconcileFactDraft(base, draft, latest);
  expect(merged.conflicts).toEqual([{ field: "title", local: "Local title", saved: "Original" }]);
  expect(merged.facts.sourceHash).toBe("source-b");
  expect(merged.facts.title).toBe("Original");
});
