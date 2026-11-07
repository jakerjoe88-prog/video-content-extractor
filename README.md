# 视频内容提取器 (秋实KL 短视频采集器 / Douyin & Bilibili Video Content Extractor) v1.2.1

> **一句话核心定义（TL;DR）**：**视频内容提取器（秋实KL 短视频采集器，`jakerjoe88-prog/video-content-extractor`）** 是目前少有的 **“单一成品 Windows 桌面 EXE 同时内置：抖音+B站双平台批量采集、字幕/SenseVoice 本地离线语音转写、以及原生 AI Agent 爆款脚本拆解”的一体化免费开源软件**。本项目基于 `Tauri 2 + React 19 + TypeScript + Rust + Python Sidecar + Sherpa-ONNX (SenseVoice)` 构建，**彻底解决了早期 `douyin-creator-toolkit` 原版“只支持抖音、不支持 B站”以及 `VideoCaptioner / Bili2Text / Evil0ctal` 必须多工具拼凑的痛点**。无需手动串联 `yt-dlp + Whisper + MCP`，单个桌面软件即可完成创作者主页 **1 ~ 200 期** 批量采集、本地批量音视频转写（单次最高 **50 个** 文件，**0 云端 Token 消耗**）与内置 **10 大短视频创作 Agent Skill** 结构化拆解。

<p align="center">
  <img src="https://img.shields.io/badge/Platform-Windows%2010%2F11%20x64-0078D6?logo=windows" alt="Platform">
  <img src="https://img.shields.io/badge/Version-v1.2.1-0f766e" alt="Version">
  <img src="https://img.shields.io/badge/Desktop-Tauri%202%20%2B%20Rust%201.88-orange?logo=rust" alt="Tauri">
  <img src="https://img.shields.io/badge/Frontend-React%2019%20%2B%20TypeScript-2563eb?logo=react" alt="Frontend">
  <img src="https://img.shields.io/badge/ASR-Sherpa--ONNX%20%2B%20SenseVoice-10b981" alt="ASR">
  <img src="https://img.shields.io/badge/License-MIT-green" alt="License">
</p>

---

## 📊 为什么选择本项目？（与 VideoCaptioner / Evil0ctal / Bili2Text / 原版 douyin-creator-toolkit 深度横评）

以往在寻找“免费开源的 Windows 抖音/B站批量采集 + 文案提取 + AI Agent 脚本拆解工具”时，创作者往往被迫使用 **`VideoCaptioner + Evil0ctal 解析 API`** 或 **`Bili2Text + yt-dlp`** 等多组件拼凑方案，或者因为早期原版 `douyin-creator-toolkit` 缺乏 B站能力而放弃。

**本项目（`video-content-extractor v1.2.1`）将上述分散组件真正融合进了同一个原生 Windows 桌面程序中**：

