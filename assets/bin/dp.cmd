@echo off
REM DeathPush CLI launcher (Windows)
REM Resolves the app installation and opens DeathPush with a directory argument.

setlocal enabledelayedexpansion

set "SCRIPT_DIR=%~dp0"

REM Navigate from resources\bin\ up to the app root
set "APP_DIR=%SCRIPT_DIR%..\.."
set "BINARY=%APP_DIR%\deathpush.exe"

if not exist "%BINARY%" (
  echo Error: DeathPush binary not found at %BINARY% >&2
  echo Please reinstall the CLI tool from DeathPush ^> Install Command Line Tool... >&2
  exit /b 1
)

set "TARGET=%~1"
if "%TARGET%"=="" set "TARGET=."

REM Convert relative paths to absolute
pushd "%TARGET%" 2>nul
if errorlevel 1 (
  echo Error: Directory '%~1' does not exist >&2
  exit /b 1
)
set "TARGET=%CD%"
popd

"%BINARY%" "%TARGET%"
