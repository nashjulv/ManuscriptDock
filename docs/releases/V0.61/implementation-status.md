# V0.61 PDF 加载异常修复

## 原因与修复

本机开发客户端运行时捕获到 `pdf-extract 0.12.0` 的 `missing unicode map and encoding` panic。此前 `dispatch_job` 启动的后台工作线程在解析异常后退出，任务记录仍为 running，前端 `waitForJob` 持续轮询。

- PDF 只读解析边界捕获 panic，返回 `PDF_TEXT_EXTRACTION_FAILED`，给出 DOCX／重新导出 PDF 的恢复建议；不猜测缺失字符，也不修改原稿。
- 两种核心任务执行入口捕获 action panic，保存并通知 `JOB_WORKER_PANICKED` 终止状态；请求锁正常释放，前端恢复操作入口。
- 产品身份与安装元数据统一为 V0.61／0.61.0。重新生成包含前端资源的 macOS 调试应用包，避免继续使用旧构建产物。

## 实际验证

- `npm run test --workspace @manuscriptdock/desktop -- --maxWorkers=1 --minWorkers=1`：73 项通过。新增中英文 PDF 读取失败和通用后台异常测试，覆盖任务轮询结束、错误提示、按钮解除禁用、再次选文件。
- `cargo test -p manuscript-core`：18 项单元测试、10 项集成测试通过。合成 Type1 字体 PDF 缺少字符映射和备用编码，验证解析异常进入 needs_input、原稿字节不变、随后可打开有效 PDF；另测后台 panic 的失败持久化、通知和请求锁释放。
- `npm run tauri -- build --debug --bundles app` 成功，包含 TypeScript 检查和生产前端构建。产物位于 `target/debug/bundle/macos/ManuscriptDock.app`。
- 原生 macOS 窗口已显示 V0.61，页面来自 `tauri://localhost`；首页、推荐入口、原生文件选择取消、AI 设置读取均实际验证，完成后停留首页。
- 原生文件选择框的自动输入不稳定，合成异常 PDF 的原生完整链路未完成；上述核心与前端测试不是该项原生验收的替代声明。未调用外部模型，未验证 Windows，未发布正式签名安装包。
- 第一轮并发前端测试有一项既有用例超过 5 秒；后续单工作进程完整测试通过，未放宽超时或跳过用例。

## Internationalization review

检查 `zh-CN` 与 `en` 的 PDF 错误、后台异常、失败后重新选文件和版本身份。新增 `JOB_WORKER_PANICKED` 的双语稳定错误码映射，更新 PDF 失败文案，四个双语组件回归用例通过。原稿和引用内容不翻译；未新增导出文案。
