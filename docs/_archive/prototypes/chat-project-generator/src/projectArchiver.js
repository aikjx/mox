/**
 * 项目归档器
 * 功能：将工作流执行结果归档到项目目录，建立版本快照，生成索引
 *
 * 核心能力：
 * 1. 产物分类归档
 * 2. 版本快照管理
 * 3. 索引生成
 * 4. 元数据提取与存储
 */

import { ref, computed } from 'vue'

// ============================================================
// 归档配置
// ============================================================
const ARCHIVE_CONFIG = {
  // 目录结构
  structure: {
    artifacts: 'artifacts',       // 产物目录
    logs: 'logs',                 // 日志目录
    snapshots: 'snapshots',       // 快照目录
    index: 'index.md',            // 索引文件
    metadata: 'metadata.json'     // 元数据文件
  },
  // 版本策略
  versioning: {
    strategy: 'semantic',         // semantic | timestamp | commit_hash
    autoTag: true
  },
  // 保留策略
  retention: {
    maxSnapshots: 50,             // 最多保留快照数
    maxAgeDays: 90                // 最大保留天数
  }
}

// ============================================================
// 产物类型分类
// ============================================================
const ARTIFACT_CATEGORIES = {
  document: {
    name: '文档',
    icon: '📄',
    extensions: ['.md', '.txt', '.pdf', '.docx', '.html']
  },
  code: {
    name: '代码',
    icon: '💻',
    extensions: ['.js', '.ts', '.py', '.java', '.go', '.rs', '.cpp', '.c', '.h', '.json', '.yaml', '.yml']
  },
  data: {
    name: '数据',
    icon: '📊',
    extensions: ['.csv', '.xlsx', '.xls', '.json', '.parquet', '.sql']
  },
  image: {
    name: '图片',
    icon: '🖼️',
    extensions: ['.png', '.jpg', '.jpeg', '.gif', '.svg', '.webp']
  },
  archive: {
    name: '压缩包',
    icon: '📦',
    extensions: ['.zip', '.tar', '.gz', '.rar', '.7z']
  },
  other: {
    name: '其他',
    icon: '📁',
    extensions: []
  }
}

/**
 * 获取产物类别
 * @param {string} filename 文件名
 * @returns {object} 类别信息
 */
function getArtifactCategory(filename) {
  const ext = '.' + filename.split('.').pop().toLowerCase()
  for (const [key, category] of Object.entries(ARTIFACT_CATEGORIES)) {
    if (category.extensions.includes(ext)) {
      return { key, ...category }
    }
  }
  return { key: 'other', ...ARTIFACT_CATEGORIES.other }
}

// ============================================================
// 版本号生成器
// ============================================================

/**
 * 生成版本号
 * @param {string} strategy 策略
 * @param {string} previousVersion 上一个版本号
 * @returns {string} 新版本号
 */
function generateVersion(strategy = 'semantic', previousVersion = null) {
  const now = new Date()

  switch (strategy) {
    case 'timestamp':
      return now.toISOString().replace(/[:.]/g, '-')

    case 'commit_hash':
      return Math.random().toString(36).substring(2, 10)

    case 'semantic':
    default:
      if (previousVersion) {
        // 递增补丁版本
        const parts = previousVersion.split('.')
        if (parts.length === 3) {
          parts[2] = parseInt(parts[2]) + 1
          return parts.join('.')
        }
      }
      return `1.0.${Math.floor(now.getTime() / 1000) % 10000}`
  }
}

// ============================================================
// 索引生成器
// ============================================================

/**
 * 生成项目索引 Markdown
 * @param {object} project 项目信息
 * @param {Array} snapshots 快照列表
 * @param {Array} artifacts 产物列表
 * @returns {string} Markdown 内容
 */
