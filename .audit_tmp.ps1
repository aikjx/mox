# mox_sys 量化审计脚本（一次性，审计后删除）
$ErrorActionPreference = 'Stop'
$dir = 'd:\a10\aikjx\gitcode\infotopograph\docs\database'
$tpl = Join-Path $dir 'mox_sys\mox_sys-universal-template.sql'
$c = Get-Content -Raw $tpl

Write-Host '==== 全局精算 ===='
Write-Host ("tables_total=" + ([regex]::Matches($c,'CREATE TABLE `[^`]+`')).Count)
Write-Host ("views_total=" + ([regex]::Matches($c,'CREATE OR REPLACE VIEW')).Count)
Write-Host ("check_constraints=" + ([regex]::Matches($c,'CONSTRAINT `chk_')).Count)
Write-Host ("unique_keys=" + ([regex]::Matches($c,'UNIQUE KEY `')).Count)
Write-Host ("plain_keys=" + ([regex]::Matches($c,'(?<!UNIQUE )KEY `idx_')).Count)

Write-Host '==== 每表精算 ===='
$mc = [regex]::Matches($c,'(?s)CREATE TABLE `([^`]+)`(.*?)ENGINE=InnoDB')
$rows = foreach ($m in $mc) {
  $name = $m.Groups[1].Value; $b = $m.Groups[2].Value
  $cols = ([regex]::Matches($b,'`[a-z0-9_]+`\s+(?:BINARY\(16\)|VARCHAR\(|VARBINARY\(|CHAR\(1\)|JSON|INT(?: UNSIGNED)?\s*$|INT(?: UNSIGNED)?\s+(?:NOT|DEFAULT)|BIGINT|SMALLINT|TINYINT|DECIMAL\(|DATETIME\(|TIMESTAMP|TEXT|MEDIUMTEXT|LONGTEXT|DOUBLE|FLOAT|GENERATED)')).Count
  $chk = ([regex]::Matches($b,'CONSTRAINT `chk_')).Count
  $uniq = ([regex]::Matches($b,'UNIQUE KEY `')).Count
  $idx = ([regex]::Matches($b,'(?<!UNIQUE )KEY `idx_')).Count
  $tenant = if ($b -match '`tenant_id`') {'Y'} else {'-'}
  $ut = if ($b -match '`updated_at`') {'Y'} else {'-'}
  $rv = if ($b -match '`row_version`') {'Y'} else {'-'}
  $dl = if ($b -match '`deleted_at`') {'Y'} else {'-'}
  [pscustomobject]@{table=$name;cols=$cols;chk=$chk;uk=$uniq;idx=$idx;tenant=$tenant;upd=$ut;rv=$rv;del=$dl}
}
$rows | Format-Table -AutoSize | Out-String -Width 200

Write-Host '==== 汇总 ===='
Write-Host ("columns_total=" + (($rows | Measure-Object cols -Sum).Sum))
Write-Host ("tables_no_tenant=" + (($rows | Where-Object tenant -eq '-').table -join ','))
Write-Host ("tables_no_updated_at=" + (($rows | Where-Object upd -eq '-').table -join ','))
Write-Host ("tables_no_row_version=" + (($rows | Where-Object rv -eq '-').table -join ','))
Write-Host ("tables_no_deleted_at=" + (($rows | Where-Object del -eq '-').table -join ','))

Write-Host '==== 前缀分布 ===='
$groups = $rows.table | ForEach-Object { ($_ -split '_',2)[0] } | Group-Object | Sort-Object Name
$groups | ForEach-Object { Write-Host ("prefix=" + $_.Name + " count=" + $_.Count) }

Write-Host '==== 交叉核对：其它文件声明 ===='
$base = Get-Content -Raw (Join-Path $dir 'mox-v3.0-baseline.sql')
Write-Host ("baseline_create_table=" + ([regex]::Matches($base,'(?m)^CREATE TABLE')).Count)
$ver = Get-Content -Raw (Join-Path $dir 'mox-v3.0-verification.sql')
$have = [regex]::Matches($c,'CREATE TABLE `([^`]+)`') | ForEach-Object { $_.Groups[1].Value }
$refs = [regex]::Matches($ver,'`((?:sys|mox_sys|rpa|ea|flow|meta|ai|kg|iam)_[a-z0-9_]+)`') | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique
$missing = $refs | Where-Object { $_ -notin $have }
Write-Host ("verification_tables_not_in_template=" + ($missing -join ','))
