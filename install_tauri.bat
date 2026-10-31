@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" x64
set PATH=%PATH%;C:\Users\mi\.cargo\bin;C:\Users\mi\tools\node-v20.19.0-win-x64
echo ========================================
echo 环境就绪：MSVC x64 + Rust + Node.js
echo ========================================
cargo install tauri-cli --version 2.0.0 --force
