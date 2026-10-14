# ====== 配置区 ======
$remoteUrl       = ""
$targetBranch    = ""
$customTempDir   = ""

$localPath = Get-Location

# 初始化配置
if ([string]::IsNullOrWhiteSpace($remoteUrl)) {
    try { $remoteUrl = git config --get remote.origin.url } catch { $remoteUrl = "" }
}
if ([string]::IsNullOrWhiteSpace($targetBranch)) {
    try { $targetBranch = git rev-parse --abbrev-ref HEAD } catch { $targetBranch = "main" }
}
if ([string]::IsNullOrWhiteSpace($customTempDir)) {
    $tempDir = Join-Path $env:TEMP "git_remote_temp_$(Get-Date -Format yyyyMMddHHmmss)"
} else {
    if (-not (Test-Path $customTempDir)) { New-Item -ItemType Directory -Path $customTempDir -Force | Out-Null }
    $tempDir = Join-Path $customTempDir "git_remote_temp_$(Get-Date -Format yyyyMMddHHmmss)"
}

Write-Host "1. 正在克隆远程仓库（分支: $targetBranch）到临时目录..." -ForegroundColor Cyan
try {
    if ([string]::IsNullOrWhiteSpace($remoteUrl)) { throw "未获取到远程仓库地址。" }
    git clone --branch $targetBranch --depth 1 $remoteUrl $tempDir 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "克隆失败，请检查远程地址和分支名。" }
    Write-Host "克隆完成，开始进行文件内容对比..." -ForegroundColor Cyan
} catch {
    Write-Host "错误: $_" -ForegroundColor Red
    exit 1
}

Push-Location $localPath
try {
    # 2. 获取本地和远程的文件列表
    $localFiles = (git ls-files -o --exclude-standard) + (git ls-files)
    Push-Location $tempDir
    $remoteFiles = (git ls-files)
    Pop-Location

    # 3. 对比差异
    $diffObjects = Compare-Object -ReferenceObject $localFiles -DifferenceObject $remoteFiles
    
    if (-not $diffObjects) {
        Write-Host "本地与远程完全一致，无差异文件。" -ForegroundColor Green
        exit 0
    }

    $diffFiles = @()
    Write-Host "发现以下差异文件：" -ForegroundColor Yellow
    foreach ($obj in $diffObjects) {
        $fileName = $obj.InputObject
        if ($obj.SideIndicator -eq '<=') {
            Write-Host " - [本地变更] $fileName" -ForegroundColor Green
            $diffFiles += $fileName
        } elseif ($obj.SideIndicator -eq '=>') {
            Write-Host " - [远程新增] $fileName (自动拉回本地)" -ForegroundColor Magenta
            $sourcePath = Join-Path $tempDir $fileName
            $destPath   = Join-Path $localPath $fileName
            $destDir    = Split-Path $destPath -Parent
            if (-not (Test-Path $destDir)) { New-Item -ItemType Directory -Path $destDir -Force | Out-Null }
            Copy-Item $sourcePath $destPath -Force
            $diffFiles += $fileName
        }
    }

    # 4. 逐个安全处理差异文件（采用独立进程提交策略）
    foreach ($file in $diffFiles) {
        Write-Host "`n处理文件: $file" -ForegroundColor Cyan
        
        $commitMessage = Read-Host "请输入该文件的提交消息"
        if ([string]::IsNullOrWhiteSpace($commitMessage)) {
            Write-Host "提交消息为空，已跳过。" -ForegroundColor Magenta
            continue
        }

        # 步骤一：在独立进程中添加当前单个文件到暂存区
        Start-Process -FilePath "git" -ArgumentList "add", "--", $file -NoNewWindow -Wait -WorkingDirectory $localPath

        # 步骤二：检查暂存区是否有该文件的实际变更
        $checkStatus = git diff --cached --name-only $file
        if (-not $checkStatus) {
            Write-Host "该文件无实际变更，自动跳过提交并重置暂存区。" -ForegroundColor Magenta
            # 在独立进程中重置，避免污染
            Start-Process -FilePath "git" -ArgumentList "reset", "--", $file -NoNewWindow -Wait -WorkingDirectory $localPath
            continue
        }

        # 步骤三：在独立进程中提交
        Start-Process -FilePath "git" -ArgumentList "commit", "-m", $commitMessage -NoNewWindow -Wait -WorkingDirectory $localPath
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "成功提交: $file" -ForegroundColor Green
        } else {
            Write-Host "提交失败: $file" -ForegroundColor Red
        }
    }

    # 5. 最后统一推送到远程
    Write-Host "`n正在推送更改到 origin $targetBranch ..." -ForegroundColor Cyan
    git push origin $targetBranch
    if ($LASTEXITCODE -eq 0) {
        Write-Host "推送成功！所有操作已完成。" -ForegroundColor Green
    } else {
        Write-Host "推送失败，请检查网络或远程仓库冲突。" -ForegroundColor Red
    }

} finally {
    # 6. 清理临时文件夹
    if ((Get-Location).Path -ne $localPath.Path) { Pop-Location }
    if (Test-Path $tempDir) { 
        Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue 
        Write-Host "临时目录已自动清理。" -ForegroundColor Cyan 
    }
}