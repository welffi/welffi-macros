@echo off
set PATH=%USERPROFILE%\.cargo\bin;%PATH%
cd /d "%~dp0"
cargo build --release || exit /b 1
if not exist dist mkdir dist
copy /y "target\release\welffi_macros.exe" "dist\welffi macros.exe" >nul || (echo Close welffi macros first, then run this again & exit /b 1)
echo Built dist\welffi macros.exe
