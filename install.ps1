# Easy install for llama.rs (Windows)
# Run from the repo root: .\install.ps1
# Prerequisites: Rust (rustup), VS Build Tools with "Desktop development with C++" and "C++ Clang tools for Windows"

$ErrorActionPreference = "Stop"

# Find Clang bin (libclang.dll)
$clangPaths = @(
    "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\Llvm\x64\bin",
    "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\Llvm\x64\bin",
    "C:\Program Files (x86)\Microsoft Visual Studio\2022\Enterprise\VC\Tools\Llvm\x64\bin",
    "C:\Program Files\LLVM\bin"
)
$libclangPath = $null
foreach ($p in $clangPaths) {
    if (Test-Path "$p\libclang.dll") {
        $libclangPath = $p
        break
    }
}
if (-not $libclangPath) {
    Write-Host "ERROR: libclang.dll not found. Install 'C++ Clang tools for Windows' in Visual Studio Installer."
    Write-Host "See docs/DEVELOPMENT.md for details."
    exit 1
}
$env:LIBCLANG_PATH = $libclangPath
Write-Host "Using LIBCLANG_PATH=$libclangPath"

# Find VsDevCmd (MSVC)
$vsPaths = @(
    "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\Tools\VsDevCmd.bat",
    "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat",
    "C:\Program Files (x86)\Microsoft Visual Studio\2022\Enterprise\Common7\Tools\VsDevCmd.bat"
)
$vsDevCmd = $null
foreach ($p in $vsPaths) {
    if (Test-Path $p) {
        $vsDevCmd = $p
        break
    }
}
if (-not $vsDevCmd) {
    Write-Host "ERROR: VsDevCmd.bat not found. Install 'Desktop development with C++' in Visual Studio Installer."
    exit 1
}

$projectRoot = $PSScriptRoot
Write-Host "Building in $projectRoot ..."
& cmd /c "`"$vsDevCmd`" -arch=amd64 && cd /d `"$projectRoot`" && cargo build --release"
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed."
    exit 1
}
Write-Host ""
Write-Host "Done. Run: .\target\release\llama_rs.exe"
Write-Host "With a model: .\target\release\llama_rs.exe path\to\model.gguf `"Your prompt`""
