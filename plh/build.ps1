# build.ps1
# chcp 65001
param(
    [int]$Version = 1
)

# 1. 读取 JSON 配置 (原生支持，无需安装模块)
$ConfigPath = Join-Path $PSScriptRoot "build.json"
if (-not (Test-Path $ConfigPath)) {
    Write-Host "[ERROR] 找不到配置文件: $ConfigPath" -ForegroundColor Red
    
}

# 读取文件内容并转换为 JSON 对象
$JsonContent = Get-Content -Path $ConfigPath -Raw -Encoding UTF8
$Config = $JsonContent | ConvertFrom-Json

# 2. 提取配置节
$Paths = $Config.paths
$Tools = $Config.tools
$Targets = $Config.targets

# 3. 设置全局错误中断
$ErrorActionPreference = "Stop"

# 4. 辅助函数
function Safe-Move($Source, $Destination) {
    $DestDir = Split-Path -Path $Destination -Parent
    if (-not (Test-Path $DestDir)) {
        New-Item -ItemType Directory -Path $DestDir -Force | Out-Null
    }
    Write-Host "[MOVE] $Source -> $Destination" -ForegroundColor Cyan
    Move-Item -Path $Source -Destination $Destination -Force
}

function Invoke-BuildStep($Name, $ScriptBlock) {
    Write-Host "`n>> 开始: $Name" -ForegroundColor Yellow
    try {
        & $ScriptBlock
        Write-Host ">> 完成: $Name" -ForegroundColor Green
    } catch {
        Write-Host "[FAILED] $Name : $_" -ForegroundColor Red
        
    }
}

# 5. 核心编译逻辑
Set-Location $Paths.root

# --- Rust 部分 ---
if ($Targets.rust_windows) {
    Invoke-BuildStep "Rust (Windows) Quern Launcher & Qvm" { 
        Move-Item "$($Paths.root)\buildtargets\1\main.rs" "$($Paths.root)\src\main.rs" -Force
        cargo build --release 
        Safe-Move "$($Paths.root)\target\release\Quern_cargo.exe" "$($Paths.template)\Windows\Qvm.exe"

        Move-Item "$($Paths.root)\buildtargets\2\main.rs" "$($Paths.root)\src\main.rs" -Force
        cargo build --release 
        Safe-Move "$($Paths.root)\target\release\Quern_cargo.exe" "$($Paths.template)\Windows\Quern.exe"
    }
}


if ($Targets.rust_linux) {
    Invoke-BuildStep "Rust (Linux) Quern Launcher & Qvm" { 
        Move-Item "$($Paths.root)\buildtargets\1\main.rs" "$($Paths.root)\src\main.rs" -Force
        wsl cargo build --release 
        Safe-Move "$($Paths.root)\target\release\Quern_cargo.exe" "$($Paths.template)\Windows\Qvm.exe"

        Move-Item "$($Paths.root)\buildtargets\2\main.rs" "$($Paths.root)\src\main.rs" -Force
        wsl cargo build --release 
        Safe-Move "$($Paths.root)\target\release\Quern_cargo.exe" "$($Paths.template)\Windows\Quern.exe"
    }
}

# --- C++ AOT 部分 ---
if ($Targets.cpp_windows) {
    Invoke-BuildStep "C++ AOT (Windows)" {
        clang++ src\tnstr\aot.cpp -o QuernBuild.exe -std=c++17 -static -O3 -s -Wall -Wextra -Wno-unused-parameter -Wno-unused-variable -Wno-unused-function
        Safe-Move "$($Paths.root)\src\QuernBuild.exe" "$($Paths.template)\Windows\QuernBuild.exe"
    }
}

if ($Targets.cpp_linux) {
    Invoke-BuildStep "C++ AOT (Linux)" {
        wsl g++ /mnt/i/Quern/QuernProj/Quern/src/tnstr/aot.cpp -o QuernBuild -std=c++17 -static -O3 -Wall -Wextra -Wno-unused-parameter -Wno-unused-variable -Wno-unused-function -Wno-unused-but-set-variable -g1 -fno-omit-frame-pointer
        Safe-Move "$($Paths.root)\src\QuernBuild" "$($Paths.template)\Linux\QuernBuild"
    }
}

# --- Go 翻译机部分 ---
$GoBin = $Tools.go
if ($Targets.go_windows) {
    Invoke-BuildStep "Go (Windows)" {
        $env:CGO_ENABLED="0"; $env:GOOS="windows"; $env:GOARCH="amd64"
        & $GoBin build -trimpath -ldflags="-s -w -buildmode=exe" -o Quernc.exe translator.go
        # 注意：这里假设你的 UPX 逻辑，如果不需要可以删掉下面这行
        # & $Tools.seven_zip a "$($Paths.template)\Windows\Quernc.exe.upx" "$($Paths.template)\Windows\Quernc.exe" 
        Safe-Move "Quernc.exe" "$($Paths.template)\Windows\Quernc.exe"
    }
}