function generateIndexMarkdown(project, snapshots = [], artifacts = []) {
  const latestSnapshot = snapshots[0]

  let content = `# ${project.name}\n\n`
  content += `> ${project.description || '暂无描述'}\n\n`

  // 基本信息
  content += `## 📋 项目信息\n\n`
  content += `- **项目ID**: ${project.id}\n`
  content += `- **创建时间**: ${project.createdAt || '未知'}\n`
  content += `- **分类**: ${project.category || '未分类'}\n`
  if (project.tags?.length) {
    content += `- **标签**: ${project.tags.map(t => `\`${t}\``).join(' ')}\n`
  }
  content += `\n`

  // 最新快照
  if (latestSnapshot) {
    content += `## 🆕 最新版本\n\n`
    content += `- **版本**: ${latestSnapshot.version}\n`
    content += `- **时间**: ${latestSnapshot.createdAt}\n`
    content += `- **产物数**: ${latestSnapshot.artifactCount}\n`
    content += `- **说明**: ${latestSnapshot.description || '无'}\n`
    content += `\n`
  }

  // 产物列表
  if (artifacts.length > 0) {
    content += `## 📦 产物列表\n\n`

    // 按类别分组
    const byCategory = {}
    artifacts.forEach(a => {
      const cat = getArtifactCategory(a.name)
      if (!byCategory[cat.key]) {
        byCategory[cat.key] = []
      }
      byCategory[cat.key].push(a)
    })

    for (const [catKey, catArtifacts] of Object.entries(byCategory)) {
      const cat = ARTIFACT_CATEGORIES[catKey] || ARTIFACT_CATEGORIES.other
      content += `### ${cat.icon} ${cat.name}\n\n`
      catArtifacts.forEach(a => {
        const path = `${ARCHIVE_CONFIG.structure.artifacts}/${a.name}`
        content += `- [${a.name}](${path})`
        if (a.agentName) {
          content += ` _(由 ${a.agentName} 生成)_`
        }
        content += '\n'
      })
      content += '\n'
    }
  }

  // 版本历史
  if (snapshots.length > 0) {
    content += `## 📜 版本历史\n\n`
    content += `| 版本 | 时间 | 产物数 | 说明 |\n`
    content += `|------|------|--------|------|\n`
    snapshots.slice(0, 10).forEach(s => {
      content += `| ${s.version} | ${s.createdAt} | ${s.artifactCount} | ${s.description || '-'} |\n`
    })
    content += `\n`
  }

  content += `---\n`
  content += `_由 MOX AI 平台自动生成于 ${new Date().toISOString()}_\n`

  return content
}

// ============================================================
// 组合式函数：项目归档器
// ============================================================

/**
 * 项目归档器
 */
