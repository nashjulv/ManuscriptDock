# 内置期刊网址核验（2026-09-06，V0.44）

检查对象为全部 17 条内置期刊入口。旧目录共 15 个独立域名，当前网络的系统 DNS
均返回 `198.18.x.x` 虚拟地址，V0.43 会在连接前统一拒绝。V0.44 使用同一受控读取器
自动进行加密 DNS 恢复和 HTTP 兼容，没有为某一期刊放松公网校验。

以下是 V0.44 真实 Rust 读取器的一轮结果，未授权额外域名、未使用 Cookie 或登录。
“主页已读取”只代表正文可解码，不代表官方身份已经独立确认或完整要求已取得。
网站响应随网络、时间和反爬策略变化，不把一次失败推断成网址永久失效。

| 期刊 | 目录入口／修正 | 当次结果 |
| --- | --- | --- |
| 计算机学报 | `http://cjc.ict.ac.cn/`，修正旧 HTTPS 记录 | 直接 HTTP；独立指南实测取得约 2,800 字，无 pending，也未探测 HTTPS |
| 计算机研究与发展 | `https://crad.ict.ac.cn/` | 主页已读取，发现 `/tougaozhinan` |
| 软件学报 | `https://www.jos.org.cn/`，修正无 www 的旧入口 | 修正后前一轮读到主页及指南链接；最终复查 200 但无正文，访问存在波动 |
| 自动化学报 | `https://www.aas.net.cn/` | 主页已读取，发现 `/news/tgxz.htm` |
| 中文信息学报 | `http://jcip.cipsc.org.cn/` | 自动 HTTP 后主页已读取，未发现指南，不能算要求获取成功 |
| 模式识别与人工智能 | `http://prai.hfcas.ac.cn/`，按学会介绍修正协议 | 加密 DNS 成功；HTTP 200 但无可读正文 |
| 中国图象图形学报 | `https://www.cjig.cn/` | 主页已读取，发现 HTTP 指南候选 |
| 智能系统学报 | `https://tis.hrbeu.edu.cn/` | 主页已读取，发现指南候选 |
| Artificial Intelligence | `https://www.sciencedirect.com/journal/artificial-intelligence` | 403，保留拒绝访问结果 |
| IEEE TPAMI | `https://www.computer.org/csdl/journal/tp` | 主页已读取，未发现指南 |
| International Journal of Computer Vision | `https://link.springer.com/journal/11263` | 跳转至 `idp.springer.com`，按共同跨域规则等待官方身份确认 |
| Journal of Machine Learning Research | `https://www.jmlr.org/` | 主页已读取，未发现指南 |
| Transactions of the ACL | `https://transacl.org/` | 服务器未声明 gzip，补上有大小限制的解压后主页已读取；未发现指南 |
| IEEE TNNLS | `https://cis.ieee.org/publications/t-neural-networks-and-learning-systems` | 主页已读取，未发现指南 |
| Pattern Recognition | `https://www.sciencedirect.com/journal/pattern-recognition` | 403，保留拒绝访问结果 |
| Journal of Artificial Intelligence Research | `https://www.jair.org/` | 主页已读取，未发现指南 |
| Knowledge-Based Systems | `https://www.sciencedirect.com/journal/knowledge-based-systems` | 403，保留拒绝访问结果 |

最终按原始协议优先复查，11 条入口取得可解码主页，3 条被站点以 403 拒绝，1 条停在跨域身份确认，2 条无正文。
计算机学报另行验证了真正的指南发现和正文读取；其余指南候选不在此表中冒充已完成要求采集。

地址修正均依据官方资料：[计算所介绍](https://www.ict.cas.cn/xscbw/jsjxb/)、
[软件学报指南](https://www.jos.org.cn/jos/site/menu/20210909102616001?id=20210909102616001)、
[中国自动化学会介绍](https://www.caa.org.cn/Content/61.html)。其他入口暂未据此证明需要改址，
不凭超时、403 或登录跳转猜测替代地址。后续真实站点适配应单独验证正文入口。

离线回归覆盖任意合成域名、HTTP／HTTPS、精确域名授权、私网与虚拟 DNS 混合结果、
解析服务错误、压缩炸弹、旧 HTTP 确认记录和已保存目标的来源修正。
真实网络检查独立于默认测试：

```bash
cargo test -p manuscriptdock-desktop live_public_journal_https_probe -- --ignored --nocapture
cargo test -p manuscriptdock-desktop live_cjc_legacy_source_capture -- --ignored --nocapture
```
