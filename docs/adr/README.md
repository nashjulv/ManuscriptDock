# Architecture Decision Records

Architecture Decision Records capture durable technical choices and their consequences.

File naming:

```text
NNNN-short-decision-title.md
```

Each ADR contains status, context, decision, consequences, and links to superseding decisions. Accepted ADRs are not rewritten to hide history; material changes create a new ADR.

## Accepted decisions

- [0011 — Rebuild the two-task product and retire legacy flows](0011-rebuild-core-and-retire-legacy-flows.md)：允许重写旧代码／接口／交互，删除无用模块；数据保护不等于旧功能兼容；详细实现待开始。

- [0010 — Local journal matching and submission-package focus](0010-local-journal-package-focus.md)：2026-09-14 新定位；覆盖旧阶段导航和远程模型的产品范围，保留源稿、版本、证据与历史记录；待实施。

Earlier decisions remain historical records. Where scope conflicts with 0010, use 0010; do not infer that the new runtime behavior is implemented.

- [0001 — Desktop application architecture](0001-desktop-application-architecture.md)
- [0002 — Local workspace storage](0002-local-workspace-storage.md)
- [0003 — Deterministic structure extraction](0003-deterministic-structure-extraction.md)
- [0004 — Signed rule packs and readiness snapshots](0004-signed-rule-packs-and-readiness-snapshots.md)
- [0005 — Local manuscript version library](0005-local-manuscript-version-library.md)
- [0006 — Local lifecycle records and seven-step state machine](0006-local-lifecycle-records.md)
- [0007 — Author-controlled model routing and knowledge-body dialogue](0007-author-controlled-model-routing.md)
- [0008 — Workspace archive and permanent-delete boundary](0008-workspace-archive-and-delete.md)
- [0009 — Evidence-bound preparation and linked sources](0009-evidence-bound-preparation-and-linked-source.md)
