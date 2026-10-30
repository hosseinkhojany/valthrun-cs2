@echo off
REM =====================================================================
REM  VALTHRUN ENHANCED - controller.exe ONE-CLICK BUILDER (Windows)
REM =====================================================================
REM  What this script does, step by step:
REM
REM   1. Move into the MASTER folder (the folder this .bat sits in).
REM   2. CHECK every requirement before doing anything:
REM        - Git for Windows      (fetches the valthrun-driver dependency)
REM        - Visual Studio Build Tools + MSVC C++ toolkit + Windows SDK
REM                                 (C/C++ we need to link the binary)
REM        - Rust + rustup        (stable toolchain)
REM        - Rust nightly         (the project needs nightly features)
REM   3. INSTALL ONLY the requirement(s) that are missing:
REM        - uses "winget" when available, otherwise falls back to
REM          downloading the official installer directly.
REM        - each step is re-checked after the install, so the script
REM          is idempotent - rerun as often as you want. Never repeats
REM          an install that is already done.
REM   4. Compile the RELEASE build:
REM        cargo +nightly build --release -p controller
REM   5. Copy target\release\controller.exe -> master folder next to
REM      this script, so controller.exe appears where everyone expects.
REM
REM  If ALL requirements are already met it skips the downloads and
REM  goes straight to "compile + copy".
REM =====================================================================

REM --- Freeze the current working directory as the master folder -----------
cd /d "%~dp0"

REM --- Sanity check: we must be inside the repository (has Cargo.toml) -----
if not exist "%~dp0Cargo.toml" goto :not_in_master

REM --- Small beautifier: nice section separator ---------------------------
echo.
echo =====================================================================
echo   Valthrun Enhanced - controller.exe Auto-Builder
echo =====================================================================

REM --- Step 1: check / install Git --------------------------------------
call :check_git

REM --- Step 2: check / install MSVC Build Tools -------------------------
call :check_vs

REM --- Step 3: check / install Rust (stable) -----------------------------
call :check_rust

REM --- Step 4: check / install Rust nightly ------------------------------
call :check_nightly

REM --- Step 5: build the release controller and copy it here -------------
call :build_controller

REM --- Done -------------------------------------------------------------
goto :finish


REM ============================================================
REM  SUBROUTINE : check_git  (call :check_git)
REM  -> verifies "git" is available; installs it if missing.
REM ============================================================
:check_git
    where git >nul 2>nul
    if not errorlevel 1 (
        echo [ OK ] Git for Windows - already installed
        exit /b 0
    )
    echo [INFO] Git is missing - installing Git for Windows...
    where winget >nul 2>nul
    if not errorlevel 1 goto :git_via_winget
    goto :git_via_download

    REM git install via the Windows package manager (winget)
    :git_via_winget
        REM one line installs the official Git for Windows package silently
        winget install --id Git.Git --exact --silent --accept-package-agreements --accept-source-agreements
        if not errorlevel 1 goto :git_after_recheck
        echo [WARN] winget failed - falling back to direct download:
        goto :git_via_download

    :git_via_download
    REM download the official 64bit installer straight from GitHub releases
    REM (you can update the version number in this URL later if needed)
    powershell -NoProfile -ExecutionPolicy Bypass -Command "$ProgressPreference='SilentlyContinue'; Invoke-WebRequest -Uri 'https://github.com/git-for-windows/git/releases/download/v2.47.1.windows.1/Git-2.47.1-64-bit.exe' -OutFile '%TEMP%\Git-2.47.1-64-bit.exe'"
    if errorlevel 1 goto :fail_git
    REM run SilentGuard: quit = no message boxes, NOrestart = do not reboot
    "%TEMP%\Git-2.47.1-64-bit.exe" /VERYSILENT /NORESTART /NOCANCEL /SP-
    if errorlevel 1 goto :fail_git

    :git_after_recheck
    REM Git registers itself in the Machine PATH: add the default location
    REM to the current command window so 'where git' works right away
    set "PATH=%PATH%;C:\Program Files\Git\cmd"
    where git >nul 2>nul
    if errorlevel 1 goto :fail_git
    echo [OK] Git for Windows - installed successfully
    exit /b 0


