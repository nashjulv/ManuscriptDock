import { invoke, isTauri } from "@tauri-apps/api/core";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { I18nProvider } from "./i18n";
import { TextSizeProvider, TextSizeSettings } from "./TextSizeSettings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: vi.fn() }));
const invokeMock = vi.mocked(invoke);
const KEY = "manuscriptdock.text-size.v1";

function Settings() {
  return <I18nProvider><TextSizeProvider><TextSizeSettings /><button type="button">Outside</button></TextSizeProvider></I18nProvider>;
}

beforeEach(() => {
  vi.restoreAllMocks();
  invokeMock.mockReset();
  vi.mocked(isTauri).mockReturnValue(false);
  window.localStorage.clear();
  document.documentElement.removeAttribute("data-text-size");
});

describe.each(["zh-CN", "en"] as const)("text size (%s)", (locale) => {
  const en = locale === "en";
  const title = en ? "Text size" : "字体大小";
  const small = en ? "Smaller" : "更小";
  const regular = en ? "Default" : "默认";
  const large = en ? "Larger" : "更大";
  beforeEach(() => window.localStorage.setItem("manuscriptdock.locale", locale));

  it("applies all sizes immediately, keeps the popover open, and restores the saved choice", async () => {
    const user = userEvent.setup();
    const { unmount } = render(<Settings />);
    expect(document.documentElement.dataset.textSize).toBe("default");
    await user.click(screen.getByRole("button", { name: title }));
    const dialog = within(screen.getByRole("dialog", { name: title }));
    expect(dialog.getByRole("button", { name: regular })).toHaveFocus();
    for (const [label, value] of [[small, "small"], [regular, "default"], [large, "large"]]) {
      await user.click(dialog.getByRole("button", { name: label }));
      expect(document.documentElement.dataset.textSize).toBe(value);
      expect(dialog.getByRole("button", { name: label })).toHaveAttribute("aria-pressed", "true");
      await waitFor(() => expect(window.localStorage.getItem(KEY)).toBe(value));
    }
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: title })).toHaveFocus();
    unmount();
    render(<Settings />);
    expect(document.documentElement.dataset.textSize).toBe("large");
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("closes via toggle, outside click, and focus leaving the non-modal popover", async () => {
    const user = userEvent.setup();
    render(<Settings />);
    const trigger = screen.getByRole("button", { name: title });
    await user.click(trigger);
    await user.click(trigger);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    await user.click(trigger);
    await user.click(screen.getByRole("button", { name: "Outside" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    await user.click(trigger);
    act(() => screen.getByRole("button", { name: "Outside" }).focus());
    expect(screen.getByRole("button", { name: "Outside" })).toHaveFocus();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("shows only three buttons and supports keyboard selection", async () => {
    const user = userEvent.setup();
    render(<Settings />);
    await user.click(screen.getByRole("button", { name: title }));
    const panel = screen.getByRole("dialog", { name: title });
    expect(within(panel).getAllByRole("button")).toHaveLength(3);
    expect(panel.textContent).toBe(`${small}${regular}${large}`);
    expect(within(panel).queryByRole("slider")).not.toBeInTheDocument();
    expect(within(panel).queryByRole("heading")).not.toBeInTheDocument();
    await user.tab();
    expect(screen.getByRole("button", { name: large })).toHaveFocus();
    await user.keyboard(" ");
    expect(document.documentElement.dataset.textSize).toBe("large");
    expect(panel).toBeVisible();
  });

  it("keeps size-button clicks active after an intermediate blur", async () => {
    const user = userEvent.setup();
    render(<Settings />);
    await user.click(screen.getByRole("button", { name: title }));
    const selected = screen.getByRole("button", { name: regular });
    // WebKit can report no next focus target before pointer activation.
    fireEvent.blur(selected, { relatedTarget: null });
    await user.click(screen.getByRole("button", { name: large }));
    expect(screen.getByRole("dialog", { name: title })).toBeVisible();
    expect(screen.getByRole("button", { name: large })).toHaveAttribute("aria-pressed", "true");
    expect(document.documentElement.dataset.textSize).toBe("large");
    await waitFor(() => expect(window.localStorage.getItem(KEY)).toBe("large"));
  });

  it("keeps the live choice after save failure and supports a localized retry", async () => {
    const user = userEvent.setup();
    render(<Settings />);
    await user.click(screen.getByRole("button", { name: title }));
    const spy = vi.spyOn(window.localStorage, "setItem").mockImplementationOnce(() => { throw new Error("disk unavailable"); });
    await user.click(screen.getByRole("button", { name: large }));
    expect(document.documentElement.dataset.textSize).toBe("large");
    expect(await screen.findByRole("alert")).toHaveTextContent(en ? "could not be saved" : "未能保存");
    spy.mockRestore();
    expect(screen.getByRole("button", { name: large })).toHaveAttribute("title", expect.stringContaining(en ? "Click any size to retry" : "点击任一档位重试"));
    await user.click(screen.getByRole("button", { name: large }));
    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
    expect(window.localStorage.getItem(KEY)).toBe("large");
  });

  it("recovers invalid preferences and read failures without crashing", async () => {
    window.localStorage.setItem(KEY, "unsupported");
    const user = userEvent.setup();
    const { unmount } = render(<Settings />);
    await user.click(screen.getByRole("button", { name: title }));
    expect(screen.getByRole("alert")).toHaveTextContent(en ? "could not be read" : "无法读取");
    expect(document.documentElement.dataset.textSize).toBe("default");
    await user.click(screen.getByRole("button", { name: small }));
    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
    unmount();
    const read = vi.spyOn(window.localStorage, "getItem").mockImplementation((key) => {
      if (key === KEY) throw new Error("storage unavailable");
      return key === "manuscriptdock.locale" ? locale : null;
    });
    render(<Settings />);
    await user.click(screen.getByRole("button", { name: title }));
    expect(screen.getByRole("alert")).toHaveTextContent(en ? "could not be read" : "无法读取");
    read.mockRestore();
  });

  it("loads native preferences and preserves the final choice across queued writes", async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    let stored = "small";
    let release: (() => void) | undefined;
    invokeMock.mockImplementation(async (command, args) => {
      if (command === "get_ui_preferences") return { textSize: stored };
      if (command === "save_ui_preferences") {
        const size = (args as { textSize: string }).textSize;
        if (size === "large") await new Promise<void>((resolve) => { release = resolve; });
        stored = size;
        return;
      }
      throw new Error("Unexpected command");
    });
    const user = userEvent.setup();
    const { unmount } = render(<Settings />);
    await waitFor(() => expect(document.documentElement.dataset.textSize).toBe("small"));
    await user.click(screen.getByRole("button", { name: title }));
    await user.click(screen.getByRole("button", { name: large }));
    await user.click(screen.getByRole("button", { name: regular }));
    expect(document.documentElement.dataset.textSize).toBe("default");
    await act(async () => { release?.(); });
    await waitFor(() => expect(stored).toBe("default"));
    expect(window.localStorage.getItem(KEY)).toBeNull();
    unmount();
    render(<Settings />);
    await waitFor(() => expect(invokeMock).toHaveBeenCalledTimes(4));
    expect(document.documentElement.dataset.textSize).toBe("default");
  });

  it("does not let a late native startup read replace a user's choice", async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    let resolveRead: ((value: unknown) => void) | undefined;
    invokeMock.mockImplementation((command) => command === "get_ui_preferences"
      ? new Promise((resolve) => { resolveRead = resolve; })
      : Promise.resolve());
    const user = userEvent.setup();
    render(<Settings />);
    await user.click(screen.getByRole("button", { name: title }));
    await user.click(screen.getByRole("button", { name: large }));
    await act(async () => { resolveRead?.({ textSize: "small" }); });
    expect(document.documentElement.dataset.textSize).toBe("large");
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(screen.getByRole("button", { name: title })).toHaveFocus();
  });
});
