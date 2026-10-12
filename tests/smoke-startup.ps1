param(
    [string]$ExePath = (Join-Path (Split-Path $PSScriptRoot -Parent) 'TipTips-normalized.exe'),
    [int]$Seconds = 45,
    [int]$WindowWidth = 360,
    [switch]$DarkMode,
    [ValidateSet('list', 'calendar')][string]$ViewMode = 'list'
)

$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $ExePath)) { throw "Executable not found: $ExePath" }
$smokeRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('TipTips-smoke-' + [guid]::NewGuid().ToString('N'))
[System.IO.Directory]::CreateDirectory($smokeRoot) | Out-Null
@'
# Startup fixture directory
This directory contains only synthetic TipTips app data and a disposable WebView profile.
No real email account, API key or existing application data is used.
'@ | Set-Content -LiteralPath (Join-Path $smokeRoot 'AGENTS.md') -Encoding utf8

# Exercise actual list/calendar rendering with synthetic personal and official cards.
$fixtureDir = Join-Path $smokeRoot 'TipTips'
[System.IO.Directory]::CreateDirectory($fixtureDir) | Out-Null
'# Fixture files: synthetic data.json and seu_fairs.json only; no email credentials.' |
    Set-Content -LiteralPath (Join-Path $fixtureDir 'AGENTS.md') -Encoding utf8
$fixtureDate = [DateTime]::UtcNow.AddHours(8).ToString('yyyy-MM-dd')
$schedule = @{ event_type = '面试'; date = $fixtureDate; time = '14:00'; time_kind = 'start';
    timezone = 'UTC+08:00'; location = '线上会议'; notes = '提前10分钟进入，准备简历与项目介绍。';
    company = '示例公司'; link = ''; source_subject = '合成界面检查'; source_date = $fixtureDate }
$personal = @{ id = 'fixture-task'; text = '[面试] 示例公司 · 后端开发 · 二面'; done = $false;
    created_date = $fixtureDate; carried_over = $false; sort_order = 0; source = 'manual'; schedule = $schedule }
$completed = @{ id = 'fixture-done'; text = '已完成：准备项目介绍与简历'; done = $true;
    created_date = $fixtureDate; carried_over = $false; sort_order = 1; source = 'manual' }
$official = @{ id = 'seu:fixture'; text = '[宣讲会] 示例科技公司校园招聘宣讲会'; done = $false;
    created_date = $fixtureDate; carried_over = $false; sort_order = 0; source = 'seu';
    schedule = @{ event_type = '宣讲会'; date = $fixtureDate; time = '16:00'; end_date = $fixtureDate;
        end_time = '18:00'; time_kind = 'window'; timezone = 'UTC+08:00';
        location = '九龙湖校区 大学生活动中心'; notes = ''; link = ''; source_subject = '合成官网日程'; source_date = $fixtureDate } }
@{ last_active_date = $fixtureDate; items = @($personal, $completed) } | ConvertTo-Json -Depth 8 |
    Set-Content -LiteralPath (Join-Path $fixtureDir 'data.json') -Encoding utf8NoBOM
@{ updated_at = "$fixtureDate 10:00:00"; items = @($official) } | ConvertTo-Json -Depth 8 |
    Set-Content -LiteralPath (Join-Path $fixtureDir 'seu_fairs.json') -Encoding utf8NoBOM
@{ x = 100; y = 100; width = $WindowWidth; height = 500; view_mode = $ViewMode; opacity = 1 } |
    ConvertTo-Json | Set-Content -LiteralPath (Join-Path $fixtureDir 'config.json') -Encoding utf8NoBOM

$startInfo = [System.Diagnostics.ProcessStartInfo]::new()
$startInfo.FileName = (Get-Item -LiteralPath $ExePath).FullName
$startInfo.WorkingDirectory = Split-Path $startInfo.FileName -Parent
$startInfo.UseShellExecute = $false
$startInfo.CreateNoWindow = $true
$startInfo.WindowStyle = [System.Diagnostics.ProcessWindowStyle]::Hidden
$startInfo.RedirectStandardOutput = $true
$startInfo.RedirectStandardError = $true
$startInfo.EnvironmentVariables['APPDATA'] = $smokeRoot
$startInfo.EnvironmentVariables['LOCALAPPDATA'] = $smokeRoot
$startInfo.EnvironmentVariables['WEBVIEW2_USER_DATA_FOLDER'] = Join-Path $smokeRoot 'WebView'
$startInfo.EnvironmentVariables['TIPTIPS_OFFLINE'] = '1'
if ($DarkMode) { $startInfo.EnvironmentVariables['WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS'] = '--force-dark-mode' }
$process = [System.Diagnostics.Process]::Start($startInfo)
try {
    if ($process.WaitForExit($Seconds * 1000)) {
        $stderr = $process.StandardError.ReadToEnd()
        throw "Application exited before ${Seconds}s: code=$($process.ExitCode); stderr=$stderr"
    }
    $process.Refresh()
    if (-not $process.Responding -or $process.MainWindowHandle -eq 0 -or $process.MainWindowTitle -ne 'TipTips') {
        throw "No responsive TipTips main window after ${Seconds}s"
    }
    [pscustomobject]@{ Result = 'PASS'; ObservedSeconds = $Seconds; ProcessId = $process.Id;
        Responding = $process.Responding; WindowTitle = $process.MainWindowTitle; FixturePath = $smokeRoot } | Format-List
}
finally {
    # Stop only the synthetic instance started by this test.
    if (-not $process.HasExited) { $process.Kill(); $process.WaitForExit(5000) | Out-Null }
    $process.Dispose()
}
