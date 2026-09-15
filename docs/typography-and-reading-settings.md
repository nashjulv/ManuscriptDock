# 字体与阅读设置 / Typography and reading settings

- 状态：V0.622
- 范围：桌面应用的中文、英文界面、浮层、弹窗和应用内预览文字。

## 字号与字体

界面统一使用四种语义字号：Meta、Body、Section、Page。首页标题复用 Page，不再单设品牌字号。右上角 Aa 提供更小、默认、更大三档；默认值等于原四档方案中的“更小”：

| 层级 | 更小 / Smaller | 默认 / Default | 更大 / Larger |
| --- | --- | --- | --- |
| Meta | 11px | 12px | 13px |
| Body | 13px | 14px | 15px |
| Section | 14px | 16px | 18px |
| Page | 20px | 22px | 24px |

正文、表单、按钮、错误和操作说明使用 Body；时间、版本、来源和短状态使用 Meta；分组使用 Section；页面标题使用 Page。正文行高为 1.6，标题为 1.5，控件随内容增高和换行。

字体优先使用 `PingFang SC`、`PingFang HK`，依次回退到 `Microsoft YaHei UI`、`Segoe UI`、sans-serif。正文使用 300 字重，标题和控件以 400 为主，必要强调最多 500；不合成粗体、不下载远程字体。

English shares the same type scale and system font stack. Manuscript text, source documents, and existing exports retain their original language and typography.

## 交互与保存

Aa 的可访问名称为“字体大小 / Text size”。浮层显示三个按钮，以选中背景和 `aria-pressed` 标识当前档位。点击后立即全局生效并保持浮层打开。打开时聚焦当前档位；Tab、空格和 Enter 可操作，Esc 关闭并返回触发按钮。外部点击或焦点移动关闭浮层。

原生桌面使用 Rust 的 `get_ui_preferences` / `save_ui_preferences` 保存 `ui-preferences.json`，schemaVersion 为 2，仅保存 small/default/large。读取旧版 schema 1 时，small 对应新 default，default/large/xlarge 对应新 large；新选择不会再次按旧方案解释。读取不会改写原文件，选择时才原子保存，和模型凭据、稿件、语言设置隔离。WebView 不接收通用文件权限。

浏览器开发预览使用 `manuscriptdock.text-size.v2` localStorage 键；无 v2 时按上述规则读取 v1，保留旧键。偏好覆盖全部稿件与页面，重启读取。快速选择按顺序保存，以最后选择为准；迟到的读取不会覆盖用户操作。读取失败使用默认大小，保存失败保留当次视觉效果并通过 Aa 标记、悬停文字和读屏提醒提示重试。

## 布局与原 UI 配色

沿用原方案的浅灰页面、细分隔线、黄绿色主按钮，详见[UI 视觉系统](ui-visual-system.md)。窄窗口使用换行和减少文件列表列数，不覆盖字号偏好。触屏控件保持至少 44px 点击高度。

## 回归范围

- Internationalization review：zh-CN / en 的三档标签、字号变量、持久化、旧设置迁移、错误重试、迟到读取、键盘与外部点击。
- Rust 三档保存重读、旧设置迁移与无关设置保护。
- 当前两项任务、材料清单、文件目录和 AI 弹窗的共享样式；源文件与导出排版不变。
- 旧版原生界面验证记录在 [V0.62 实施记录](releases/V0.62/implementation-status.md)。本轮按用户要求仅构建与自动化测试，不启动应用。
