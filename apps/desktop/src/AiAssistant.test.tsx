import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { AiSettingsDialog, AiWorkbench, AiHistory } from "./AiAssistant";
import { I18nProvider, localizeBackendText } from "./i18n";
import { api } from "./shared/ipc";
import type { AiPreview, AiRun } from "./shared/contracts";
vi.mock("./shared/ipc", () => ({ api: { aiSettings: vi.fn(), saveAiSettings: vi.fn(), prepareAi: vi.fn(), runAi: vi.fn(), acceptAiDraft: vi.fn(), aiHistory: vi.fn() } }));
const preview: AiPreview = { id: "preview", input: { task: "draft_cover_letter", projectId: "p", materialId: "cover_letter", contextHash: "ctx", sources: [{ id: "facts", label: { zhCn: "资料", en: "Sources" }, text: "Synthetic study", sha256: "hash" }] }, provider: "https://example.com/v1", model: "synthetic-model", local: false, inputCharacters: 100, estimatedInputTokens: 90, maxOutputTokens: 2400 };
const result: AiRun = { id: "run", task: "draft_cover_letter", contextHash: "ctx", provider: preview.provider, model: preview.model, status: "succeeded", createdAt: 1, sources: [["facts", "hash"]], current: true, output: { paragraphs: [{ text: "Synthetic cover letter", evidence: [{ sourceId: "facts", quote: "Synthetic study" }] }], findings: [] } };
beforeEach(() => {
  vi.clearAllMocks();
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute("open"); };
  vi.mocked(api.prepareAi).mockResolvedValue(preview);
  vi.mocked(api.runAi).mockResolvedValue(result);
  vi.mocked(api.acceptAiDraft).mockResolvedValue("author-tools/cover-letter-DRAFT.docx");
  vi.mocked(api.aiSettings).mockResolvedValue({ enabled: false, endpoint: "", model: "", local: false, maxOutputTokens: 2400, hasKey: true });
});
for (const locale of ["zh-CN", "en"] as const) {
  const en = locale === "en";
  it(`previews, requires explicit consent, and saves only a review draft in ${locale}`, async () => {
    localStorage.setItem("manuscriptdock.locale", locale);
    const user = userEvent.setup();
    render(<I18nProvider><AiWorkbench projectId="p" task="draft_cover_letter" materialId="cover_letter" onClose={vi.fn()} onChanged={vi.fn()}/></I18nProvider>);
    const send = await screen.findByRole("button", { name: en ? "Send and run once" : "发送并执行一次" });
    expect(send).toBeDisabled(); expect(api.runAi).not.toHaveBeenCalled();
    await user.click(screen.getByText(en ? "Sources" : "资料"));
    expect(screen.getByText("Synthetic study")).toBeVisible();
    await user.click(screen.getByRole("checkbox")); await user.dblClick(send);
    expect(api.runAi).toHaveBeenCalledTimes(1);
    const save = await screen.findByRole("button", { name: en ? "Save as draft for review" : "保存为待核对草稿" });
    expect(api.acceptAiDraft).not.toHaveBeenCalled();
    await user.click(save);
    expect(await screen.findByText(en ? /Saved as a draft for review/ : /已保存为待核对草稿/)).toBeVisible();
    expect(save).toBeDisabled(); expect(api.acceptAiDraft).toHaveBeenCalledTimes(1);
  });
  it(`retains failure history and never retries in ${locale}`, async () => {
    localStorage.setItem("manuscriptdock.locale", locale);
    vi.mocked(api.runAi).mockResolvedValue({ ...result, output: null, status: "failed", errorCode: "AI_RATE_LIMITED" });
    const user = userEvent.setup();
    render(<I18nProvider><AiWorkbench projectId="p" task="review_consistency" onClose={vi.fn()} onChanged={vi.fn()}/></I18nProvider>);
    await user.click(await screen.findByRole("checkbox")); await user.click(screen.getByRole("button", { name: en ? "Send and run once" : "发送并执行一次" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(localizeBackendText(locale, "AI_RATE_LIMITED"));
    expect(api.runAi).toHaveBeenCalledTimes(1); expect(api.acceptAiDraft).not.toHaveBeenCalled();
  });
  it(`shows outdated evidence and prevents draft acceptance in ${locale}`, async () => {
    localStorage.setItem("manuscriptdock.locale", locale);
    render(<I18nProvider><AiWorkbench projectId="p" task="draft_cover_letter" existingRun={{ ...result, current: false }} onClose={vi.fn()} onChanged={vi.fn()}/></I18nProvider>);
    expect(screen.getByRole("button", { name: en ? "Save as draft for review" : "保存为待核对草稿" })).toBeDisabled();
    expect(api.prepareAi).not.toHaveBeenCalled(); expect(api.runAi).not.toHaveBeenCalled();
  });
  it(`does not treat a semantic review with no findings as confirmation in ${locale}`, async () => {
    localStorage.setItem("manuscriptdock.locale", locale);
    render(<I18nProvider><AiWorkbench projectId="p" task="review_material" existingRun={{ ...result, task: "review_material", output: { paragraphs: [], findings: [] } }} onClose={vi.fn()} onChanged={vi.fn()}/></I18nProvider>);
    expect(screen.getByText(en ? /This does not mean submission checks passed/ : /不代表已通过投稿检查/)).toBeVisible();
    expect(screen.queryByRole("button", { name: en ? "Save as draft for review" : "保存为待核对草稿" })).toBeNull();
  });
  it(`saves optional settings without invoking a model or exposing a stored key in ${locale}`, async () => {
    localStorage.setItem("manuscriptdock.locale", locale);
    const user = userEvent.setup(); const close = vi.fn();
    render(<I18nProvider><AiSettingsDialog onClose={close}/></I18nProvider>);
    expect(await screen.findByLabelText(en ? "API key" : "API 密钥")).toHaveValue("");
    await user.click(screen.getByRole("button", { name: en ? "Save settings" : "保存设置" }));
    expect(api.saveAiSettings).toHaveBeenCalledWith({ enabled: false, endpoint: "", model: "", local: false, maxOutputTokens: 2400, apiKey: "" });
    expect(api.runAi).not.toHaveBeenCalled(); expect(close).toHaveBeenCalled();
  });
  it(`localizes persisted history and dynamic errors in ${locale}`, async () => {
    localStorage.setItem("manuscriptdock.locale", locale);
    vi.mocked(api.aiHistory).mockResolvedValue([{ ...result, status: "started" }]);
    const user = userEvent.setup();
    render(<I18nProvider><AiHistory projectId="p" refreshKey={0} onSelect={vi.fn()}/></I18nProvider>);
    await user.click(screen.getByText(en ? "AI usage history and evidence" : "AI 使用记录与依据"));
    expect(await screen.findByRole("button", { name: en ? /Outcome unknown/ : /结果待确认/ })).toBeVisible();
    for (const code of ["AI_SOURCE_REQUIRED", "AI_INPUT_LIMIT", "AI_OUTPUT_INVALID", "AI_RUN_NOT_FOUND", "AI_RUN_ALREADY_STARTED", "AI_DRAFT_UNAVAILABLE", "AI_CONTEXT_CHANGED", "AI_BUSY", "AI_SETTINGS_INVALID", "AI_KEYCHAIN_UNAVAILABLE", "AI_ENDPOINT_INVALID", "AI_KEY_REQUIRED", "AI_NOT_CONFIGURED", "AI_NETWORK_FAILED", "AI_AUTH_FAILED", "AI_RATE_LIMITED", "AI_PROVIDER_FAILED", "AI_CONSENT_REQUIRED", "AI_PREVIEW_EXPIRED"]) {
      const translated = localizeBackendText(locale, code); expect(translated).not.toEqual(code); expect(/[\u4e00-\u9fff]/.test(translated)).toBe(!en);
    }
  });
}