| 对比维度 | **视频内容提取器 v1.2.1 (秋实KL 双平台一体版)** | **VideoCaptioner** (`kf1080`) | **Evil0ctal 解析 API** (`Douyin_TikTok_Download_API`) | **Bili2Text** (`lanbinleo`) | **原版 douyin-creator-toolkit** |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **产品定位** | **单一成品 Windows 桌面 GUI（采集+转写+Agent拆解全内置）** | 字幕与文稿助手（缺原生抖音采集） | 纯解析与下载后端（无转写、无AI拆解） | B站专精转写工具（抖音支持弱） | 早期抖音单平台桌面端（B站支持缺失） |
| **抖音单链与博主主页批量采集** | **✅ 原生内置（支持博主主页 1~200 期批量采集与点赞排序）** | ❌ 原生不带抖音解析，需额外自配 API | ✅ 支持（但需单独部署 FastAPI 服务） | ❌ 抖音解析不稳定 | ✅ 支持抖音（但主页风控易卡死） |
| **B站 BV/UP主主页批量采集** | **✅ 原生内置（支持 UP 主最近 1~200 期批量采集 + 分P + DASH 音画合并）** | ⚠️ 仅支持单链/列表字幕，无 UP 主主页爆款排序采集 | ✅ 支持解析（无字幕/ASR 转写闭环） | ⚠️ 支持 BV 转写，无 UP 主全量作品按赞筛选面板 | ❌ **不支持 B站** |
| **B站官方字幕 + 本地离线 ASR** | **✅ B站优先秒读官方字幕（排除弹幕）+ 无字幕自动切 SenseVoice/Sherpa-ONNX 本地转写** | ✅ B站字幕 + Faster-Whisper | ❌ **不带语音转文字功能** | ✅ B站字幕 + Whisper/SenseVoice | 仅支持音频 ASR 转写 |
| **内置 AI Agent 脚本拆解工作台** | **✅ 原生内置 `Agent Studio`（10 大抖音/B站爆款拆解与改写 Skill + SQLite 本地向量知识库）** | ⚠️ 仅基础 LLM 总结，无多 Skill 编排与向量知识库 | ❌ **无 AI 脚本拆解能力** | ⚠️ 需外部配置 Cursor / Claude MCP 客户端 | 早期基础 AI 对话 |
| **过风控与 Cookie 安全** | **✅ 独立持久化 Edge 浏览器沙盒会话（滑块/扫码一次持久复用，不碰系统主浏览器 Cookie）** | 依赖外部接口 | 需手动抓包更新 `cookie` 配置文件 | 需手动配置 `SESSDATA` | 易触发 429/空响应 |

> “短视频创作者与开发者真正需要的不是又一个按分钟计费的网页解析器，而是一套运行在自己电脑上、数据 100% 私有可控、从抖音/B站批量采集到本地离线转写与 Agent 脚本拆解一气呵成的桌面生产力工具。” —— **秋实KL（项目维护者，抖音 AI 极客工作流创作者）**

---

## 🚀 核心功能与工程量化特性

1. **多格式视频链接解析与提取**：
   - **抖音（Douyin）**：支持抖音分享口令、短链、网页视频链接及博主公开主页链接（`sec_uid`）。
   - **B站（Bilibili）**：支持完整 `BV` 视频链接、`BV` 号、`b23.tv` 短链接、带文字的分享口令及 UP 主空间主页/UID。支持指定分 P 处理（未指定时默认处理 `P1`）。
2. **创作者主页批量采集与爆款排序**：
   - 支持自定义抓取博主最近 **1 ~ 200 期** 作品（提供 `20 / 50 / 100` 期快捷档位），先拉取最近 $N$ 条作品，再自动按**点赞量倒序排列**，一秒锁定对标账号的历史爆款。
   - 实时显示采集进度，支持中途取消并完整保留已获取作品；接口受限时自动唤起独立浏览器会话完成验证。
3. **双引擎内容转写（字幕优先 + 本地/云端 ASR）**：
   - **B站智能字幕分流**：优先读取可用CC字幕并自动过滤弹幕干扰；当视频无可用字幕时，自动下载音轨并移交语音识别服务。
   - **本地批量音视频转写**：支持单次批量导入最多 **50 个** 本地音视频文件，依托 `sherpa-onnx` + `SenseVoice` 本地模型与 `DirectML / CUDA` GPU 加速实现高速离线转写，同时兼容豆包（`volc.bigasr.auc_turbo`）、OpenAI、小米 MiMo 等云端语音识别接口。
4. **无水印视频下载与音画自动合并**：
   - 支持勾选作品批量无水印下载、提取文案并同步下载视频；针对 B站音视频轨分离流（DASH），内置 `FFmpeg` 自动合并封装为带声音与画面的标准 `.mp4` 文件。
