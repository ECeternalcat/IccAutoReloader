# IccAutoReloader Registry Cleaner
# 用于清理测试时遗留的注册表项

$ErrorActionPreference = "SilentlyContinue"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host " IccAutoReloader Registry Cleaner " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$cleaned = 0

# 1. 清理开机自启项
Write-Host "[1/2] 清理开机自启项..." -ForegroundColor Yellow
$runPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
if (Get-ItemProperty -Path $runPath -Name "IccAutoReloader" -ErrorAction SilentlyContinue) {
    Remove-ItemProperty -Path $runPath -Name "IccAutoReloader"
    Write-Host "      已删除开机自启项" -ForegroundColor Green
    $cleaned++
} else {
    Write-Host "      开机自启项不存在" -ForegroundColor Gray
}

# 2. 清理软件核心配置项
Write-Host "[2/2] 清理软件配置项..." -ForegroundColor Yellow
$configPath = "HKCU:\Software\IccAutoReloader"
if (Test-Path $configPath) {
    Remove-Item -Path $configPath -Recurse
    Write-Host "      已删除软件配置项 (将触发首次运行向导)" -ForegroundColor Green
    $cleaned++
} else {
    Write-Host "      软件配置项不存在" -ForegroundColor Gray
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
if ($cleaned -gt 0) {
    Write-Host " 清理完成！已清理 $cleaned 项" -ForegroundColor Green
} else {
    Write-Host " 没有需要清理的项" -ForegroundColor Gray
}
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "按任意键退出..." -ForegroundColor Gray
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
