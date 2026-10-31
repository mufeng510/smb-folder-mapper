# smb-folder-mapper

极轻量 Windows 图形工具：将局域网内的 SMB 共享文件夹（UNC 路径，如 `\\192.168.1.100\shared`）以**目录符号链接**（Directory Symbolic Link）形式挂载到本地空文件夹，而不是映射为网络盘符。挂载完成后关闭程序即可，链接持久化，无后台常驻进程。

- 前端：原生 HTML / CSS / JavaScript（无构建工具）
- 后端：Rust + Tauri v2
- 平台：Windows 10 / 11（需管理员权限）

---

## 项目结构

```
smb-folder-mapper/
├── setup_env.ps1              # 一键环境搭建 PowerShell 脚本
├── package.json               # Node 元数据
├── README.md
├── src/                       # 前端（极致轻量，无构建步骤）
│   ├── index.html
│   └── main.js
├── src-tauri/                 # 后端 Rust（Tauri v2）
│   ├── Cargo.toml
│   ├── src/main.rs            # mount / unmount / 凭据管理器 命令
│   ├── tauri.conf.json        # v2 配置、权限、插件
│   ├── app.manifest           # requireAdministrator UAC 清单
│   └── capabilities/default.json
└── .github/workflows/release.yml   # GitHub Actions：打包单独立alone .exe
```

---

## 第一阶段：开发环境搭建（在干净 Windows 上执行）

以**管理员身份**打开 PowerShell，运行：

```powershell
# 1. 进入工作区
cd C:\workspace\smb-folder-mapper

# 2. 执行环境搭建脚本（安装 Build Tools / Rust / Node.js / tauri-cli）
.\setup_env.ps1
```

脚本会依次安装：
1. **Visual Studio Build Tools**（MSVC C++ 编译工具链，勾选“使用 C++ 的桌面开发”工作负载）
2. **Rust (rustup)** 及环境变量（`%USERPROFILE%\.cargo\bin`）
3. **Node.js LTS** + npm（`%PROGRAMFILES%\nodejs`）
4. **Tauri CLI**（`cargo install tauri-cli --version 2`）

> 若本机 `winget` 不可用，脚本会给出手动下载链接。安装完成后**请重新打开 PowerShell / 终端** 使 `PATH` 环境变量生效。

校验安装：

```powershell
rustc --version
cargo --version
node --version
npm --version
```

---

### 初始化 Tauri v2 项目

本仓库已包含完整的 `src-tauri/` 与前端文件，无需再运行 `tauri init`。若你需要从零再创建一次，可执行：

```powershell
cargo tauri init
```

> `cargo tauri` 来自 `tauri-cli`；也可直接使用本仓库现有的 `src-tauri/` 文件，跳过初始化。

---

## 第二阶段：开发与运行

### 启动开发模式

```powershell
cd C:\workspace\smb-folder-mapper
cargo tauri dev
```

> 前端使用 `@tauri-apps/api`（通过 unpkg CDN 加载），开发时需联网。如需完全离线生产，请将 `src/index.html` 中的 `importmap` unpkg 地址替换为本地 vendored 的 `@tauri-apps/api/dist` 文件并相应调整。

### 打包发行版（本地）

```powershell
cargo tauri build
```

产物位于 `src-tauri/target/release/smb-folder-mapper.exe`（单一独立可执行文件，前端已嵌入），并会生成 `.msi` 安装包。

---

## 功能说明

| 模块 | 说明 |
|------|------|
| 本地路径 | 文本框 + “浏览”按钮，调用系统原生文件夹选择对话框（Tauri v2 `tauri-plugin-dialog`） |
| 远程 SMB 路径 | 输入 UNC 路径，如 `\\192.168.1.100\shared` |
| 凭据管理 | 用户名/密码，勾选后写入 **Windows 凭据管理器**，重启后系统自动鉴权 |
| 挂载 | 执行 `cmd /c mklink /d <本地> <UNC>`，需管理员权限 |
| 卸载 | `std::fs::remove_dir` 仅删除本地符号链接，不删除远程共享 |
| 状态栏 | 操作成功/失败提示 |

---

## 关键设计点（避坑）

1. **管理员权限**：`src-tauri/app.manifest` 中 `requestedExecutionLevel level="requireAdministrator"`，启动时自动弹出 UAC 提升。目录符号链接的创建必须在管理员权限下进行。
2. **mklink 内置命令限制**：Rust 侧严禁直接 `Command::new("mklink")`，代码中统一使用 `Command::new("cmd").args(["/c", "mklink", "/d", link, target])`。
3. **Tauri v2 规范**：本地文件夹选择使用官方 `tauri-plugin-dialog`，并在 `tauri.conf.json` 的 `plugins.dialog` 与 `capabilities/default.json` 中放行权限。
4. **零后台残留**：符号链接由系统持久化于文件系统，关闭 Tauri 窗口后整个进程彻底退出，不留任何后台服务。

---

## GitHub Actions 自动打包

推送 tag（如 `v0.1.0`）后，`.github/workflows/release.yml` 会在 `windows-latest` 上编译并：

- 上传 `src-tauri/target/release/smb-folder-mapper.exe` 作为 Release 附件（单一独立 `.exe`）
- 创建 GitHub Release

本地等价命令：

```powershell
cargo tauri build --verbose
```
