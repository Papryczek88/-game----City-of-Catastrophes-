@echo off
setlocal
cd /d "%~dp0"
echo Starting City of Catastrophes...
where cargo >nul 2>nul
if errorlevel 1 (
  echo Rust/Cargo is not installed or is not in PATH.
  echo Install Rust from https://rustup.rs/ and try again.
  pause
  exit /b 1
)
cargo run --release
if errorlevel 1 (
  echo.
  echo The game could not start. Check the error above.
  pause
  exit /b 1
)
endlocal
