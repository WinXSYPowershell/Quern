# 1. 获取远程仓库最新信息，确保对比准确
git fetch origin

# 2. 获取当前本地分支与远程 main 分支相比有差异的文件列表
# --name-only 表示只输出文件名
$changedFiles = git diff --name-only origin/main..HEAD

# 检查是否有修改的文件
if (-not $changedFiles) {
    Write-Host "没有发现与远程 main 分支相比有差异的文件，无需提交。" -ForegroundColor Green
    exit 0
}

Write-Host "发现以下文件有修改：" -ForegroundColor Yellow
$changedFiles | ForEach-Object { Write-Host " - $_" }

# 3. 遍历每个文件，要求用户输入提交信息并提交
foreach ($file in $changedFiles) {
    Write-Host "`n正在处理文件: $file" -ForegroundColor Cyan
    
    # 将当前文件添加到暂存区
    git add $file
    
    # 提示用户输入该文件的提交信息
    $commitMsg = Read-Host "请输入文件 [$file] 的提交信息"
    
    # 如果用户输入为空，给出默认提示或跳过
    if ([string]::IsNullOrWhiteSpace($commitMsg)) {
        Write-Host "未输入提交信息，跳过该文件的提交。" -ForegroundColor Red
        git reset HEAD $file # 从暂存区撤销
        continue
    }
    
    # 执行提交
    git commit -m "$commitMsg"
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "文件 [$file] 提交成功！" -ForegroundColor Green
    } else {
        Write-Host "文件 [$file] 提交失败，请检查。" -ForegroundColor Red
    }
}

# 4. 将所有本地提交推送到远程 main 分支
Write-Host "`n所有文件处理完毕，正在推送到远程仓库..." -ForegroundColor Yellow
git push origin main

if ($LASTEXITCODE -eq 0) {
    Write-Host "推送成功！" -ForegroundColor Green
} else {
    Write-Host "推送失败，请检查网络连接或权限。" -ForegroundColor Red
}