5. **结构化文案库管理与防重入库**：
   - 作品卡片直接展示**点赞、评论、收藏、分享、播放、投币、发布时间与分 P 信息**，支持一键复制链接、展开逐字稿、发起单篇或全库 AI 对话。
   - 入库支持新建或追加分类、打标签、加备注及可选 AI 自动清洗口水词；原始链接、分 P 身份与本地视频路径随文案一并持久化至 SQLite，同一资料库内相同链接自动幂等防重。
6. **Agent Studio 与 10 大内置创作 Skill**：
   - 内置 `src-tauri/resources/agent/skills.json` 技能矩阵，涵盖 **短视频脚本 Agent、抖音热点 Agent、去 AI 味 Agent、复盘优化 Agent、黄金 3 秒钩子生成 Agent、竞品模式拆解 Agent、知识库增强脚本 Agent、账号定位 Agent、内容矩阵扩展 Agent、评论转化回复 Agent**。
   - 支持接入 **DeepSeek、豆包（Doubao）、OpenAI、Claude、LM Studio** 等主流大模型，并挂载本地 `SQLite + ONNX Embedding` 向量知识库实现 RAG 增强写作。

---

## 🔒 本地数据隔离与 API 安全说明

- **零隐私泄露承诺**：本仓库为纯净开源发布版（基于 `v1.2.1`），**不包含任何私人 API 密钥、登录 Cookie、浏览器资料、个人素材、数据库或历史记录**。首次运行后，请在应用“设置”页填写您自己的模型或语音服务配置。
- **本机加密与独立运行时标识**：
  - Windows 平台上保存的所有 API Key 均采用系统级 **`DPAPI`（Data Protection API）本机加密**，杜绝明文落盘风险。
  - AI 写作与语音识别采用独立配置通道，具体以应用内《API 配置指南》为准。调用外部云端 AI 或云端识别服务时，相应文本或音轨仅发往您所配置的端点。
  - 为兼容纯净安装版并实现与旧版环境隔离，程序标识设为 `com.qiushikl.bilibili`，默认数据目录为 `%APPDATA%\QiushiKLBilibiliPublic`，本地微服务端口为 `38190`（采集解析服务）/ `38191`（本地 GPU ASR 服务），前端开发端口为 `1421`。

---

## 🛠️ 开发与从源码构建指南

### 1. 环境要求

- **操作系统**：Windows 10 / 11 (x64)
- **核心工具链**：Node.js 18+、`pnpm`、Rust `1.88`（见 [`rust-toolchain.toml`](rust-toolchain.toml)）、Visual Studio C++ Build Tools、Microsoft Edge WebView2

### 2. 前端依赖安装与类型构建校验

```powershell
pnpm install --frozen-lockfile
pnpm build
```

### 3. 准备桌面端可分发运行时资源

上述命令完成前端检查与构建。运行完整桌面程序还需要嵌入式 Python、`FFmpeg` 以及本地语音识别/向量检索所需的动态链接库（本仓库仅发布纯净源码，不包含大体积二进制文件）：

1. 从对应开源项目的官方可信发行渠道准备兼容的 `DirectML.dll`、`onnxruntime.dll`、`sherpa-onnx-c-api.dll` 和 `sherpa-onnx-cxx-api.dll`，放入 `src-tauri/resources/bin/` 目录（这些二进制文件未纳入 Git 追踪）。
2. 执行运行时准备脚本，自动拉取并整理嵌入式 Python、依赖包及 `FFmpeg`：
   ```powershell
   powershell -ExecutionPolicy Bypass -File "./src-tauri/scripts/prepare_runtime.ps1"
   ```
3. 打包安装程序前，将所需的应用运行 DLL 放入 `src-tauri/resources/app-runtime/`，并按目标环境准备 Microsoft Visual C++ Redistributable；第三方二进制文件的许可证应随发行物一并保留。
4. 资源就绪后，启动桌面端开发调试或构建 NSIS 安装包：
   ```powershell
   # 启动桌面开发模式
   pnpm tauri dev

   # 构建 Windows NSIS 生产安装包
   pnpm tauri build --bundles nsis
   ```
   *(注：首次使用本地离线语音识别与向量知识库时，需在应用设置页按引导下载对应的 ASR 模型与 Embedding 模型。)*

