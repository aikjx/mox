# PowerShell script to rename files with brand string pollution
$BASE = 'd:\a10\aikjx\gitcode\infotopograph\docs'
$OLD = 'mox 模块化系统架构'
$NEW = 'MOX'

Get-ChildItem -Path $BASE -Recurse -File | Where-Object {
    $_.Name -match $OLD -and $_.FullName -notmatch '\\_archive\\'
} | ForEach-Object {
    $newName = $_.Name -replace [regex]::Escape($OLD), $NEW
    $newPath = Join-Path $_.DirectoryName $newName
    if (-not (Test-Path $newPath)) {
        Rename-Item -LiteralPath $_.FullName -NewName $newName -Force
        Write-Output "OK: $($_.Name) -> $newName"
    } else {
        Write-Output "SKIP (exists): $($_.Name) -> $newName"
    }
}
Write-Output "Done!"
