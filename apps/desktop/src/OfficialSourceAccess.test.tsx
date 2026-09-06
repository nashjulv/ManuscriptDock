import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { OfficialSourceAccess, visibleAccessEvents, type OfficialFetchResult } from "./OfficialSourceAccess";
import { I18nProvider, localizeBackendText, OFFICIAL_SOURCE_MESSAGES } from "./i18n";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
beforeEach(() => { window.localStorage.clear(); invokeMock.mockReset(); invokeMock.mockResolvedValue(null); });

it.each(["zh-CN", "en"] as const)("ignores legacy HTTP gates and only confirms additional domains in %s", async (locale) => {
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
  await waitFor(() => expect(onDiscover).toHaveBeenCalledWith({ approvedOrigins: [] }));
  expect(await screen.findByText(localizeBackendText(locale, "OFFICIAL_TLS_FAILED"))).toBeInTheDocument();
  expect(button).toBeDisabled();
  const boxes = screen.getAllByRole("checkbox");
  expect(boxes).toHaveLength(2);
  for (const box of boxes) expect(box).not.toBeChecked();
  await user.click(boxes[0]); await user.click(boxes[1]);
  expect(button).toBeEnabled();
  await user.click(button);
  await waitFor(() => expect(onDiscover).toHaveBeenLastCalledWith({ approvedOrigins: ["https://authors.publisher.example"] }));
  for (const box of screen.getAllByRole("checkbox")) expect(box).not.toBeChecked();
  expect(button).toBeDisabled();
});

it.each(["zh-CN", "en"] as const)("records cancellation without fetching and localizes failures in %s", async (locale) => {
  window.localStorage.setItem("manuscriptdock.locale", locale);
  const user = userEvent.setup();
  invokeMock.mockImplementation(async (command) => command === "get_journal_source_access" ? { runId: "saved", partial: true, snapshot: null, options: { approvedOrigins: [], httpOrigins: [] }, pending: [{ origin: "http://authors.publisher.example", kind: "origin" }], events: [{ requestedUrl: "https://journal.example", url: "https://journal.example", code: "OFFICIAL_HTTP_STATUS", detail: "403" }] } : null);
  const onDiscover = vi.fn();
  render(<I18nProvider><OfficialSourceAccess workspaceId="workspace" selectionId="selection" homepageUrl="https://journal.example" busy={false} onDiscover={onDiscover} /></I18nProvider>);
  await user.click(await screen.findByRole("button", { name: locale === "en" ? "Cancel additional access and paste official text" : "取消额外访问，改用粘贴原文" }));
  expect(invokeMock).toHaveBeenCalledWith("cancel_journal_source_access", { workspaceId: "workspace", targetSelectionId: "selection" });
  expect(onDiscover).not.toHaveBeenCalled();
  expect(await screen.findByText(locale === "en" ? "Additional access cancelled. Paste official text below." : "已取消额外访问，请在下方粘贴官方原文。")).toBeVisible();
});

it.each(["zh-CN", "en"] as const)("captures HTTP in one authorized action and hides recovery details in %s", async (locale) => {
  window.localStorage.setItem("manuscriptdock.locale", locale);
  const user = userEvent.setup();
  const result: OfficialFetchResult = { runId: "success", snapshot: {}, partial: false, options: { approvedOrigins: [] }, pending: [], events: ["OFFICIAL_VIRTUAL_DNS", "OFFICIAL_DNS_RECOVERED", "OFFICIAL_TLS_FAILED", "OFFICIAL_HTTP_USED", "OFFICIAL_RECEIVED", "OFFICIAL_ENCODING_INFERRED", "OFFICIAL_CAPTURED"].map((code) => ({ requestedUrl: "http://journal.example", url: "http://journal.example", code, detail: code === "OFFICIAL_RECEIVED" ? "200" : null })) };
  const onDiscover = vi.fn().mockResolvedValue(result);
  render(<I18nProvider><OfficialSourceAccess workspaceId="workspace" selectionId="selection" homepageUrl="http://journal.example" busy={false} onDiscover={onDiscover} /></I18nProvider>);
  expect(screen.getByText(locale === "en" ? "About this access" : "读取说明").closest("details")).not.toHaveAttribute("open");
  expect(screen.getByText(/Cloudflare/)).not.toBeVisible();
  expect(screen.queryByText(/HTTPS is preferred|优先使用 HTTPS/)).not.toBeInTheDocument();
  await user.click(screen.getByRole("checkbox"));
  await user.click(screen.getByRole("button"));
  await waitFor(() => expect(onDiscover).toHaveBeenCalledTimes(1));
  expect(onDiscover).toHaveBeenCalledWith({ approvedOrigins: [] });
  expect(screen.getAllByRole("checkbox")).toHaveLength(1);
  expect(screen.queryByRole("button", { name: /Cancel additional|取消额外/ })).not.toBeInTheDocument();
  await user.click(screen.getByText(locale === "en" ? "View accessed URLs and results" : "查看访问地址与结果"));
  expect(screen.queryByText(localizeBackendText(locale, "OFFICIAL_DNS_RECOVERED"))).not.toBeInTheDocument();
  expect(screen.queryByText(localizeBackendText(locale, "OFFICIAL_HTTP_USED"))).not.toBeInTheDocument();
  expect(screen.queryByText(localizeBackendText(locale, "OFFICIAL_TLS_FAILED"))).not.toBeInTheDocument();
  expect(screen.getByText(`${localizeBackendText(locale, "OFFICIAL_RECEIVED")} (200)`)).toBeVisible();
  expect(screen.getByText(localizeBackendText(locale, "OFFICIAL_ENCODING_INFERRED"))).toBeVisible();
  expect(screen.getByText(localizeBackendText(locale, "OFFICIAL_CAPTURED"))).toBeVisible();
});

