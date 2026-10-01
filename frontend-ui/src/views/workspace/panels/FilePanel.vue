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
import { makeUploadRow } from '@/modules/_kernel/upload-row'
import { ref } from 'vue'
import { Upload, Document, Download } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus/es/components/message/index'

const props = defineProps({
  sharedFiles: { type: Array, default: () => [] }
})

const emit = defineEmits(['preview-file', 'download-file', 'file-uploaded'])

const fileDragOver = ref(false)

function fileIconEmoji(type) {
  const icons = { pdf: '📕', doc: '📘', image: '🖼️', excel: '📗', ppt: '📙', zip: '📦', code: '💻', other: '📄' }
  return icons[type] || '📄'
}





function handleBeforeFileUpload(file) {
  const newFile = makeUploadRow(file)
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
