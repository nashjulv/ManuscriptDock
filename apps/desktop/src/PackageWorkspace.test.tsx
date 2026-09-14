import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { I18nProvider, localizeBackendText } from "./i18n";
import { PackageWorkspace } from "./PackageWorkspace";
import { JournalTargetMap } from "./JournalTargetMap";
import { api } from "./shared/ipc";
import type { Project, Recommendation } from "./shared/contracts";

vi.mock("./shared/ipc", () => ({ api: { workspace: vi.fn(), moveWorkspace: vi.fn(), importWorkspace: vi.fn(), openWorkspace: vi.fn(), useWorkspaceMaterial: vi.fn(), choose: vi.fn() } }));
const handlers = vi.hoisted(() => new Map<string, (event: any) => void>());
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async (name: string, callback: (event: any) => void) => { handlers.set(name, callback); return () => handlers.delete(name); }) }));
const project = { id: "project-1", revision: 2 } as Project;
const listing = { rootPath: "/local/package", entries: [
  { relativePath: "submission", directory: true, sizeBytes: 0 },
  { relativePath: "figure.png", directory: false, sizeBytes: 2048 },
] };
beforeEach(() => { vi.clearAllMocks(); handlers.clear(); vi.mocked(api.workspace).mockResolvedValue(structuredClone(listing)); });

it("moves through pointer events while native file drops remain enabled", async () => {
  localStorage.setItem("manuscriptdock.locale", "en");
  // jsdom does not provide PointerEvent or pointer capture.
  vi.stubGlobal("PointerEvent", MouseEvent);
  const capture = vi.fn();
  Object.defineProperty(HTMLElement.prototype, "setPointerCapture", { configurable: true, value: capture });
  render(<I18nProvider><PackageWorkspace project={project} setProject={vi.fn()} run={async action => action()} refreshKey="1" /></I18nProvider>);
  const file = await screen.findByRole("button", { name: /figure.png/ });
  Object.defineProperty(document, "elementFromPoint", { configurable: true, value: () => document.querySelector('[data-directory="submission"]') });
  fireEvent.pointerDown(file, { button: 0, clientX: 10, clientY: 10 });
  fireEvent.pointerMove(file, { clientX: 80, clientY: 80 });
  fireEvent.pointerUp(file, { clientX: 80, clientY: 80 });
  await waitFor(() => expect(api.moveWorkspace).toHaveBeenCalledWith("project-1", "figure.png", "submission"));
  expect(file).toHaveAttribute("draggable", "false");
  vi.unstubAllGlobals();
});

for (const locale of ["zh-CN", "en"] as const) {
  it(`browses, changes view, moves, refreshes and snapshots an externally edited file in ${locale}`, async () => {
    localStorage.setItem("manuscriptdock.locale", locale);
    const user = userEvent.setup();
    const setProject = vi.fn();
    const run = async <T,>(action: () => Promise<T>) => action();
    render(<I18nProvider><PackageWorkspace project={project} setProject={setProject} run={run} refreshKey="1" /></I18nProvider>);
    expect(await screen.findByText("/local/package")).toBeVisible();
    await user.click(screen.getByRole("button", { name: locale === "en" ? "List" : "列表" }));
    expect(screen.getByRole("button", { name: /figure.png/ }).parentElement).toHaveClass("list");
    await user.click(screen.getByRole("button", { name: /figure.png/ }));
    vi.mocked(api.useWorkspaceMaterial).mockResolvedValue({ ...project, revision: 3 });
    await user.click(screen.getByRole("button", { name: locale === "en" ? "Include using selected attachment purpose" : "按所选附件用途加入投稿包" }));
    expect(api.useWorkspaceMaterial).toHaveBeenCalledWith(project, "figure.png", "supplementary");
    expect(setProject).toHaveBeenCalledWith({ ...project, revision: 3 });
    const folder = document.querySelector('[data-directory="submission"]')!;
    fireEvent.drop(folder, { dataTransfer: { getData: () => "figure.png" } });
    await waitFor(() => expect(api.moveWorkspace).toHaveBeenCalledWith("project-1", "figure.png", "submission"));
    vi.mocked(api.workspace).mockResolvedValue({ ...listing, entries: [...listing.entries, { relativePath: "external-edit.txt", directory: false, sizeBytes: 22 }] });
    fireEvent(window, new Event("focus"));
    expect(await screen.findByRole("button", { name: /external-edit.txt/ })).toBeVisible();
    await user.click(screen.getByRole("button", { name: locale === "en" ? "Open in file manager" : "在文件管理器中打开" }));
    expect(api.openWorkspace).toHaveBeenCalledWith("project-1");
    vi.mocked(api.workspace).mockRejectedValue(new Error("offline folder"));
    await user.click(screen.getByRole("button", { name: locale === "en" ? "Refresh" : "刷新" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(locale === "en" ? "Could not read" : "无法读取或整理投稿目录");
    for (const code of ["WORKSPACE_PATH_INVALID", "WORKSPACE_FILE_EXISTS", "WORKSPACE_IO_FAILED", "SYSTEM_OPEN_FAILED", "SOURCE_URL_INVALID"]) {
      expect(localizeBackendText(locale, code)).not.toContain(code);
    }
  });

  it(`links concentric target points to journal selection in ${locale}`, async () => {
    localStorage.setItem("manuscriptdock.locale", locale);
    const select = vi.fn();
    const items = [{ journal: { id: "eswa", displayName: "Expert Systems with Applications" }, role: "best_overall_fit" }] as Recommendation[];
    render(<I18nProvider><JournalTargetMap items={items} selected={null} onSelect={select} /></I18nProvider>);
    expect(document.querySelectorAll(".journal-target-plot circle")).toHaveLength(3);
    await userEvent.click(screen.getAllByRole("button", { name: /Expert Systems with Applications/ })[0]);
    expect(select).toHaveBeenCalledWith("eswa");
  });
}

it("accepts native drops only inside the package browser and refreshes partial imports", async () => {
  localStorage.setItem("manuscriptdock.locale", "en");
  const run = async <T,>(action: () => Promise<T>) => action();
  render(<I18nProvider><PackageWorkspace project={project} setProject={vi.fn()} run={run} refreshKey="1" /></I18nProvider>);
  await screen.findByText("/local/package");
  await waitFor(() => expect(handlers.has("manuscriptdock://files-dropped")).toBe(true));
  const folder = document.querySelector('[data-directory="submission"]');
  Object.defineProperty(document, "elementFromPoint", { configurable: true, value: () => folder });
  handlers.get("manuscriptdock://files-dropped")!({ payload: { items: [{ token: "native-token" }], x: 20, y: 20 } });
  await waitFor(() => expect(api.importWorkspace).toHaveBeenCalledWith("project-1", "native-token", "submission"));
  expect(api.workspace).toHaveBeenCalledTimes(2);
});