it("keeps final failures on other pages while suppressing recovered attempts", () => {
  const events = [
    { requestedUrl: "http://one.example/", url: "https://one.example/", code: "OFFICIAL_TLS_FAILED", detail: null },
    { requestedUrl: "http://one.example/", url: "http://one.example/", code: "OFFICIAL_RECEIVED", detail: "200" },
    { requestedUrl: "https://two.example/guide", url: "https://two.example/guide", code: "OFFICIAL_HTTP_STATUS", detail: "403" },
  ];
  expect(visibleAccessEvents(events)).toEqual(events.slice(1));
  expect(events).toHaveLength(3);
  expect(visibleAccessEvents([events[0], { ...events[0], code: "OFFICIAL_CONNECTION_FAILED" }]).map((event) => event.code)).toEqual(["OFFICIAL_CONNECTION_FAILED"]);
});

it.each(["zh-CN", "en"] as const)("shows an actionable final network failure without DNS internals in %s", async (locale) => {
  window.localStorage.setItem("manuscriptdock.locale", locale);
  invokeMock.mockImplementation(async (command) => command === "get_journal_source_access" ? { runId: "failure", snapshot: null, partial: true, options: { approvedOrigins: [] }, pending: [], events: [{ requestedUrl: "https://journal.example/", url: "https://journal.example/", code: "OFFICIAL_ENCRYPTED_DNS_FAILED", detail: null }] } : null);
  render(<I18nProvider><OfficialSourceAccess workspaceId="workspace" selectionId="selection" homepageUrl="https://journal.example/" busy={false} onDiscover={vi.fn()} /></I18nProvider>);
  await userEvent.setup().click(await screen.findByText(locale === "en" ? "View accessed URLs and results" : "查看访问地址与结果"));
  expect(screen.getByText(localizeBackendText(locale, "OFFICIAL_SOURCE_UNAVAILABLE"))).toBeVisible();
  expect(screen.queryByText(localizeBackendText(locale, "OFFICIAL_ENCRYPTED_DNS_FAILED"))).not.toBeInTheDocument();
});

it.each(["zh-CN", "en"] as const)("shows the corrected source before reading an existing target in %s", async (locale) => {
  window.localStorage.setItem("manuscriptdock.locale", locale);
  invokeMock.mockImplementation(async (command) => command === "get_journal_homepage_correction" ? { url: "http://cjc.ict.ac.cn/", authorityUrl: "https://www.ict.cas.cn/xscbw/jsjxb/", verifiedOn: "2026-09-06" } : null);
  render(<I18nProvider><OfficialSourceAccess workspaceId="workspace" selectionId="old-primary" homepageUrl="https://cjc.ict.ac.cn/" busy={false} onDiscover={vi.fn()} /></I18nProvider>);
  expect(await screen.findByText("http://cjc.ict.ac.cn/")).not.toBeVisible();
  await userEvent.setup().click(screen.getByText(locale === "en" ? "About this access" : "读取说明"));
  expect(await screen.findByText("http://cjc.ict.ac.cn/")).toBeVisible();
  expect(screen.getByText("https://www.ict.cas.cn/xscbw/jsjxb/")).toBeVisible();
  expect(screen.getAllByRole("checkbox")).toHaveLength(1);
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
