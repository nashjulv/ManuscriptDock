import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ExistingWorkspaceMatches } from "./App";
import { I18nProvider } from "./i18n";

const matches = Array.from({ length: 5 }, (_, index) => ({ exactContent: index % 2 === 0, workspace: { id: `synthetic-${index}`, manuscript: { name: `Synthetic ${index + 1}.docx`, extension: "docx", kind: "word" as const, sizeBytes: 10, modifiedUnixMs: null }, contentHash: "a".repeat(64), importedUnixMs: Date.UTC(2026, 8, 8), snapshotVersion: index + 1 } }));

describe("existing workspace matches", () => {
  beforeEach(() => window.localStorage.clear());
  for (const locale of ["zh-CN", "en"] as const) {
    const en = locale === "en";
    it(`${locale}: limits to three, expands by keyboard, opens a later match, and collapses`, async () => {
      window.localStorage.setItem("manuscriptdock.locale", locale);
      const user = userEvent.setup(); const open = vi.fn();
      render(<I18nProvider><ExistingWorkspaceMatches matches={matches} onOpen={open} /></I18nProvider>);
      const region = screen.getByRole("region", { name: en ? "Existing local workspaces found" : "发现已有本地档案" });
      const buttons = () => within(region).getAllByRole("button", { name: en ? "Continue existing workspace" : "继续已有档案" });
      expect(buttons()).toHaveLength(3);
      expect(screen.queryByText("Synthetic 4.docx · v4")).not.toBeInTheDocument();
      const more = screen.getByRole("button", { name: en ? "Show more (2 remaining)" : "展开更多（另 2 个）" });
      expect(more).toHaveAttribute("aria-expanded", "false");
      more.focus(); await user.keyboard("{Enter}");
      expect(buttons()).toHaveLength(5);
      expect(more).toHaveAttribute("aria-expanded", "true");
      expect(more).toHaveAccessibleName(en ? "Show fewer" : "收起");
      await user.click(buttons()[4]); expect(open).toHaveBeenCalledWith(matches[4].workspace);
      await user.click(more); expect(buttons()).toHaveLength(3);
      expect(more).toHaveAttribute("aria-expanded", "false");
    });
    it(`${locale}: hides controls for zero to three matches and resets for another selection`, async () => {
      window.localStorage.setItem("manuscriptdock.locale", locale);
      const { rerender } = render(<I18nProvider><ExistingWorkspaceMatches key="one" matches={[]} onOpen={vi.fn()} /></I18nProvider>);
      expect(screen.queryByRole("region")).not.toBeInTheDocument();
      for (const count of [1, 2, 3]) {
        rerender(<I18nProvider><ExistingWorkspaceMatches key="one" matches={matches.slice(0, count)} onOpen={vi.fn()} /></I18nProvider>);
        expect(screen.getAllByRole("button")).toHaveLength(count);
        expect(screen.queryByRole("button", { name: /展开更多|Show more/ })).not.toBeInTheDocument();
      }
      rerender(<I18nProvider><ExistingWorkspaceMatches key="one" matches={matches} onOpen={vi.fn()} /></I18nProvider>);
      await userEvent.click(screen.getByRole("button", { name: /展开更多|Show more/ }));
      rerender(<I18nProvider><ExistingWorkspaceMatches key="two" matches={matches.slice(0, 4)} onOpen={vi.fn()} /></I18nProvider>);
      expect(screen.queryByText("Synthetic 4.docx · v4")).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: /展开更多|Show more/ })).toHaveAttribute("aria-expanded", "false");
    });
  }
});
