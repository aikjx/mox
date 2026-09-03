<!--
  共享文件面板
  职责：文件列表展示、上传、预览、下载
-->
<template>
  <div class="ws-files-content-inner">
    <!-- 上传区域 -->
    <div class="ws-files-upload-area"
      @dragover.prevent="fileDragOver = true"
      @dragleave="fileDragOver = false"
      @drop.prevent="handleFileDropToFiles"
      :class="{ 'drag-over': fileDragOver }"
    >
      <el-icon class="upload-area-icon"><Upload /></el-icon>
      <div class="upload-area-text">拖拽文件到此处上传</div>
      <div class="upload-area-hint">或</div>
      <el-upload
        :show-file-list="false"
        :before-upload="handleBeforeFileUpload"
        multiple
      >
        <el-button type="primary" plain size="small">点击选择文件</el-button>
      </el-upload>
    </div>

    <!-- 文件网格 -->
    <el-scrollbar class="ws-files-scroll">
      <div v-if="sharedFiles.length === 0" class="ws-files-empty">
        <el-empty description="暂无共享文件" :image-size="60" />
      </div>
      <div v-else class="ws-files-grid">
        <div
          v-for="file in sharedFiles"
          :key="file.id"
          class="ws-file-card-large"
          @click="$emit('preview-file', file)"
        >
          <div class="ws-file-preview" :class="'preview-' + file.type">
            <span class="file-preview-icon">{{ fileIconEmoji(file.type) }}</span>
          </div>
          <div class="ws-file-card-body">
            <div class="ws-file-name-row">
              <span class="ws-file-name-large">{{ file.name }}</span>
            </div>
            <div class="ws-file-meta-row">
              <span>{{ file.size }}</span>
              <span>·</span>
              <span>{{ file.uploader }}</span>
            </div>
            <div class="ws-file-actions-row">
              <el-button size="small" text @click.stop="$emit('preview-file', file)">
                <el-icon><Document /></el-icon>
                预览
              </el-button>
              <el-button size="small" text @click.stop="$emit('download-file', file)">
                <el-icon><Download /></el-icon>
                下载
              </el-button>
            </div>
          </div>
        </div>
      </div>
    </el-scrollbar>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { Upload, Document, Download } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'

const props = defineProps({
  sharedFiles: { type: Array, default: () => [] }
})

const emit = defineEmits(['preview-file', 'download-file', 'file-uploaded'])

const fileDragOver = ref(false)

function fileIconEmoji(type) {
  const icons = { pdf: '📕', doc: '📘', image: '🖼️', excel: '📗', ppt: '📙', zip: '📦', code: '💻', other: '📄' }
  return icons[type] || '📄'
}

function getFileType(filename) {
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

function formatFileSize(bytes) {
  if (!bytes) return '未知'
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / 1048576).toFixed(1) + ' MB'
}

function handleBeforeFileUpload(file) {
  const type = getFileType(file.name)
  const newFile = {
    id: 'f-' + Date.now(),
    name: file.name,
    type: type,
    size: formatFileSize(file.size),
    uploader: '我',
    time: '刚刚'
  }
  emit('file-uploaded', newFile)
  ElMessage.success(`文件「${file.name}」上传成功`)
  return false
}

function handleFileDropToFiles(e) {
  fileDragOver.value = false
  const files = e.dataTransfer?.files
  if (files && files.length > 0) {
    Array.from(files).forEach(file => {
      handleBeforeFileUpload(file)
    })
  }
}
</script>
