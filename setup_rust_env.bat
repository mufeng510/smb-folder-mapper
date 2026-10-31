@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" x64
set PATH=%PATH%;C:\Users\mi\.cargo\bin;C:\Users\mi\tools\node-v20.19.0-win-x64
echo MSVC x64 + Rust + Node 环境已就绪
echo %PATH%
