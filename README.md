# 视频内容提取器

提取抖音、B站和本地音视频的内容，把视频链接变成可整理、检索和导出的文字素材。

基于 Windows 桌面运行，使用 Tauri 2、React 19、TypeScript、Rust 和 Python。本次公开源码基于 v1.2.1 纯净发布版，保留现有页面和操作方式。

## 主要功能

- **视频链接提取**：处理抖音分享链接、B站 BV 号、视频链接和 b23.tv 短链接。
- **批量采集**：采集作者主页作品；B站支持最近 N 期和在结果内按点赞排序。
- **内容转写**：B站优先读取可用字幕，无字幕时转写音轨；支持本地音视频。
- **视频下载**：下载选中作品；B站分离的音视频轨由 FFmpeg 合并。
- **内容整理**：文案入库、标签、备注、检索、脚本保存与导出。
- **辅助创作**：接入用户自己的 AI 服务，进行内容分析、对话和写作。

各平台的可用性受视频权限、登录状态和平台接口变化影响。指定分 P 可处理对应内容；未指定时默认 P1。完整多 P 展开、超长视频分段及片段级续传仍有待完善。

## 本地数据与 API

仓库不包含 API 密钥、登录 Cookie、浏览器资料、个人素材、数据库和历史记录。首次运行后，请在设置中填写自己的服务配置。

AI 写作与语音识别使用不同的配置，具体以应用中的 API 配置指南为准。Windows 上保存的 API Key 使用 DPAPI 本机加密。调用外部 AI 或识别服务时，相应文字或音频会发送给所选服务。

为兼容现有纯净版，程序标识保留为 `com.qiushikl.bilibili`，数据目录为 `%APPDATA%\QiushiKLBilibiliPublic`，本地服务端口为 38190 / 38191，前端开发端口为 1421。

## 开发与构建

需要 Windows 10/11、Node.js、pnpm、Rust 1.88、Visual Studio C++ Build Tools 和 WebView2。

```powershell
pnpm install --frozen-lockfile
pnpm build
```

上面检查并构建前端。运行完整桌面程序还需要 Python、FFmpeg 和本地识别/向量检索运行依赖；本仓库仅发布源码，不包含这些体积较大的二进制文件或已有安装包。

准备桌面运行资源：

1. 从相应项目的可信发行渠道准备兼容的 `DirectML.dll`、`onnxruntime.dll`、`sherpa-onnx-c-api.dll` 和 `sherpa-onnx-cxx-api.dll`，放到 `src-tauri/resources/bin/`。这些文件未纳入 Git。
2. 运行 `src-tauri/scripts/prepare_runtime.ps1` 准备嵌入式 Python、Python 包及 FFmpeg；部分包需可用网络。
3. 打安装包前，将需要的应用运行 DLL 放入 `src-tauri/resources/app-runtime/`，并依运行环境准备微软 Visual C++ 运行库。第三方二进制的许可证应随发行物保留。
4. 完成资源准备后运行 `pnpm tauri dev`；打包运行 `pnpm tauri build --bundles nsis`。本地识别与知识库还需在应用中准备相应模型。

源码中的 `tools/prepare_public_release.py` 可从**仅含通用程序依赖**的资源目录整理运行时，不能以个人应用数据目录作为输入。运行时准备脚本下载的外部资源可能随上游更新，需要自行核对兼容性。

Python 侧 B站解析测试：

```powershell
python -m unittest discover -s src-tauri/resources/dy-mcp/tests -p "test_bilibili_*.py"
```

## 项目来源与许可

本项目基于 [douyin-creator-toolkit](https://github.com/lid664951-crypto/douyin-creator-toolkit)，并在秋实KL定制版与 B站扩展版基础上整理公开。保留原项目版权与 [MIT 许可证](LICENSE)；更名不改变第三方代码的来源。

B站依赖说明见 [THIRD_PARTY_NOTICES_BILIBILI.md](THIRD_PARTY_NOTICES_BILIBILI.md)。Python、FFmpeg、ONNX、DirectML 等外部运行依赖分别遵循各自许可证。

请仅处理有权访问和使用的内容，尊重原作者与平台规则。
