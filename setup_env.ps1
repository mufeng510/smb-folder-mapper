<#
.SYNOPSIS
    smb-folder-mapper 一键开发环境搭建脚本（Windows 10 / 11）
    请以【管理员身份】运行 PowerShell 执行本脚本。
.DESCRIPTION
    依次安装：
      1. Visual Studio Build Tools（MSVC C++ 编译工具链）
      2. Rust (rustup) 及环境变量
      3. Node.js LTS + npm
      4. Tauri CLI (cargo tauri)
#>
$ErrorActionPreference = 'Stop'

Write-Host "========================================" -ForegroundColor Cyan
Write-Host " smb-folder-mapper 开发环境搭建向导    " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# ---------- 检查管理员权限 ----------
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Warning ">>> 当前不是管理员身份，创建目录符号链接和安装 Build Tools 可能会失败。"
    Write-Warning ">>> 建议右键 PowerShell -> “以管理员身份运行”后再次执行本脚本。`n"
}

# ---------- 0. 确认 winget 可用 ----------
Write-Host "[0/5] 检查 winget ..." -ForegroundColor Cyan
if (Get-Command winget -ErrorAction SilentlyContinue) {
    Write-Host "    winget 已可用。" -ForegroundColor Green
} else {
    Write-Warning "    winget 不可用（未安装应用安装程序）。请先从 Microsoft Store 安装“应用安装程序”，"
    Write-Warning "    或手动下载各工具离线安装包。下面各步骤若 winget 失败会给出手动下载链接。`n"
}

# ---------- 1. C++ Build Tools (MSVC) ----------
Write-Host "[1/5] 安装 Visual Studio Build Tools（MSVC C++ 工具链）..." -ForegroundColor Cyan
if (Get-Command winget -ErrorAction SilentlyContinue) {
    winget install --id Microsoft.VisualStudio.2022.BuildTools `
        --accept-package-agreements --accept-source-agreements `
        --override "--add Microsoft.VisualStudio.Workload.VC.Tools --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --locale en-US" `
        --silent `
        2>$null
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "    winget 安装 Build Tools 失败，请手动下载安装："
        Write-Host "    https://visualstudio.microsoft.com/zh-hans/downloads/#build-tools-for-visual-studio-2022"
        Write-Host "    安装时请勾选工作负载：使用 C++ 的桌面开发"
    } else {
        Write-Host "    Build Tools 安装完成。" -ForegroundColor Green
    }
} else {
    Write-Warning "    跳过（winget 不可用）。请手动安装 Build Tools。"
}

# ---------- 2. Rust (rustup) ----------
Write-Host "[2/5] 安装 Rust (rustup) ..." -ForegroundColor Cyan
if (Get-Command winget -ErrorAction SilentlyContinue) {
    winget install --id Rustlang.Rustup --accept-package-agreements --accept-source-agreements --silent 2>$null
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "    winget 安装 Rustup 失败，请手动下载：https://rustup.rs/ (运行 rustup-init.exe)"
    } else {
        Write-Host "    Rustup 安装完成。" -ForegroundColor Green
    }
} else {
    Write-Warning "    跳过（winget 不可用）。"
}

# 将当前会话的 PATH 指向 cargo/bin（rustup 安装后通常在 %USERPROFILE%\.cargo\bin）
$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (Test-Path $cargoBin) {
    $env:PATH = "$cargoBin;$env:PATH"
}
if (Test-Path "$env:USERPROFILE\.cargo\env") {
    & "$env:USERPROFILE\.cargo\env"
}

# ---------- 3. Node.js LTS ----------
Write-Host "[3/5] 安装 Node.js LTS + npm ..." -ForegroundColor Cyan
if (Get-Command winget -ErrorAction SilentlyContinue) {
    winget install --id OpenJS.NodeJS.LTS --accept-package-agreements --accept-source-agreements --silent 2>$null
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "    winget 安装 Node.js 失败，请手动下载 LTS 版：https://nodejs.org/"
    } else {
        Write-Host "    Node.js 安装完成。" -ForegroundColor Green
    }
} else {
    Write-Warning "    跳过（winget 不可用）。"
}
$nodeDir = "$env:PROGRAMFILES\nodejs"
if (Test-Path $nodeDir) { $env:PATH = "$nodeDir;$env:PATH" }

# ---------- 4. Tauri CLI ----------
Write-Host "[4/5] 安装 Tauri CLI (cargo tauri) ..." -ForegroundColor Cyan
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    cargo install tauri-cli --version 2 --force 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "    tauri-cli 安装完成。" -ForegroundColor Green
    } else {
        Write-Warning "    tauri-cli 安装失败（可跳过，后续使用 cargo tauri 亦可）。"
    }
} else {
    Write-Warning "    跳过（cargo 未就绪，请先完成 Rust 安装并重启终端）。"
}

# ---------- 5. 版本校验 ----------
Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host " 环境版本校验                            " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
rustc --version 2>$null
cargo --version 2>$null
node --version 2>$null
npm --version 2>$null
tauri --version 2>$null

Write-Host "`n>>> 搭建完毕！请【重新打开】PowerShell/终端 使环境变量（PATH）生效。" -ForegroundColor Green
Write-Host ">>> 接下来进入项目目录并执行 Tauri 初始化命令（见 PROJECT_SETUP.md 或下文说明）。" -ForegroundColor Green
