import { invoke } from "@tauri-apps/api/core";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { I18nProvider, localizeBackendText } from "./i18n";
import { PackageExportSummary, type PackageExportStatus } from "./PackageExportStatus";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: () => true }));
const invokeMock = vi.mocked(invoke);
const receipt = { exportId: "synthetic-export", contextHash: "synthetic-context", packageLocation: "/synthetic/export", packageName: "Synthetic-study-submission-v1", manuscriptVersion: 1, targetSelectionId: "target", targetName: "Synthetic Journal", files: ["submission/manuscript.docx"], warnings: [], exportedUnixMs: Date.UTC(2026, 8, 8), externalTransmission: "not_performed" as const };

describe("durable package export status", () => {
  beforeEach(() => { invokeMock.mockReset(); window.localStorage.clear(); });
  for (const locale of ["zh-CN", "en"] as const) {
    const en = locale === "en";
    it(`${locale}: retains export history while explaining changed contents and unavailable files`, async () => {
      window.localStorage.setItem("manuscriptdock.locale", locale);
      const status: PackageExportStatus = { state: "changed", lastExport: receipt, locationAvailable: false, recordWarning: null };
      render(<I18nProvider><PackageExportSummary status={status} /></I18nProvider>);
      expect(screen.getByRole("status")).toHaveTextContent(en ? "Preparation changed" : "内容已变更");
      expect(screen.getByText(en ? /Check and export again/ : /请重新检查并导出/)).toBeVisible();
      await userEvent.click(screen.getByText(en ? "View last successful export" : "查看上次成功导出记录"));
      expect(screen.getByText(receipt.packageName)).toBeVisible();
      expect(screen.getByText(/\/synthetic\/export/)).toBeVisible();
      expect(screen.getByRole("alert")).toHaveTextContent(en ? "may have been moved or deleted" : "可能已移动或删除");
      expect(screen.getByText(en ? /Recording the receipt after submission is a separate step/ : /回执登记是独立步骤/)).toBeVisible();
    });
    it(`${locale}: explains save failures and unverified legacy records without claiming success`, () => {
      window.localStorage.setItem("manuscriptdock.locale", locale);
      const { rerender } = render(<I18nProvider><PackageExportSummary status={{ state: "legacy", lastExport: null, locationAvailable: null, recordWarning: null }} /></I18nProvider>);
      expect(screen.getByRole("status")).toHaveTextContent(en ? "Previous export needs review" : "历史导出待核验");
      rerender(<I18nProvider><PackageExportSummary status={{ state: "unverified", lastExport: null, locationAvailable: null, recordWarning: "EXPORT_RECORD_UNVERIFIED" }} /></I18nProvider>);
      expect(screen.getByRole("alert")).toHaveTextContent(en ? "could not be verified" : "无法核验");
      expect(localizeBackendText(locale, "本地工作区记录无效：EXPORT_RECORD_SAVE_FAILED")).toContain(en ? "Files were generated" : "文件已生成");
      expect(localizeBackendText(locale, "EXPORT_AUDIT_WRITE_FAILED")).toContain(en ? "audit log" : "审计日志");
      const notes = ["参考文献文件：可上传 BIB、RIS、NBIB、EndNote、XML 或其他可编辑参考文献文件", "声明文件：可上传伦理、知情同意、利益冲突、资金、数据可用性、作者贡献或 AI 使用声明", "投稿信：可上传致编辑的投稿信；是否需要以及内容格式以当前期刊要求为准", "标题页与作者信息页：适用于期刊要求将作者、单位、通讯信息与匿名主稿分离的情况", "补充材料与研究数据：可上传附录、方法补充、数据、代码归档、演示或音视频等期刊允许的补充材料", "说明、回复与其他支持文件：可上传情况说明、回复信、报告清单、版权或许可文件、作者协议及其他支持资料"];
      for (const note of notes) {
        const translated = localizeBackendText(locale, note);
        if (en) { expect(translated).not.toMatch(/[\u4e00-\u9fff]/); expect(translated).not.toContain("operation could not be completed"); }
        else expect(translated).toBe(note);
      }
      expect(localizeBackendText(locale, "已自动排除 2 个属于旧版本、旧目标或旧要求快照的附件")).toContain(en ? "2 attachments" : "2 个");
    });
    for (const state of ["pending", "ready", "exported", "changed", "legacy"] as const) {
      it(`${locale}: restores ${state} consistently in navigation and overview without a submission receipt`, async () => {
        window.localStorage.setItem("manuscriptdock.locale", locale);
        const workspace = { id: "synthetic", manuscript: { name: "Synthetic-study.docx", extension: "docx", kind: "word", sizeBytes: 2048, modifiedUnixMs: null }, contentHash: "a".repeat(64), importedUnixMs: Date.UTC(2026, 8, 8), snapshotVersion: 1 };
        const exportStatus = { state, lastExport: ["exported", "changed"].includes(state) ? receipt : null, locationAvailable: true, recordWarning: null };
        invokeMock.mockImplementation(async command => {
          if (command === "list_workspaces") return { workspaces: [workspace], warnings: [] };
          if (command === "list_linked_sources" || command === "get_journal_requirement_snapshots") return [];
          if (command === "get_workspace_lifecycle") return { workspaceId: workspace.id, currentVersion: 1, structureReport: null, readinessReport: null, attestation: null, submission: null, knowledgeBody: null, submissionTarget: null, journalRequirements: null, submissionMaterials: { schemaVersion: 6, materials: [], checklist: [], targetCheckReady: state === "exported" || state === "ready", exportStatus } };
          return null;
        });
        render(<App />);
        await userEvent.click(await screen.findByRole("button", { name: `${en ? "Open" : "打开"} Synthetic-study.docx` }));
        const label = { pending: en ? "Pending" : "待处理", ready: en ? "Ready to export" : "可导出", exported: en ? "Exported" : "已导出", changed: en ? "Preparation changed" : "内容已变更", legacy: en ? "Previous export needs review" : "历史导出待核验" }[state];
        const nav = within(screen.getByRole("navigation", { name: en ? "Primary submission tasks" : "投稿准备主任务" })).getByRole("button", { name: en ? /Package/ : /投稿包/ });
        expect(nav).toHaveTextContent(label);
        expect(nav).toHaveAttribute("data-complete", String(state === "exported"));
        expect(within(screen.getByRole("region", { name: en ? "Package export record" : "投稿包导出记录" })).getByRole("status")).toHaveTextContent(label);
      });
    }
  }
});
