import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { I18nProvider } from "./i18n";
import { ManuscriptInput } from "./ManuscriptInput";
import { api } from "./shared/ipc";
import type { MaterialTask, Project } from "./shared/contracts";

vi.mock("./shared/ipc", () => ({ api: { choose: vi.fn(), workspace: vi.fn(), addMaterial: vi.fn(), useWorkspaceMaterial: vi.fn() } }));
const project = { id: "synthetic", revision: 4 } as Project;
const item = { id: "manuscript", manuscriptKind: "anonymized_manuscript" } as MaterialTask;
beforeEach(() => vi.resetAllMocks());

for (const locale of ["zh-CN", "en"] as const) {
  const en = locale === "en";
  const choose = en ? "Choose DOCX" : "选择 DOCX";
  const search = en ? "Find DOCX in folder" : "检索目录内 DOCX";
  const use = en ? "Use as manuscript" : "用作主稿";
  const setup = (task = item) => {
    localStorage.setItem("manuscriptdock.locale", locale);
    const onProvided = vi.fn();
    render(<I18nProvider><ManuscriptInput project={project} item={task} disabled={false} onProvided={onProvided}/></I18nProvider>);
    return { user: userEvent.setup(), onProvided };
  };
  it(`finds nested and uppercase DOCX, then explicitly adopts the author-confirmed choice in ${locale}`, async () => {
    vi.mocked(api.workspace).mockResolvedValue({ rootPath: "/synthetic", entries: [
      { relativePath: "submission/manuscript.docx", directory: false, sizeBytes: 42 },
      { relativePath: "submission/nested/revised.DOCX", directory: false, sizeBytes: 42 },
      { relativePath: "submission/figures.docx", directory: true, sizeBytes: 0 },
      { relativePath: "submission/source.pdf", directory: false, sizeBytes: 42 },
    ] });
    const updated = { ...project, revision: 5 };
    vi.mocked(api.useWorkspaceMaterial).mockResolvedValue(updated);
    const { user, onProvided } = setup();
    await user.click(screen.getByRole("button", { name: search }));
    const select = await screen.findByRole("combobox");
    expect(screen.getAllByRole("option")).toHaveLength(3);
    expect(api.useWorkspaceMaterial).not.toHaveBeenCalled();
    await user.selectOptions(select, "submission/manuscript.docx");
    expect(screen.getByRole("button", { name: use })).toBeDisabled();
    await user.click(screen.getByRole("checkbox"));
    await user.selectOptions(select, "submission/nested/revised.DOCX");
    expect(screen.getByRole("checkbox")).not.toBeChecked();
    await user.click(screen.getByRole("checkbox", { name: en ? /removed author identity/ : /已移除作者身份/ }));
    await user.click(screen.getByRole("button", { name: use }));
    expect(api.useWorkspaceMaterial).toHaveBeenCalledWith(project, "submission/nested/revised.DOCX", "anonymized_manuscript");
    expect(onProvided).toHaveBeenCalledWith(updated);
  });
  it(`handles cancellation, an empty folder, and a retryable import failure in ${locale}`, async () => {
    vi.mocked(api.choose).mockResolvedValueOnce({ status: "cancelled" }).mockResolvedValue({ status: "selected", items: [{ token: "opaque-token", name: "author.docx", extension: "docx", kind: "material", sizeBytes: 42 }] });
    vi.mocked(api.workspace).mockResolvedValue({ rootPath: "/synthetic", entries: [] });
    vi.mocked(api.addMaterial).mockRejectedValueOnce({ code: "WORKSPACE_IO_FAILED" }).mockResolvedValue({ ...project, revision: 5 });
    const { user, onProvided } = setup({ ...item, manuscriptKind: "editable_manuscript", providedFileName: "original.docx" });
    expect(screen.getByText("original.docx")).toBeVisible();
    await user.click(screen.getByRole("button", { name: choose }));
    expect(screen.queryByRole("checkbox")).toBeNull();
    await user.click(screen.getByRole("button", { name: search }));
    expect(await screen.findByRole("status")).toHaveTextContent(en ? "No DOCX found" : "未找到 DOCX");
    await user.click(screen.getByRole("button", { name: choose }));
    expect(api.choose).toHaveBeenCalledWith("editable_manuscript");
    await user.click(await screen.findByRole("checkbox", { name: en ? /editable manuscript/ : /可编辑主稿/ }));
    await user.click(screen.getByRole("button", { name: use }));
    expect(await screen.findByRole("alert")).not.toHaveTextContent("WORKSPACE_IO_FAILED");
    expect(onProvided).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: use }));
    expect(api.addMaterial).toHaveBeenCalledWith(project, "opaque-token", "editable_manuscript");
    expect(onProvided).toHaveBeenCalledWith({ ...project, revision: 5 });
  });
}