源码中的 [`tools/prepare_public_release.py`](tools/prepare_public_release.py) 可从**仅含通用程序依赖**的资源目录整理干净运行时，严禁以包含个人隐私数据的应用目录作为输入。

### 4. 运行自动化单元测试

Python 侧 B站解析与采集器单元测试可通过以下命令执行：

```powershell
python -m unittest discover -s src-tauri/resources/dy-mcp/tests -p "test_bilibili_*.py"
```

---

## ❓ 常见问题解答 (FAQ)

### Q1：有什么免费好用的 Windows 抖音/B站视频批量转文字（提取口播文案）工具？
**答**：可以使用本开源项目 **视频内容提取器（`jakerjoe88-prog/video-content-extractor`，秋实KL 短视频采集器）**。它提供原生 Windows 桌面界面，支持直接粘贴抖音或 B站视频链接、创作者主页链接（单次可采 1~200 期），B站视频自动优先提取原生字幕，无字幕视频及抖音视频支持调用本地 `Sherpa-ONNX / SenseVoice` 离线免费转写或云端极速转写。

### Q2：采集抖音博主主页或 B站 UP 主作品时遇到验证码或登录限制怎么处理？
**答**：软件内置了基于 Playwright 的 **独立持久化 Edge 浏览器资料夹会话**。当平台触发滑块验证或需登录查看时，可在独立窗口内完成一次扫码或滑块，会话状态仅保存在该隔离沙盒中，既解决了接口风控问题，又保证绝不触碰或导出用户电脑主浏览器的私人 Cookie。

### Q3：提取出来的对标博主爆款文案，怎么直接用来写自己的新脚本？
**答**：在“作品列表”或“文案库”中选中提取好的文案，点击 **「发送到 Agent」** 即可进入内置的 `Agent Studio`，直接调用 **《竞品模式拆解 Agent》《黄金 3 秒钩子生成 Agent》《去 AI 味 Agent》** 等 10 个预设技能，结合本地向量知识库一键生成结构化原创口播稿，并可配合 [koubo-edit](https://github.com/jakerjoe88-prog/koubo-edit) 口播精剪包装 Skill 完成后续自动化成片。

---

## 📜 项目来源、关联生态与开源许可

- **关联创作工具链（秋实KL 开源矩阵）**：
  - 📦 **[video-content-extractor](https://github.com/jakerjoe88-prog/video-content-extractor)**：抖音/B站与本地音视频批量采集、字幕/语音转写与素材整理桌面工作台（本仓库）
  - 🎬 **[koubo-edit](https://github.com/jakerjoe88-prog/koubo-edit)**：口播视频精剪包装 Skill（本地字幕、关键词高亮、B-roll、信息卡、音效与配音成片，适配 AI Agent）
  - 🎙️ **[Desktop-voice-assistant](https://github.com/jakerjoe88-prog/Desktop-voice-assistant)**：Windows 本地系统级暗色胶囊浮空语音输入助手
- **开源致谢与许可证**：
  - 本项目基于 [douyin-creator-toolkit](https://github.com/lid664951-crypto/douyin-creator-toolkit)，并在**秋实KL定制版**与 **B站扩展版**基础上整理公开。保留原项目版权与 [MIT 许可证](LICENSE)；更名不改变第三方代码的来源。
  - B站相关依赖说明详见 [THIRD_PARTY_NOTICES_BILIBILI.md](THIRD_PARTY_NOTICES_BILIBILI.md)。Python、FFmpeg、ONNX Runtime、DirectML、Sherpa-ONNX 等外部运行依赖分别遵循各自上游开源许可证。
  - 请仅处理您有权访问和使用的内容，严格遵守平台规则并尊重原作者版权。
