param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$CargoArgs = @("check")
)

$ErrorActionPreference = "Stop"

$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (Test-Path $cargoBin) {
    $env:PATH = "$cargoBin;$env:PATH"
}

$targetDir = Join-Path $env:LOCALAPPDATA "Jarvis\target"
New-Item -ItemType Directory -Force -Path $targetDir | Out-Null
$env:CARGO_TARGET_DIR = $targetDir

$vsRoot = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools"
$vcvars = Join-Path $vsRoot "VC\Auxiliary\Build\vcvars64.bat"
if (-not (Test-Path $vcvars)) {
    throw "Visual Studio Build Tools vcvars64.bat not found at $vcvars"
}

$msvcRoot = Join-Path $vsRoot "VC\Tools\MSVC"
$msvcVersion = Get-ChildItem $msvcRoot -Directory |
    Sort-Object Name -Descending |
    Select-Object -First 1
if (-not $msvcVersion) {
    throw "MSVC tools were not found under $msvcRoot"
}

$sdkLibRoot = "C:\Program Files (x86)\Windows Kits\10\Lib"
$sdkVersion = Get-ChildItem $sdkLibRoot -Directory |
    Sort-Object Name -Descending |
    Select-Object -First 1
if (-not $sdkVersion) {
    throw "Windows SDK libs were not found under $sdkLibRoot"
}

$msvcLib = Join-Path $msvcVersion.FullName "lib\x64"
$sdkUmLib = Join-Path $sdkVersion.FullName "um\x64"
$sdkUcrtLib = Join-Path $sdkVersion.FullName "ucrt\x64"

foreach ($path in @($msvcLib, $sdkUmLib, $sdkUcrtLib)) {
    if (-not (Test-Path $path)) {
        throw "Required native library path is missing: $path"
    }
}

function Get-ShortPath([string]$Path) {
    $escaped = $Path.Replace('"', '""')
    $result = cmd.exe /C "for %I in (`"$escaped`") do @echo %~sI"
    if (-not $result) {
        return $Path
    }
    return $result.Trim()
}

$rustFlags = @(
    "-Lnative=$(Get-ShortPath $msvcLib)",
    "-Lnative=$(Get-ShortPath $sdkUmLib)",
    "-Lnative=$(Get-ShortPath $sdkUcrtLib)"
) -join " "

$env:RUSTFLAGS = $rustFlags

$cargoCommand = if ($CargoArgs.Count -eq 0) { "check" } else { $CargoArgs -join " " }
$cmd = "set `"PATH=$cargoBin;%PATH%`" && set `"CARGO_TARGET_DIR=$targetDir`" && `"$vcvars`" >nul && set `"RUSTFLAGS=$rustFlags`" && cargo $cargoCommand"
cmd.exe /S /C $cmd
