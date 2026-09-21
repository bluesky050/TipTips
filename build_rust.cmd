@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
set PATH=%PATH%;C:\Users\25735\.cargo\bin
cd /d D:\TipTips\src-tauri
cargo build 2>&1