export function useProjectArchiver() {
  const isArchiving = ref(false)
  const currentSnapshot = ref(null)
  const snapshots = ref([])
  const archiveProgress = ref(0)
  const lastError = ref(null)

  // 计算属性
  const latestSnapshot = computed(() => snapshots.value[0] || null)
  const snapshotCount = computed(() => snapshots.value.length)

  /**
   * 归档工作流执行结果
   * @param {object} params 归档参数
   * @param {object} params.project 项目信息
   * @param {Array} params.artifacts 产物列表
   * @param {object} params.executionResult 执行结果
   * @param {string} params.description 快照描述
   * @returns {Promise<object>} 归档结果
   */
  async function archive(params = {}) {
    const {
      project,
      artifacts = [],
      executionResult = null,
      description = ''
    } = params

    if (!project) {
      throw new Error('归档需要项目信息')
    }

    isArchiving.value = true
    archiveProgress.value = 0
    lastError.value = null

    try {
      // Step 1: 生成版本号
      const prevVersion = snapshots.value[0]?.version
      const version = generateVersion(ARCHIVE_CONFIG.versioning.strategy, prevVersion)

      archiveProgress.value = 10

      // Step 2: 分类产物
      const categorizedArtifacts = artifacts.map(artifact => ({
        ...artifact,
        category: getArtifactCategory(artifact.name),
        path: `${ARCHIVE_CONFIG.structure.artifacts}/${artifact.name}`
      }))

      archiveProgress.value = 30

      // Step 3: 创建快照
      const snapshot = {
        id: `snap_${Date.now()}`,
        version,
        projectId: project.id,
        createdAt: new Date().toISOString(),
        artifactCount: artifacts.length,
        artifacts: categorizedArtifacts,
        description,
        executionResult: executionResult ? {
          status: executionResult.status,
          duration_ms: executionResult.duration_ms,
          agentResults: executionResult.agent_results?.length || 0
        } : null
      }

      archiveProgress.value = 60

      // Step 4: 生成索引
      const indexContent = generateIndexMarkdown(
        project,
        [snapshot, ...snapshots.value],
        categorizedArtifacts
      )

      archiveProgress.value = 80

      // Step 5: 生成元数据
      const metadata = {
        project: {
          id: project.id,
          name: project.name,
          description: project.description,
          category: project.category,
          tags: project.tags || []
        },
        latestVersion: version,
        snapshotCount: snapshots.value.length + 1,
        totalArtifacts: artifacts.length,
        lastUpdated: new Date().toISOString()
      }

      archiveProgress.value = 90

      // Step 6: 保存快照到列表
      snapshots.value.unshift(snapshot)
      currentSnapshot.value = snapshot

      // 清理旧快照（超过最大保留数）
      if (snapshots.value.length > ARCHIVE_CONFIG.retention.maxSnapshots) {
        snapshots.value = snapshots.value.slice(0, ARCHIVE_CONFIG.retention.maxSnapshots)
      }

      archiveProgress.value = 100

      return {
        success: true,
        snapshot,
        indexContent,
        metadata,
        artifactCategories: Object.keys(ARTIFACT_CATEGORIES).map(key => ({
          key,
          ...ARTIFACT_CATEGORIES[key]
        }))
      }

    } catch (error) {
      console.error('归档失败:', error)
      lastError.value = error.message
      throw error
    } finally {
      isArchiving.value = false
    }
  }

  /**
   * 获取快照详情
   * @param {string} snapshotId 快照ID
   * @returns {object|null} 快照详情
   */
  function getSnapshot(snapshotId) {
    return snapshots.value.find(s => s.id === snapshotId) || null
  }

  /**
   * 获取指定版本的快照
   * @param {string} version 版本号
   * @returns {object|null} 快照
   */
  function getSnapshotByVersion(version) {
    return snapshots.value.find(s => s.version === version) || null
  }

  /**
   * 比较两个版本的差异
   * @param {string} versionA 版本A
   * @param {string} versionB 版本B
   * @returns {object} 差异信息
   */
  function compareVersions(versionA, versionB) {
    const snapA = getSnapshotByVersion(versionA)
    const snapB = getSnapshotByVersion(versionB)

    if (!snapA || !snapB) {
      return null
    }

    const artifactsA = new Set(snapA.artifacts.map(a => a.name))
    const artifactsB = new Set(snapB.artifacts.map(a => a.name))

    const added = [...artifactsB].filter(a => !artifactsA.has(a))
    const removed = [...artifactsA].filter(a => !artifactsB.has(a))
    const common = [...artifactsA].filter(a => artifactsB.has(a))

    return {
      versionA,
      versionB,
      added,
      removed,
      common,
      summary: {
        addedCount: added.length,
        removedCount: removed.length,
        commonCount: common.length
      }
    }
  }

  /**
   * 导出归档数据
   * @param {object} project 项目信息
   * @returns {object} 完整归档数据
   */
  function exportArchive(project) {
    return {
      project,
      snapshots: snapshots.value,
      config: ARCHIVE_CONFIG,
      exportedAt: new Date().toISOString()
    }
  }

  /**
   * 导入归档数据
   * @param {object} archiveData 归档数据
   */
  function importArchive(archiveData) {
    if (archiveData.snapshots) {
      snapshots.value = archiveData.snapshots
    }
  }

  /**
   * 重置归档器状态
   */
  function reset() {
    isArchiving.value = false
    currentSnapshot.value = null
    snapshots.value = []
    archiveProgress.value = 0
    lastError.value = null
  }

  return {
    // 状态
    isArchiving,
    currentSnapshot,
    snapshots,
    archiveProgress,
    lastError,
    // 计算属性
    latestSnapshot,
    snapshotCount,
    // 方法
    archive,
    getSnapshot,
    getSnapshotByVersion,
    compareVersions,
    generateIndexMarkdown,
    getArtifactCategory,
    exportArchive,
    importArchive,
    reset,
    // 常量
    ARCHIVE_CONFIG,
    ARTIFACT_CATEGORIES
  }
}

export default useProjectArchiver
