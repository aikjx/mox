$p = 'd:\a10\aikjx\gitcode\infotopograph\docs\database\mox_sys\mox_sys-seed.sql'
$c = Get-Content -Raw $p
# 字典类型 INSERT 语句数
$t = ([regex]::Matches($c,'INSERT IGNORE INTO `sys_enum_type`')).Count
Write-Host ("enum_type_inserts=$t")
# 字典条目行数：每个条目一行以 UNHEX('...0100... 开头
$items = [regex]::Matches($c,"(?m)^\(UNHEX\('0000000000000000000000000100[0-9a-f]{4}'\),@et,")
Write-Host ("enum_item_rows=$($items.Count)")
# 全部 32 位 UNHEX id 并查重
$u = [regex]::Matches($c,"UNHEX\('([0-9a-f]{32})'\)")
$ids = $u | ForEach-Object { $_.Groups[1].Value }
Write-Host ("unhex_total=$($ids.Count)")
$dup = $ids | Group-Object | Where-Object { $_.Count -gt 1 }
if ($dup) { Write-Host ('DUP_FOUND=' + (($dup | ForEach-Object { $_.Name }) -join ',')) } else { Write-Host 'ids_unique=OK' }
$objs = $ids | Where-Object { $_ -match '0000000000000000000000000000000[1-7]$' }
Write-Host ("platform_object_ids=$($objs.Count)")
$ty = $ids | Where-Object { $_ -match '00e000' }
Write-Host ("type_marker_ids=$($ty.Count)")
$vm = $ids | Where-Object { $_ -notmatch '00e000' -and $_ -notmatch '0000000000000000000000000000000[1-7]$' }
Write-Host ("item_marker_ids=$($vm.Count)")
