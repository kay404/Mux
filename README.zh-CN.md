<h1 align="center">Mux</h1>
<p align="center">
  <b>在项目之间自由穿梭</b><br>
  <a href="#安装">安装</a> •
  <a href="#功能">功能</a> •
  <a href="#工作原理">工作原理</a> •
  <a href="#开发模式">开发</a><br>
  <a href="README.md">English</a> | 简体中文
</p>

---

macOS 菜单栏应用，一键切换开发工具的项目窗口。

你同时打开了 6 个以上的 VSCode 窗口，Cmd+Tab 列出 30 多个窗口，找到目标项目太慢了。Mux 解决这个问题。

![macOS](https://img.shields.io/badge/平台-macOS-lightgrey)
![Tauri](https://img.shields.io/badge/构建-Tauri%20v2-blue)
![License](https://img.shields.io/badge/许可证-MIT-green)

## 功能

- 自动检测正在运行的 VSCode、VSCode Insiders 和 Cursor
- 显示每个窗口的项目名称和完整文件路径
- 点击即可将项目窗口切到最前面（两步聚焦：激活应用 + 提升窗口）
- 显示每个 IDE 的真实应用图标
- 键盘导航（方向键、回车、Escape）
- 暗色主题，macOS 毛玻璃模糊效果
- 后台每 5 秒刷新，保持列表实时更新（仅在弹窗可见时轮询）
- 首次启动自动引导授予辅助功能权限
- 零配置，开箱即用

## 界面预览

```
┌─────────────────────────────────┐
│  [图标] Visual Studio Code      │
│  ● my-project    ~/work/my-p..  │
│    api-server    ~/work/api-..  │
│    frontend      ~/dev/front..  │
│                                 │
│  [图标] Cursor                  │
│    dashboard     ~/work/dash..  │
└─────────────────────────────────┘
```

## 系统要求

- macOS（建议 Sonoma 或更高版本）
- 辅助功能权限（首次启动时授予一次即可）

## 安装

### 从源码构建

```bash
git clone https://github.com/kay404/Mux.git
cd Mux
npm install
npm run tauri build
```

构建产物位于 `src-tauri/target/release/bundle/macos/Mux.app`，拖到 `/Applications` 即可。

### 开发模式

```bash
npm install
npm run tauri dev
```

## 工作原理

1. 通过 `NSRunningApplication` 按 Bundle ID 查找正在运行的 IDE
2. 通过 macOS 辅助功能 API（`AXUIElement`）读取窗口标题
3. 从标题中解析项目名称（兼容默认和自定义标题格式）
4. 查询 VSCode 的 `state.vscdb`（SQLite）获取完整路径
5. 两步聚焦：`NSRunningApplication.activate` + `AXUIElementPerformAction(kAXRaiseAction)`

## 支持的应用

| 应用 | Bundle ID |
|------|-----------|
| Visual Studio Code | `com.microsoft.VSCode` |
| VSCode Insiders | `com.microsoft.VSCodeInsiders` |
| Cursor | `com.todesktop.230313mzl4w4u92` |

## 技术栈

- **Tauri v2** + Rust 后端
- **原生 JS/HTML/CSS** 前端
- **macOS 辅助功能 API**（通过 FFI + core-foundation 调用）
- **SQLite**（rusqlite）读取 VSCode 工作区状态

## 限制

- 仅支持 macOS
- 无法通过 Mac App Store 分发（辅助功能 API 需要非沙盒环境）
- Antigravity IDE 支持将在后续版本添加

## 项目结构

```
├── src-tauri/src/
│   ├── lib.rs              # 应用启动、托盘、弹出窗口、状态管理
│   ├── accessibility.rs    # macOS AX API：查找应用、列出窗口、聚焦
│   ├── title_parser.rs     # 从窗口标题解析项目名称
│   ├── path_resolver.rs    # 通过 state.vscdb 解析完整路径
│   └── icon_cache.rs       # 应用图标提取 + 磁盘/内存缓存
├── src/
│   ├── index.html          # 弹出窗口 HTML
│   ├── main.js             # 前端逻辑
│   └── styles.css          # 暗色毛玻璃主题
└── docs/
    ├── PRD.md              # 产品需求文档
    ├── DESIGN.md           # 视觉设计规范
    └── DEVELOPMENT.md      # 开发指南
```

## 许可证

MIT
