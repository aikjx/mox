// 上传后的乐观占位行 —— 单源。
// 本轮之前这段在 views/workspace/panels 的两个面板里逐字符各有一份：
// getFileType 574 B x2、formatFileSize 210 B x2、占位行块 328 B x2（三处函数体 sha 现场比对相同才动手）。
// 键与文案逐字照搬原副本，本文件不改进任何行为；'我'/'刚刚' 是乐观本地行的占位口径，
// 真实归属与时间要等后端回读后由列表刷新覆盖（这条边界见 FRONTEND-MODULE-GOVERNANCE §5.28）。
export function getFileType(filename) {
  const ext = filename.split('.').pop()?.toLowerCase()
  if (['pdf'].includes(ext)) return 'pdf'
  if (['doc', 'docx', 'txt', 'md'].includes(ext)) return 'doc'
  if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext)) return 'image'
  if (['xls', 'xlsx', 'csv'].includes(ext)) return 'excel'
  if (['ppt', 'pptx'].includes(ext)) return 'ppt'
  if (['zip', 'rar', '7z', 'tar', 'gz'].includes(ext)) return 'zip'
  if (['js', 'ts', 'py', 'java', 'go', 'cpp', 'html', 'css', 'vue', 'json'].includes(ext)) return 'code'
  return 'other'
}

export function formatFileSize(bytes) {
  if (!bytes) return '未知'
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / 1048576).toFixed(1) + ' MB'
}

/** 文件对象 -> 列表行（与合并前两份副本的输出逐字段一致） */
export function makeUploadRow(file) {
  return {
    id: 'f-' + Date.now(),
    name: file.name,
    type: getFileType(file.name),
    size: formatFileSize(file.size),
    uploader: '我',
    time: '刚刚'
  
  }
}
