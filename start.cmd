@echo off
chcp 65001 >nul
cd /d "%~dp0"
setlocal

set "APP=dist\EmptyFolderCleaner-win-x64\EmptyFolderCleaner.exe"
set "DEBUG=target\debug\EmptyFolderCleaner.exe"

if exist "%APP%" (
  "%APP%" %*
  if errorlevel 1 pause
  endlocal
  exit /b 0
)

if exist "%DEBUG%" (
  "%DEBUG%" %*
  if errorlevel 1 pause
  endlocal
  exit /b 0
)

echo.
echo 尚未找到可运行的程序，请先执行 build.cmd。
echo.
pause
endlocal
exit /b 1
