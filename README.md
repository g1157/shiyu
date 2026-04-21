<div align="center">
  <img src="public/logo.png" alt="拾语" width="120" />
  <h1>拾语 (Shiyu)</h1>
  <p><strong>增强版外刊精读 · 原版书阅读桌面应用</strong></p>
  <p>图书书架 / EPUB 原生阅读器 · AI 非阻塞查词查句 · FSRS 间隔复习</p>
  <p>
    <a href="https://github.com/g1157/shiyu/releases">⬇️ 当前发行版</a>
    ·
    <a href="https://github.com/amluckydave/shiyu">上游原版项目</a>
  </p>

  <br/>

  [![Version](https://img.shields.io/badge/version-0.2.4-brightgreen.svg)](https://github.com/g1157/shiyu/releases)
  [![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
  [![Tauri](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white)](https://v2.tauri.app)
  [![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS-lightgrey?logo=apple&logoColor=white)](https://github.com/g1157/shiyu/releases)

</div>

---

## 📖 项目介绍

拾语是一款开源的英语学习桌面应用，集成 AI 翻译、文章阅读标注、生词/句子管理、SRS 间隔复习、思维导图、OCR 导入、EPUB 提取等功能。

基于 **Tauri 2 + Vue 3 + Rust** 构建，数据全部存储在本地，隐私安全。

> 当前仓库 **`g1157/shiyu`** 是基于上游 **`amluckydave/shiyu`** 持续演进的增强版桌面发行线。  
> 与原版相比，本仓库当前重点补上了 **EPUB 作为图书导入/书架/原生阅读器**、**图书内快查与回跳锚点**、**更偏阅读器取向的深浅色主题** 等能力。

<div align="center">
  <img src="public/intro.gif" alt="拾语演示" width="800" />
</div>

## ✨ 核心功能

| 功能 | 说明 |
|---|---|
| 📖 **文章阅读 & 标注** | 沉浸式 Markdown 阅读器，划词后后台快速查词/查句，不打断阅读 |
| 🧠 **AI 思维导图** | 一键分析文章结构，中英双语切换，缓存秒开 |
| 🤖 **AI 翻译 & 解析** | 流式翻译，句子成分彩色标注，兼容 DeepSeek/OpenAI |
| 📝 **生词本 & 句子库** | 搜索、批量操作、来源跳转、TTS 朗读 |
| 🔄 **SRS 间隔复习** | FSRS 算法闪卡，4 级评分，键盘快捷键 |
| ✍️ **在线编辑** | md-editor-v3 全功能编辑器，实时预览 |
| 📚 **EPUB 图书书架** | 直接导入为图书，保留目录、进度、章节定位与书内阅读流 |
| 📷 **OCR 导入** | PP-StructureV3 识别 + AI 校正，一键导入 |
| 📦 **数据管理** | JSON 导入/导出，支持合并或覆盖 |

## 🆚 当前增强版 vs 上游原版

| 维度 | 当前仓库 `g1157/shiyu` | 上游原版 `amluckydave/shiyu` |
|---|---|---|
| EPUB 入口 | 直接作为**图书**进入书架 | 以文章提取/拆分为主 |
| 阅读器形态 | 文章阅读器 + 图书阅读器双轨 | 以文章阅读器为核心 |
| 图书内快查 | 支持书内快查、句库/生词锚点回跳 | 未以图书阅读器为中心展开 |
| 主题与阅读 UI | 针对图书阅读场景持续调过 dark mode、目录、侧边快译 | 保留原版风格 |
| 发布地址 | `https://github.com/g1157/shiyu/releases` | `https://github.com/amluckydave/shiyu/releases` |

如果你需要的是**原始项目介绍和原版发行包**，请查看上游仓库。  
如果你需要的是**当前这条增强版桌面阅读线**，请以本仓库 README、Release 和 exe 为准。

## 🛠 技术栈

| 层 | 技术 |
|---|---|
| 前端 | Vue 3 + TypeScript + Vite + Vue Router + Pinia |
| 桌面运行时 | Tauri 2 |
| 后端 | Rust + rusqlite (SQLite) + reqwest + tokio |
| AI | DeepSeek / OpenAI 兼容接口（流式 SSE） |
| 解析渲染 | epub + html2md + scraper + marked + md-editor-v3 |
| 思维导图 | markmap-lib + markmap-view |
| 复习算法 | ts-fsrs (FSRS) |
| 语音 | edge-tts-universal（三级缓存） |

## 🚀 快速开始

### 环境要求

- Node.js 18+ / pnpm
- Rust stable toolchain
- Tauri 2 平台依赖（Windows: VS C++ Build Tools + WebView2）

### 安装 & 运行

```bash
# 安装依赖
pnpm install

# 开发模式
pnpm tauri dev

# 生产构建
pnpm tauri build
```

### 首次配置

打开应用 → 设置页面，配置：
- `API Key` — AI API 密钥
- `API URL` — API 地址（默认 DeepSeek）
- `API Model` — 模型名称

## 🧭 阅读交互

- **选中单词**：点击后先后台快速查词，面板返回语境义项，再决定直接保存或打开编辑表单
- **选中句子**：先返回快速中文释义，需要时再点 **深度解析** 补句法结构
- **保存路径**：快速结果可直接入生词本/句库，也可以带着预填内容进入原编辑表单继续修改
- **阅读不中断**：快速查询不再先弹全屏遮罩，返回结果期间可以继续滚动阅读
- **图书阅读器**：支持目录导航、书内选词/选句、句库/生词锚点化回跳、图书阅读进度保存

## 📁 项目结构

```
src/
├── views/              # 页面
├── components/         # 组件（含 ArticleReader / BookReader）
├── composables/        # 组合式逻辑
├── stores/             # Pinia 状态管理
├── services/           # API 封装
├── constants/          # 应用常量
├── styles/             # 全局样式
└── utils/              # 阅读器辅助逻辑

src-tauri/
├── src/commands/       # Tauri 命令
├── src/repositories/   # 数据访问层
├── src/models.rs       # 数据模型
├── src/db.rs           # SQLite 初始化
└── tauri.conf.json     # 应用配置
```

## 💾 数据存储

| 数据 | 路径 |
|---|---|
| 数据库 | `~/.shiyu/shiyu.db` (SQLite) |
| 图片 | `~/.shiyu/images/` |
| TTS 缓存 | `~/.shiyu/tts-cache/` |

## 🔗 友情链接

- [LINUX DO](https://linux.do/) — 感谢 LINUX DO 社区的支持与推广
- [上游原版拾语](https://github.com/amluckydave/shiyu) — 当前增强版的上游来源

---

<div align="center">

## 📄 License

[MIT](./LICENSE) © 2026 amluckydave

Made with ❤️ for English learners

</div>
