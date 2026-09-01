import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { I18nProvider, localizeBackendText, localizeSourceLabel, useI18n } from "./i18n";

function LocaleProbe() {
  const { locale, setLocale, text } = useI18n();
  return <div>
    <output data-testid="locale">{locale}</output>
    <p>{text("中文界面", "English interface")}</p>
    <button type="button" onClick={() => setLocale("zh-CN")}>中文</button>
  </div>;
}

describe("client internationalization", () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.head.innerHTML = '<meta name="description" content="initial">';
    Object.defineProperty(window.navigator, "language", { configurable: true, value: "zh-CN" });
    Object.defineProperty(window.navigator, "languages", { configurable: true, value: ["zh-CN"] });
  });

  it("uses the operating-system locale until the user stores a preference", async () => {
    Object.defineProperty(window.navigator, "language", { configurable: true, value: "en-US" });
    Object.defineProperty(window.navigator, "languages", { configurable: true, value: ["en-US"] });
    const user = userEvent.setup();

    render(<I18nProvider><LocaleProbe /></I18nProvider>);

    expect(screen.getByTestId("locale")).toHaveTextContent("en");
    expect(screen.getByText("English interface")).toBeVisible();
    expect(document.documentElement).toHaveAttribute("lang", "en");
    expect(document.querySelector('meta[name="description"]')).toHaveAttribute("content", "ManuscriptDock: a local-first manuscript submission workspace");

    await user.click(screen.getByRole("button", { name: "中文" }));
    expect(screen.getByTestId("locale")).toHaveTextContent("zh-CN");
    expect(window.localStorage.getItem("manuscriptdock.locale")).toBe("zh-CN");
    expect(document.querySelector('meta[name="description"]')).toHaveAttribute("content", "ManuscriptDock 投稿舱：本地优先的论文投稿准备工作台");
  });

  it("translates current dynamic system messages and never leaks an unknown Chinese system error into English", () => {
    expect(localizeBackendText("en", "无法创建模型设置目录：permission denied"))
      .toBe("The model-settings directory could not be created: permission denied");
    expect(localizeBackendText("en", "无法创建模型设置目录：权限不足"))
      .toBe("The model-settings directory could not be created: See the local audit record for system details.");
    expect(localizeBackendText("en", "主模型和备选模型均未完成回答：primary 连接超时"))
      .toBe("No configured model completed the answer: primary timed out.");
    expect(localizeBackendText("en", "一个尚未登记的中文系统错误"))
      .toBe("The operation could not be completed. Switch to Chinese for the original system detail, then retry or review the local audit record.");
  });

  it("localizes system provenance labels without translating manuscript content", () => {
    expect(localizeSourceLabel("en", "LaTeX 正文 · 片段 3")).toBe("LaTeX body · Fragment 3");
    expect(localizeSourceLabel("en", "PDF Markdown · 行 8")).toBe("PDF Markdown · Line 8");
    expect(localizeSourceLabel("en", "用户自定义章节样式")).toBe("用户自定义章节样式");
  });
});
