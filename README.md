# Orchis Manager

<p align="center">
  <img src="src-tauri/icons/icon.png" width="128" height="128" alt="Orchis Logo">
</p>

<p align="center">
  <strong>🌸 Orchids 账号自动化管理工具</strong>
</p>

<p align="center">
  <a href="#功能特性">功能特性</a> •
  <a href="#技术栈">技术栈</a> •
  <a href="#安装使用">安装使用</a> •
  <a href="#开发指南">开发指南</a>
</p>

---

## 功能特性

### 🤖 自动化注册

- **全自动流程**：一键完成 Orchids 账号注册
- **邮箱验证码自动填充**：通过 IMAP 协议自动获取验证码并填入
- **系统级粘贴模拟**：使用 AppleScript 实现 Cmd+V 自动粘贴
- **Deep Link 自动唤起**：注册完成后自动回调本地 Orchids 客户端

### 📧 邮箱配置

- **多域名支持**：预设常用邮箱域名 (@huaan666.site, @qq.com, @gmail.com)
- **自定义域名**：支持手动输入任意邮箱域名
- **IMAP 配置管理**：可保存多个邮箱配置文件，快速切换

### 👥 账号管理

- **多账号支持**：管理多个 Orchids 账号
- **会话捕获**：自动捕获登录会话 Cookie
- **一键登出**：快速退出指定账号

### 🔌 API 代理

- **内置代理服务**：本地 API 代理服务器
- **请求转发**：支持自定义 API 端点转发

### 🌐 Gmail OAuth 2.0

- **OAuth 认证**：支持 Gmail OAuth 2.0 认证流程
- **安全访问**：无需存储明文密码

---

## 技术栈

| 层级       | 技术                       |
| ---------- | -------------------------- |
| **前端**   | Vue 3 + Vite + TailwindCSS |
| **后端**   | Rust + Tauri v2            |
| **邮件**   | IMAP + Gmail API           |
| **自动化** | AppleScript + WebView 注入 |

---

## 安装使用

### 系统要求

- macOS 10.15+
- Orchids 客户端已安装

### 快速开始

1. **下载安装包**

   ```bash
   # 从 Release 页面下载 Orchis.dmg
   ```

2. **安装应用**

   - 打开 DMG 文件
   - 将 Orchis.app 拖入 Applications 文件夹

3. **授权权限**

   - 首次运行需授予 **辅助功能** 权限（用于自动化操作）
   - 系统设置 → 隐私与安全性 → 辅助功能 → 添加 Orchis

4. **配置 IMAP**

   - 进入 Automation 标签页
   - 点击 "Manage Profiles" 添加邮箱配置
   - 或直接在下方填写 IMAP 服务器信息

5. **开始使用**
   - 填写目标邮箱和密码
   - 点击 "Start Job" 开始自动注册

---

## 开发指南

### 环境准备

```bash
# 安装 Node.js 依赖
npm install

# 安装 Rust (如果未安装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 开发模式

```bash
# 启动开发服务器
npm run tauri dev
```

### 构建发布

```bash
# 构建 Release 版本
npm run tauri build

# 输出位置
# - DMG: src-tauri/target/release/bundle/dmg/
# - App: src-tauri/target/release/bundle/macos/
```

### 项目结构

```
orchids-manager/
├── src/                    # Vue 前端源码
│   ├── App.vue            # 主应用组件
│   └── assets/            # 静态资源
├── src-tauri/             # Rust 后端源码
│   ├── src/
│   │   ├── lib.rs         # 核心逻辑 (注册、IMAP、自动化)
│   │   ├── clerk_utils.rs # Clerk API 工具
│   │   ├── imap_service.rs# IMAP 邮件服务
│   │   └── gmail_oauth.rs # Gmail OAuth 实现
│   ├── icons/             # 应用图标
│   └── tauri.conf.json    # Tauri 配置
└── package.json           # Node.js 配置
```

---

## 工作原理

### 自动注册流程

```
1. [Start Job]
   ↓
2. 重置本地 Orchids 客户端 (killall + 清除数据)
   ↓
3. 启动 Orchids 客户端
   ↓
4. AppleScript 自动点击 "Log in" 按钮
   ↓
5. 创建隐身 WebView 打开注册页面
   ↓
6. 自动填写邮箱和密码
   ↓
7. IMAP 轮询获取验证码
   ↓
8. 验证码稳定后自动填入 + 系统粘贴
   ↓
9. 捕获登录 Session
   ↓
10. 自动重定向到 Auth 页面
    ↓
11. 提取 Deep Link (orchids://) 并系统打开
    ↓
12. Orchids 客户端接收回调，完成登录 ✓
```

---

## 常见问题

### Q: 验证码填入失败？

确保已授予辅助功能权限，并且验证码邮件能正常接收。

### Q: Orchids 客户端没有自动唤起？

检查 Orchids 客户端是否正确安装，以及是否授予了必要权限。

### Q: IMAP 连接失败？

- 确认 IMAP 服务器地址和端口正确
- 检查用户名密码是否正确
- 部分邮箱需要生成应用专用密码

---

## 许可证

MIT License © 2025

---

<p align="center">
  Made with ❤️ for Orchids Community
</p>
