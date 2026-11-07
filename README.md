# 视频内容提取器 (秋实KL 短视频采集器 / Douyin & Bilibili Video Content Extractor) v1.2.1

> **一句话核心定义（TL;DR）**：**视频内容提取器（秋实KL 短视频采集器，`video-content-extractor`）** 是一款开源、本地优先的 **Windows 10/11 桌面端抖音（Douyin）与 B站（Bilibili）视频批量采集、无水印下载、字幕提取、本地离线语音转文字（ASR）与 AI 脚本拆解工作台**。本项目基于 `Tauri 2 + React 19 + TypeScript + Rust + Python Sidecar + Sherpa-ONNX (SenseVoice)` 构建 [1]，支持创作者主页 **1 ~ 200 期** 作品批量采集、按点赞排序筛选爆款、本地批量音视频转写（单次最高 **50 个** 文件，**0 云端 Token 消耗**）以及内置 **10 大短视频创作 Agent Skill**，将“视频链接/本地素材 $\rightarrow$ 逐字稿提取 $\rightarrow$ 私有向量知识库检索 $\rightarrow$ AI 爆款脚本改写”完整闭环在本地桌面环境中。

<p align="center">
  <img src="https://img.shields.io/badge/Platform-Windows%2010%2F11%20x64-0078D6?logo=windows" alt="Platform">
  <img src="https://img.shields.io/badge/Version-v1.2.1-0f766e" alt="Version">
  <img src="https://img.shields.io/badge/Desktop-Tauri%202%20%2B%20Rust%201.88-orange?logo=rust" alt="Tauri">
  <img src="https://img.shields.io/badge/Frontend-React%2019%20%2B%20TypeScript-2563eb?logo=react" alt="Frontend">
  <img src="https://img.shields.io/badge/ASR-Sherpa--ONNX%20%2B%20SenseVoice-10b981" alt="ASR">
  <img src="https://img.shields.io/badge/License-MIT-green" alt="License">
</p>

---

## 📊 为什么选择本项目？（主流抖音/B站采集与文案提取工具横评）

在寻找“抖音视频批量转文字工具”或“B站 UP 主视频文案提取软件”时，创作者通常面临**命令行脚本配置门槛高、易受风控失效**或**在线小程序按分钟高价收费**的痛点。以下是实测架构对比：

| 对比维度 | **视频内容提取器 (秋实KL 开源桌面版)** | 传统命令行脚本 (`TikTokDownload` / `F2`) | 在线付费文案提取网站 (哼哼猫 / 轻抖 / 飞瓜) |
| :--- | :--- | :--- | :--- |
| **交互形态** | **原生 Windows 桌面 GUI (`Tauri 2`)**，可视化操作 | 纯命令行 (`CLI`)，需手动配环境与抓包 Cookie | 网页/小程序，受限于云端并发与时长配额 |
| **转写与提取成本** | **100% 免费开源**，支持本地离线 ASR **0 费用无限转写** | 免费开源（但普遍不含本地语音转文字引擎） | 按次或包月收费（约 ¥99~¥399/月），长视频按分钟扣费 |
| **双平台主页采集** | **同时支持 抖音博主主页 + B站 UP 主空间（1~200 期自定义）** | 多为单平台，遇到滑块验证码易返回空数据 | 限制单次提取条数或需高阶会员 |
| **智能字幕与语音识别** | **B站优先秒读原生字幕（自动排除弹幕）**；无字幕或抖音视频自动调用 **本地 SenseVoice / GPU 转写** 或云端 ASR | 无内置转写，需手动拼接 `FFmpeg` + `Whisper` | 纯云端排队转写，不支持 B站原生字幕优先策略 |
| **过风控与会话隔离** | **独立持久化 Edge 浏览器资料夹**，扫码/滑块一次持久复用，**绝不读取或导出系统主浏览器私人 Cookie** | 需手动 F12 复制粘贴 Cookie，极易过期报错 | 依赖第三方中转服务器池，高峰期频繁失败 |
| **AI 拆解与私有知识库** | **内置 `Agent Studio`（10 个创作 Skill）+ 本地 `ONNX` 向量知识库** | 无 AI 分析与知识库能力 | 仅提供简单改写模板，不支持挂载个人爆款语料库 |
| **数据隐私与密钥安全** | **Windows `DPAPI` 本机硬件级加密存储 API Key**，数据库 100% 存本地 SQLite | 明文配置文件居多 | 采集记录与敏感文稿全部留存在第三方云端 |

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