REM ============================================================
REM  SUBROUTINE : check_vs  (call :check_vs)
REM  -> verifies MSVC linker + Windows SDK are present;
REM     if not, installs VS 2022 Build Tools with the C++ workload.
REM ============================================================
:check_vs
    set "VSDIR="
    if exist "%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" (
        for /f "usebackq delims=" %%i in (`"%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSDIR=%%i"
    )
    if defined VSDIR (
        echo [ OK ] MSVC C++ Build Tools - found at:%VSDIR%
        exit /b 0
    )
    echo [INFO] Visual Studio Build Tools \ MSVC - MISSING, one-time installation starts...
    where winget >nul 2>nul
    if not errorlevel 1 goto :vs_via_winget
    goto :vs_via_download

    :vs_via_winget
    REM --override hands our own switches to the Visual Studio bootstrapper:
    REM --quiet      no GUI, --wait  wait until finished,
    REM --add        install the C++ workload (+recommended tools = MSVC + SDK)
    echo [    ] Downloading/installing VS 2022 Build Tools (takes ~5-15 min)...
    winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --silent --accept-package-agreements --accept-source-agreements --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
    if not errorlevel 1 goto :vs_recheck
    echo [WARN] winget failed - trying the official bootstrapper directly:
    goto :vs_via_download

    :vs_via_download
    REM download the official bootstrapper from the stable MS "aka.ms" link
    powershell -NoProfile -ExecutionPolicy Bypass -Command "$ProgressPreference='SilentlyContinue'; Invoke-WebRequest -Uri 'https://aka.ms/vs/17/release/vs_BuildTools.exe' -OutFile '%TEMP%\vs_BuildTools.exe'"
    if errorlevel 1 goto :fail_vs
    "%TEMP%\vs_BuildTools.exe" --quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended
    if errorlevel 1 goto :fail_vs

    :vs_recheck
    REM re-query vswhere after install; errorlevel 1 means still missing
    set "VSDIR="
    for /f "usebackq delims=" %%i in (`"%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSDIR=%%i"
    if not defined VSDIR goto :fail_vs
    echo [ OK ] MSVC Build Tools - installed successfully
    exit /b 0


REM ============================================================
REM  SUBROUTINE : check_rust  (call :check_rust)
REM  -> verifies a Rust toolchain managed by rustup exists.
REM ============================================================
:check_rust
    where rustup >nul 2>nul
    if not errorlevel 1 goto :check_rust_recheck

    echo [INFO] Rust - missing, downloading rustup-init.exe (official)...
    REM download rustup (the official Rust installer) from win.rustup.rs
    REM (this tiny URL redirects to the latest official rustup-init.exe)
    powershell -NoProfile -ExecutionPolicy Bypass -Command "$ProgressPreference='SilentlyContinue'; Invoke-WebRequest -Uri 'https://win.rustup.rs/x86_64' -OutFile '%TEMP%\rustup-init.exe'"
    if errorlevel 1 goto :fail_rust
    REM -y    accept all defaults without prompts
    REM --default-host  x86_64-pc-windows-msvc builds with MSVC toolset
    REM --default-toolchain stable  installs the stable compiler
    REM --profile minimal           only compiler + std (small download)
    "%TEMP%\rustup-init.exe" -y --default-host x86_64-pc-windows-msvc --default-toolchain stable --profile minimal
    if errorlevel 1 goto :fail_rust
    REM rustup registers itself in the User PATH; prepend it to the
    REM current window so 'cargo' is callable right now
    set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

    :check_rust_recheck
    REM verify the exact stable MSVC toolchain is installed
    rustup toolchain list 2>nul | findstr /i /c:"stable-x86_64-pc-windows-msvc" >nul
    if not errorlevel 1 (
        echo [ OK ] Rust stable-x86_64-pc-windows-msvc - installed
        exit /b 0
    )
    echo [INFO] Creating the stable MSVC toolchain (one-time download)...
    rustup toolchain install stable-x86_64-pc-windows-msvc --profile minimal
    if errorlevel 1 goto :fail_rust
    echo [ OK ] Rust stable-x86_64-pc-windows-msvc - installed
    exit /b 0


REM ============================================================
REM  SUBROUTINE : check_nightly  (call :check_nightly)
REM  -> the project needs nightly; installs it if absent.
REM ============================================================
:check_nightly
    REM look for any installed nightly in rustup's toolchain list
    rustup toolchain list 2>nul | findstr /i /c:"nightly" >nul
    if not errorlevel 1 (
        echo [ OK ] Rust nightly - installed
        exit /b 0
    )
    echo [INFO] Rust nightly - installing (one-time download, ~300 MB)...
    REM --profile minimal keeps the download and disk use small
    rustup toolchain install nightly --profile minimal
    if errorlevel 1 goto :fail_nightly
    echo [ OK ] Rust nightly - installed
    exit /b 0


REM ============================================================
REM  SUBROUTINE : build_controller  (call :build_controller)
REM  -> compiles the release binary and copies it to the master folder.
REM ============================================================
:build_controller
    REM let "rustc" find cl.exe / link.exe through the vcvars environment
    REM (vswhere gave us the install directory in %VSDIR% earlier)
    if defined VSDIR call :load_vcvars
    echo.
    echo -------------------------------------------------------------
    echo  Building release controller.exe (first build can take 10-40 min)
    echo -------------------------------------------------------------
    REM this is the exact build command from the official README
    cargo +nightly build --release -p controller
    if errorlevel 1 goto :fail_build
    REM the compiled binary always lands in target\release\
    if not exist "%~dp0target\release\controller.exe" goto :fail_build
    REM copy the freshly built exe from target\release to the master folder
    copy /y "%~dp0target\release\controller.exe" "%~dp0controller.exe" >nul
    if errorlevel 1 goto :fail_copy
    echo.
    echo [ OK ] controller.exe copied into the master folder:
    echo        %~dp0controller.exe
    echo        (source binary: %~dp0target\release\controller.exe)
    exit /b 0


REM ============================================================
REM  SUBROUTINE : load_vcvars  (call :load_vcvars)
REM  -> pushes the Visual Studio compiler/linker environment vars
REM     into this command window so the Rust build succeeds.
REM ============================================================
:load_vcvars
    if not defined VSDIR exit /b 0
    if exist "%VSDIR%\VC\Auxiliary\Build\vcvarsall.bat" (
        echo [INFO] Loading MSVC environment from vcvarsall.bat ...
        call "%VSDIR%\VC\Auxiliary\Build\vcvarsall.bat" x64 >nul 2>nul
    )
    exit /b 0


REM ============================================================
REM  ERROR HANDLING LABELS
REM ============================================================
:not_in_master
echo [ERROR] This script must live inside the valthrun-enhanced-master folder.
echo         Please move build_controller.bat into the repository root.
pause
exit /b 1

:fail_git
echo.
echo [ERROR] Git could not be installed automatically.
echo         Install it manually from https://git-scm.com then reopen this script.
pause
exit /b 1

:fail_vs
echo.
echo [ERROR] Visual Studio Build Tools could not be installed automatically.
echo         Install manually: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
echo         (check the "Desktop development with C++" workload)
pause
exit /b 1

:fail_rust
echo.
echo [ERROR] Rust could not be installed automatically.
echo         Install it manually from https://rustup.rs and run this script again.
pause
exit /b 1

:fail_nightly
echo.
echo [ERROR] The Rust nightly toolchain could not be installed.
echo         Try manually:  rustup toolchain install nightly
pause
exit /b 1

:fail_build
echo.
echo [ERROR] The cargo build failed - check the error messages above
echo         (usually a missing component: rerun the script, or run
echo          "rustup component add rust-src --toolchain nightly").
pause
exit /b 1

:fail_copy
echo.
echo [ERROR] Build succeeded but controller.exe could not be copied.
echo         Copy target\release\controller.exe to the master folder manually.
pause
exit /b 1


REM ============================================================
REM  SUCCESS SCREEN
REM ============================================================
:finish
echo.
echo =====================================================================
echo  DONE - controller.exe is ready in the master folder:
echo    %~dp0controller.exe
echo.
echo  If everything was already installed the build was all that ran;
echo  if tools were missing they were installed and the build ran after.
echo.
echo  (put controller.exe in the Valthrun Loader folder and launch it)
echo =====================================================================
pause