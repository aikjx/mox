'use strict';

/**
 * 知识库域 · 纯算法层：版本差异对比（LCS 最长公共子序列，零 IO）
 */

/** 基于行级 LCS 的版本对比：返回新增行/删除行/相似度 */
function diffVersions(ver1, ver2) {
  const lines1 = (ver1.content || '').split('\n');
  const lines2 = (ver2.content || '').split('\n');
  const lcs = [];
  for (let i = 0; i <= lines1.length; i++) {
    lcs[i] = [];
    for (let j = 0; j <= lines2.length; j++) lcs[i][j] = 0;
  }
  for (let i = 1; i <= lines1.length; i++) {
    for (let j = 1; j <= lines2.length; j++) {
      if (lines1[i - 1] === lines2[j - 1]) lcs[i][j] = lcs[i - 1][j - 1] + 1;
      else lcs[i][j] = Math.max(lcs[i - 1][j], lcs[i][j - 1]);
    }
  }
  const added = [];
  const removed = [];
  let i = lines1.length, j = lines2.length;
  while (i > 0 && j > 0) {
    if (lines1[i - 1] === lines2[j - 1]) { i--; j--; }
    else if (lcs[i - 1][j] >= lcs[i][j - 1]) { removed.unshift(lines1[i - 1]); i--; }
    else { added.unshift(lines2[j - 1]); j--; }
  }
  while (i > 0) { removed.unshift(lines1[i - 1]); i--; }
  while (j > 0) { added.unshift(lines2[j - 1]); j--; }
  const total = Math.max(lines1.length, lines2.length);
  const similarity = total > 0 ? Math.round((lcs[lines1.length][lines2.length] / total) * 1000) / 10 : 0;
  return { added, removed, changed: [], similarity, fromVersion: ver1.version, toVersion: ver2.version };
}

module.exports = { diffVersions };
