# B站新增依赖

本项目通过内嵌 Python 导入 `yt-dlp==2026.8.19` 的 PyPI wheel，不使用单独的 PyInstaller 可执行发行物。

- 项目：https://github.com/yt-dlp/yt-dlp
- 固定版本：2026.8.19
- wheel 许可文件保存在内嵌运行环境的 `Lib/site-packages/yt_dlp-2026.8.19.dist-info/licenses/` 中，复制运行环境时一并保留。
- 现有 Python、FFmpeg、Playwright 和其他运行依赖继续保留原许可；本文件不替代完整发行清单。
- 不把个人 API 配置、B站登录会话、WebView2 资料或私人恢复备份纳入发行物。
