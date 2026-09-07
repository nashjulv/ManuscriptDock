import { localizeBackendText, useI18n, type Locale } from "./i18n";

export interface TargetSubmissionExport {
  exportId: string;
  contextHash: string;
  packageLocation: string;
  packageName: string;
  manuscriptVersion: number;
  targetSelectionId: string;
  targetName: string;
  files: string[];
  warnings: string[];
  exportedUnixMs: number;
  externalTransmission: "not_performed";
}

export interface PackageExportStatus {
  state: "pending" | "ready" | "exported" | "changed" | "legacy" | "unverified";
  lastExport: TargetSubmissionExport | null;
  locationAvailable: boolean | null;
  recordWarning: string | null;
}

export function packageExportLabel(status: PackageExportStatus | undefined, ready: boolean | undefined, locale: Locale) {
  const labels = {
    pending: ["待处理", "Pending"], ready: ["可导出", "Ready to export"],
    exported: ["已导出", "Exported"], changed: ["内容已变更", "Preparation changed"],
    legacy: ["历史导出待核验", "Previous export needs review"], unverified: ["导出记录待核验", "Export record needs review"],
  };
  return labels[status?.state ?? (ready ? "ready" : "pending")][locale === "zh-CN" ? 0 : 1];
}

export function PackageExportSummary({ status, ready }: { status?: PackageExportStatus; ready?: boolean }) {
  const { locale, text } = useI18n();
  const receipt = status?.lastExport;
  return <section className="package-export-summary" aria-label={text("投稿包导出记录", "Package export record")}>
    <strong role="status">{text("投稿包：", "Package: ")}{packageExportLabel(status, ready, locale)}</strong>
    {status?.state === "changed" && <p>{text("上次导出后，稿件、材料、目标要求或核验状态发生了变化。请重新检查并导出；历史记录仍保留。", "The manuscript, materials, target requirements, or verification changed after the last export. Check and export again; the previous record is retained.")}</p>}
    {status?.state === "legacy" && <p>{text("发现旧版导出记录，但无法确认它对应当前内容。请重新检查并导出以建立完整记录。", "An older export was recorded, but its contents cannot be matched to the current preparation. Check and export again to create a complete record.")}</p>}
    {receipt && <details><summary>{text("查看上次成功导出记录", "View last successful export")}</summary>
      <p>{receipt.packageName}</p>
      <p>{text(`稿件 v${receipt.manuscriptVersion} · ${receipt.files.length} 个文件`, `Manuscript v${receipt.manuscriptVersion} · ${receipt.files.length} files`)}</p>
      <p>{new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "medium" }).format(receipt.exportedUnixMs)}</p>
      <p>{text("导出位置：", "Export location: ")}{receipt.packageLocation}</p>
      {status?.locationAvailable === false && <p role="alert">{text("原导出文件夹目前不可访问，可能已移动或删除。历史导出记录仍然有效；需要文件时请重新导出。", "The original export folder is unavailable and may have been moved or deleted. The historical export record is retained; export again if you need the files.")}</p>}
      {receipt.warnings.map((warning, index) => <p key={index}>{localizeBackendText(locale, warning)}</p>)}
    </details>}
    {status?.recordWarning && <p role="alert">{localizeBackendText(locale, status.recordWarning)}</p>}
    <p>{text("导出完成后可自行上传到期刊网站。实际投稿后的回执登记是独立步骤，不影响导出完成状态。", "After exporting, upload the files to the journal website yourself. Recording the receipt after submission is a separate step and does not affect export completion.")}</p>
  </section>;
}
