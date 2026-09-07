# 字体与阅读设置 / Typography and reading settings

- 状态：V0.49
- 范围：桌面应用的中文、英文界面、浮层、弹窗和应用生成的预览文字。

## 字号规则

采用四档常规字号与一档首页品牌字号。默认是 Meta 13/20、Body 15/24、Section 17/26、Page 20/30、Brand 26/36px。完整三档数值以[全局设计系统](../design-system/manuscriptdock/MASTER.md#42-字号层级)为准。

同一语义跨页面共用同一 token。正文、按钮、导航、输入和操作说明使用 Body，时间、版本、来源和短状态使用 Meta。卡片和分组使用 Section，当前页面与主要弹窗使用 Page。字号与行高一起变化，普通字重为 400，标题和必要强调为 500。

English uses the same semantic roles and sizes. User-authored manuscripts and quoted source material retain their original language. Small annotations must not carry instructions, errors, or decisions that belong in body text.

## 交互

右上角 Aa 按钮的可访问名称为“字体大小 / Text size”。点击打开非模态浮层；V0.49 浮层只显示更小 / Smaller、默认 / Default、更大 / Larger 三个按钮。选中背景与 `aria-pressed` 标识当前字号，不显示标题、预览、状态说明或关闭按钮。点击后立即全局生效，保持浮层打开。

打开时聚焦当前档位，Tab 移动、空格或 Enter 选择。Esc 关闭并回到触发按钮；再次点击 Aa、点击外部或焦点离开也会关闭。窗口变窄不能覆盖用户选择，文字通过换行与重排适配。

点击选项文字和选项背景同样有效。V0.48 改为在指针点击或焦点确实到达外部元素时关闭；不把标签激活前短暂的空焦点误判为离开浮层。

## 本机保存与失败恢复

原生桌面由 Rust 的 `get_ui_preferences` / `save_ui_preferences` 管理应用配置目录内的 `ui-preferences.json`，WebView 不接收文件路径或通用文件权限。数据只有 schemaVersion 与 small/default/large 枚举。先写临时文件并同步，再原子替换；与模型凭据、稿件和语言设置隔离。

浏览器开发预览使用独立的 `manuscriptdock.text-size.v1` localStorage 键。原生桌面以 Rust 文件为准。偏好覆盖全部稿件与工作区；切换页面不重置，重启时读取。

前端串行保存快速选择，以最后一次选择为准。迟到的启动读取不会撤销用户已经作出的选择。读取失败使用默认档；保存失败保留当次视觉效果。错误通过 Aa 异常标记、按钮悬停提示和读屏提醒说明，点击任一档位（包括当前档位）即可重试，不在浮层增加内容。后端使用稳定错误代码，不将系统路径和原始异常直接呈现在界面。

这只是应用阅读偏好；不会修改源稿字形、原始论文页面或已经生成的导出文件。

## 回归范围

- zh-CN / en × small / default / large，验证实际字号、对应行高和同类控件一致性。
- 1180、980、760px 桌面宽度；额外检查 390px 浏览器布局。
- 首页、概览、材料、声明编辑、期刊目标、检查修订、投稿包、版本、知识体、证据和模型设置。
- 首次默认、持久化恢复、错误恢复、快速连续切换、迟到读取、键盘和外部点击。
- 原生 Rust 文件保存与重新读取使用合成临时目录；浏览器工作区场景使用合成数据，不代表真实稿件导入或出版社投递验收。