if ($Targets.go_linux) {
    Invoke-BuildStep "Go (Linux)" {
        $env:CGO_ENABLED="0"; $env:GOOS="linux"; $env:GOARCH="amd64"
        & $GoBin build -trimpath -ldflags="-s -w" -o Quernc-linux translator.go
        Safe-Move "Quernc-linux" "$($Paths.template)\Linux\Quernc"
    }
}

if ($Targets.go_macos_arm64) {
    Invoke-BuildStep "Go (macOS ARM64)" {
        $env:CGO_ENABLED="0"; $env:GOOS="darwin"; $env:GOARCH="arm64"
        & $GoBin build -trimpath -ldflags="-s -w" -o Quernc-macOS-arm64 translator.go
        Safe-Move "Quernc-macOS-arm64" "$($Paths.template)\macOS\arm64\Quernc"
    }
}

# if ($Targets.go_macos_amd64) {
#     Invoke-BuildStep "Go (macOS AMD64)" {
#         $env:CGO_ENABLED="0"; $env:GOOS="darwin"; $env:GOARCH="amd64"
#         & $GoBin build -trimpath -ldflags="-s -w" -o Quernc-macOS-amd64 translator.go
#         Safe-Move "Quernc-macOS-amd64" "$($Paths.template)\macOS\amd64\Quernc"
#     }
# }


# --- 修复后的 QVM 重命名逻辑 ---
$TemplatePath = $Paths.template # 先提取变量，方便阅读

# 1. 安全地删除旧的 Qvm.exe (如果存在)
$OldQvmPath = Join-Path $TemplatePath "Qvm.exe"
if (Test-Path $OldQvmPath) {
    Write-Host "[INFO] Removing old Qvm.exe..." -ForegroundColor Gray
    Remove-Item $OldQvmPath -Force
} else {
    Write-Host "[INFO] No old Qvm.exe to remove." -ForegroundColor Gray
}

# 2. 准备源文件路径
$SourceExePath = Join-Path $TemplatePath "Quern_cargo.exe"

# 3. 检查源文件是否存在，防止 Move-Item 崩溃
if (Test-Path $SourceExePath) {
    Write-Host "[MOVE] Renaming Quern_cargo.exe -> Qvm.exe" -ForegroundColor Cyan
    try {
        Move-Item -Path $SourceExePath -Destination $OldQvmPath -Force
        Write-Host "[SUCCESS] Qvm.exe created successfully." -ForegroundColor Green
    } catch {
        Write-Host "[ERROR] Failed to move/rename Quern_cargo.exe: $_" -ForegroundColor Red
    }
} else {
    Write-Host "[WARNING] Quern_cargo.exe not found at $SourceExePath, skipping Qvm rename." -ForegroundColor Yellow
}

# --- 打包与安装程序 ---
if ($Targets.pack_zip) {
    Invoke-BuildStep "打包 ZIP" {
        $ZipPath = "$($Paths.root)\Quern-Linux&Windows-amd64-LSDK.zip"
        & $Tools.seven_zip a $ZipPath "$($Paths.template)\*"
        & $Tools.seven_zip d $ZipPath "setup.iss" # 删除不需要的文件
        Safe-Move $ZipPath "$($Paths.versions)\$Version\$(Split-Path $ZipPath -Leaf)"
    }
}

if ($Targets.pack_installer) {
    Invoke-BuildStep "生成 Inno Setup 安装包" {
        & $Tools.inno_setup /O"$($Paths.output)" "$($Paths.template)\setup.iss"
        $InstallerPath = "$($Paths.output)\Quern-Windows-amd64-LSDK_Installer.exe"
        Safe-Move $InstallerPath "$($Paths.versions)\$Version\$(Split-Path $InstallerPath -Leaf)"
    }
}

Copy-Item -Path "I:\Quern\Quern_Template\Windows\Q*" -Destination "I:\VW\vw_sys\user\deft\Quern" -Force
Copy-Item -Path "I:\Quern\Quern_Template\Windows\Updater.exe" -Destination "I:\VW\vw_sys\user\deft\Quern" -Force

Write-Host "`n? 所有配置驱动的任务已完成！版本: $Version" -ForegroundColor Green
Get-ChildItem -Path "$($Paths.versions)\$version"