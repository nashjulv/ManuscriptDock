import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { OfficialSourceAccess, type OfficialFetchResult } from "./OfficialSourceAccess";
import { I18nProvider, localizeBackendText, OFFICIAL_SOURCE_MESSAGES } from "./i18n";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
beforeEach(() => { window.localStorage.clear(); invokeMock.mockReset(); invokeMock.mockResolvedValue(null); });

it.each(["zh-CN", "en"] as const)("requires fresh HTTP and exact-origin consent in %s", async (locale) => {
  window.localStorage.setItem("manuscriptdock.locale", locale);
  const user = userEvent.setup();
  const result: OfficialFetchResult = {
    runId: "fetch-synthetic", snapshot: null, partial: true, options: { approvedOrigins: [], httpOrigins: [] },
    pending: [{ kind: "origin", origin: "https://authors.publisher.example" }, { kind: "http", origin: "http://journal.example" }],
    events: [{ requestedUrl: "http://journal.example/guide", url: "https://journal.example/guide", code: "OFFICIAL_TLS_FAILED", detail: null }],
  };
  const onDiscover = vi.fn().mockResolvedValueOnce(result).mockResolvedValueOnce({ ...result, pending: [], options: { approvedOrigins: ["https://authors.publisher.example"], httpOrigins: ["http://journal.example"] } });
  render(<I18nProvider><OfficialSourceAccess workspaceId="workspace" selectionId="selection" homepageUrl="http://journal.example/guide" busy={false} onDiscover={onDiscover} /></I18nProvider>);
  const button = screen.getByRole("button", { name: locale === "en" ? "Capture official requirements" : "获取官方投稿要求" });
  expect(button).toBeDisabled();
  await user.click(screen.getByRole("checkbox"));
  await user.click(button);
  await waitFor(() => expect(onDiscover).toHaveBeenCalledWith({ approvedOrigins: [], httpOrigins: [] }));
  expect(await screen.findByText(localizeBackendText(locale, "OFFICIAL_TLS_FAILED"))).toBeInTheDocument();
  expect(button).toBeDisabled();
  const boxes = screen.getAllByRole("checkbox");
  expect(boxes).toHaveLength(3);
  for (const box of boxes) expect(box).not.toBeChecked();
  await user.click(boxes[0]); await user.click(boxes[1]);
  expect(button).toBeDisabled();
  await user.click(boxes[2]); await user.click(button);
  await waitFor(() => expect(onDiscover).toHaveBeenLastCalledWith({ approvedOrigins: ["https://authors.publisher.example"], httpOrigins: ["http://journal.example"] }));
  for (const box of screen.getAllByRole("checkbox")) expect(box).not.toBeChecked();
  expect(button).toBeDisabled();
});

it.each(["zh-CN", "en"] as const)("records cancellation without fetching and localizes failures in %s", async (locale) => {
  window.localStorage.setItem("manuscriptdock.locale", locale);
  const user = userEvent.setup();
  invokeMock.mockResolvedValueOnce({ runId: "saved", partial: true, snapshot: null, options: { approvedOrigins: [], httpOrigins: [] }, pending: [{ origin: "http://journal.example", kind: "http" }], events: [{ requestedUrl: "https://journal.example", url: "https://journal.example", code: "OFFICIAL_HTTP_STATUS", detail: "403" }] });
  const onDiscover = vi.fn();
  render(<I18nProvider><OfficialSourceAccess workspaceId="workspace" selectionId="selection" homepageUrl="https://journal.example" busy={false} onDiscover={onDiscover} /></I18nProvider>);
  await user.click(await screen.findByRole("button", { name: locale === "en" ? "Cancel additional access and paste official text" : "取消额外访问，改用粘贴原文" }));
  expect(invokeMock).toHaveBeenCalledWith("cancel_journal_source_access", { workspaceId: "workspace", targetSelectionId: "selection" });
  expect(onDiscover).not.toHaveBeenCalled();
  expect(await screen.findByText(locale === "en" ? "Additional access cancelled. Paste official text below." : "已取消额外访问，请在下方粘贴官方原文。")).toBeVisible();
});

it("does not apply a late result or reuse consent after switching targets", async () => {
  window.localStorage.setItem("manuscriptdock.locale", "en");
  const user = userEvent.setup();
  let complete!: (result: OfficialFetchResult) => void;
  const onDiscover = vi.fn(() => new Promise<OfficialFetchResult>((resolve) => { complete = resolve; }));
  const view = (selectionId: string) => <I18nProvider><OfficialSourceAccess workspaceId="workspace" selectionId={selectionId} homepageUrl="https://journal.example" busy={false} onDiscover={onDiscover} /></I18nProvider>;
  const { rerender } = render(view("one"));
  await user.click(screen.getByRole("checkbox")); await user.click(screen.getByRole("button"));
  rerender(view("two"));
  complete({ runId: "old", snapshot: null, partial: true, options: { approvedOrigins: [], httpOrigins: [] }, pending: [{ kind: "http", origin: "http://old.example" }], events: [] });
  await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("get_journal_source_access", { workspaceId: "workspace", targetSelectionId: "two" }));
  expect(screen.getByRole("checkbox")).not.toBeChecked();
  expect(screen.queryByText(/old.example/)).not.toBeInTheDocument();
});

it("distinguishes invalid URLs from HTTP and translates every access code in both locales", () => {
  window.localStorage.setItem("manuscriptdock.locale", "en");
  render(<I18nProvider><OfficialSourceAccess workspaceId="workspace" selectionId="selection" homepageUrl="https://" busy={false} onDiscover={vi.fn()} /></I18nProvider>);
  expect(screen.getByRole("alert")).toHaveTextContent("Enter a valid HTTP or HTTPS source URL.");
  for (const [code, [zh, en]] of Object.entries(OFFICIAL_SOURCE_MESSAGES)) {
    expect(localizeBackendText("zh-CN", code)).toBe(zh);
    expect(localizeBackendText("en", code)).toBe(en);
    expect(en).not.toMatch(/[\u4e00-\u9fff]/);
  }
});
