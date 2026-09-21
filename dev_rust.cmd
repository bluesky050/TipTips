@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
set PATH=%PATH%;C:\Users\25735\.cargo\bin;D:\TipTips\node_modules\.bin
cd /d D:\TipTips
npm run tauri dev 2>&1
