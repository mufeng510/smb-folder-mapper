@echo off
:: 初始化 MSVC x64 环境
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" x64
:: 恢复 Rust 和 Node.js 到 PATH
set PATH=%PATH%;C:\Users\mi\.cargo\bin;C:\Users\mi\tools\node-v20.19.0-win-x64
echo ========================================
echo Build environment ready (MSVC x64 + Rust + Node)
echo ========================================
cmd /k
