<template>
  <div class="melody2score">
    <!-- 顶部状态栏 -->
    <div class="m2s-header">
      <div class="m2s-title">
        <span class="m2s-icon">♪</span>
        <span>Melody2Score 企业级转谱引擎</span>
        <el-tag :type="pyStatus === 'online' ? 'success' : 'danger'" size="small" effect="dark">
          {{ pyStatus === 'online' ? '引擎在线' : '引擎离线' }}
        </el-tag>
      </div>
      <div class="m2s-actions">
        <el-button size="small" @click="refreshSamples" :icon="'Refresh'">刷新样例</el-button>
        <el-button size="small" @click="checkHealth" :icon="'Monitor'">健康检查</el-button>
      </div>
    </div>

    <div class="m2s-layout">
      <!-- 左侧：输入区 -->
      <div class="m2s-left">
        <!-- 输入方式切换 -->
        <el-card shadow="never" class="m2s-card">
          <template #header>
            <span class="card-title">音频输入</span>
          </template>
          <el-tabs v-model="inputMode" @tab-change="onInputModeChange">
            <el-tab-pane label="上传文件" name="upload">
              <el-upload
                ref="uploadRef"
                drag
                :auto-upload="false"
                :show-file-list="true"
                accept="audio/*,.wav,.mp3,.m4a,.flac,.ogg"
                :limit="1"
                :on-change="onFileChange"
                class="m2s-upload"
              >
                <el-icon class="el-icon--upload"><UploadFilled /></el-icon>
                <div class="el-upload__text">拖拽或点击上传音频文件</div>
                <template #tip>
                  <div class="el-upload__tip">支持 WAV/MP3/M4A/FLAC/OGG，最大 50MB</div>
                </template>
              </el-upload>
            </el-tab-pane>

            <el-tab-pane label="内置样例" name="sample">
              <div class="sample-list">
                <div
                  v-for="g in samples"
                  :key="g.melody_index"
                  class="sample-item"
                  :class="{ active: selectedSample === g.melody_index }"
                  @click="selectSample(g)"
                >
                  <div class="sample-title">{{ g.title_zh }}<span class="sample-en"> ({{ g.title_en }})</span></div>
                  <div class="sample-timbres">
                    <el-tag
                      v-for="t in g.timbres"
                      :key="t.timbre"
                      :type="selectedTimbre === t.timbre ? 'primary' : 'info'"
                      size="small"
                      @click.stop="selectTimbre(g, t)"
                    >{{ t.timbre_zh || t.timbre }}</el-tag>
                  </div>
                </div>
                <el-empty v-if="!samples.length" description="暂无内置样例" />
              </div>
            </el-tab-pane>

            <el-tab-pane label="实时录音" name="record">
              <div class="record-area">
                <el-button
                  :type="recording ? 'danger' : 'primary'"
                  :icon="recording ? 'Microphone' : 'Microphone'"
                  size="large"
                  @click="toggleRecord"
                  :loading="recording"
                >
                  {{ recording ? '停止录音' : '开始录音' }}
                </el-button>
                <div v-if="recordedDuration" class="record-duration">
                  已录制 {{ recordedDuration }}s
                </div>
                <div class="record-tip">
                  <p>点击"开始录音"后对着麦克风哼唱旋律</p>
                  <p>支持 5-30 秒的人声哼唱</p>
                </div>
              </div>
            </el-tab-pane>
          </el-tabs>

          <div class="param-bar">
            <el-select v-model="params.model_size" size="small" style="width:110px">
              <el-option label="tiny 模型" value="tiny" />
              <el-option label="small 模型" value="small" />
            </el-select>
            <el-switch v-model="params.denoise" active-text="降噪" size="small" />
            <el-switch v-model="params.robust" active-text="稳健识别" size="small" />
            <el-switch v-model="params.vocal_mode" active-text="人声模式" size="small" />
          </div>

          <div class="action-bar">
            <el-button
              type="primary"
              size="large"
              :loading="loading"
              :disabled="!canRecognize"
              @click="startRecognize"
              style="width:100%"
            >
              {{ loading ? '识别中...' : '开始识别' }}
            </el-button>
          </div>
        </el-card>

        <!-- 识别状态 -->
        <el-card v-if="loading" shadow="never" class="m2s-card">
          <div class="progress-info">
            <el-progress :percentage="progress" :stroke-width="8" />
            <div class="progress-text">{{ progressText }}</div>
          </div>
        </el-card>
      </div>

      <!-- 右侧：结果区 -->
      <div class="m2s-right">
        <el-card v-if="!result" shadow="never" class="m2s-card m2s-empty">
          <el-empty description="上传音频或选择样例，点击"开始识别"" />
        </el-card>

        <template v-if="result">
          <!-- 概要信息 -->
          <el-card shadow="never" class="m2s-card">
            <template #header>
              <span class="card-title">识别概要</span>
              <el-tag size="small" type="info">{{ result.source }}</el-tag>
            </template>
            <div class="summary-grid">
              <div class="summary-item">
                <span class="label">调式</span>
                <span class="value">{{ result.key?.tonic || '?' }} {{ result.key?.mode || '?' }}</span>
              </div>
              <div class="summary-item">
                <span class="label">速度</span>
                <span class="value">{{ result.bpm }} BPM</span>
              </div>
              <div class="summary-item">
                <span class="label">音符数</span>
                <span class="value">{{ result.note_count }}</span>
              </div>
              <div class="summary-item">
                <span class="label">时长</span>
                <span class="value">{{ result.duration_sec }}s</span>
              </div>
              <div class="summary-item">
                <span class="label">置信度</span>
                <span class="value">{{ (result.confidence * 100).toFixed(1) }}%</span>
              </div>
              <div class="summary-item">
                <span class="label">音高后端</span>
                <span class="value">{{ result.backend }}</span>
              </div>
              <div class="summary-item">
                <span class="label">稳健识别</span>
                <span class="value">{{ result.robust_runs }} 次 (保留 {{ result.robust_kept }})</span>
              </div>
              <div class="summary-item">
                <span class="label">性能</span>
                <span class="value">{{ result.perf?.preprocess_ms || 0 }}/{{ result.perf?.pitch_ms || 0 }}/{{ result.perf?.parse_ms || 0 }} ms</span>
              </div>
            </div>
          </el-card>

          <!-- 简谱 -->
          <el-card shadow="never" class="m2s-card">
            <template #header>
              <span class="card-title">简谱</span>
              <el-button size="small" text @click="copyJianpu">复制</el-button>
            </template>
            <pre class="jianpu-display">{{ result.jianpu }}</pre>
          </el-card>

          <!-- 五线谱（VexFlow 渲染） -->
          <el-card shadow="never" class="m2s-card">
            <template #header>
              <span class="card-title">五线谱</span>
              <div class="score-controls">
                <el-button size="small" :icon="'VideoPlay'" @click="playMidi" :disabled="!result.midi_sequence?.length">
                  {{ playing ? '播放中...' : '播放' }}
                </el-button>
                <el-button size="small" @click="exportSheet('png')" :icon="'Picture'">导出 PNG</el-button>
                <el-button size="small" @click="exportSheet('pdf')" :icon="'Document'">导出 PDF</el-button>
                <el-button size="small" @click="exportSheet('svg')" :icon="'PictureFilled'">导出 SVG</el-button>
              </div>
            </template>
            <div class="vexflow-container" ref="vexflowRef">
              <div v-if="vexflowError" class="vexflow-error">
                {{ vexflowError }}
              </div>
              <div id="vexflow-output" class="vexflow-output"></div>
            </div>
          </el-card>

          <!-- 音符明细 -->
          <el-card shadow="never" class="m2s-card">
            <template #header>
              <span class="card-title">音符明细</span>
              <el-button size="small" text @click="saveReport">保存报告</el-button>
            </template>
            <el-table :data="result.notes || []" size="small" max-height="300" stripe>
              <el-table-column type="index" label="#" width="50" />
              <el-table-column prop="midi" label="MIDI" width="70" />
              <el-table-column prop="name" label="音名" width="80" />
              <el-table-column prop="start" label="起始(s)" width="90" />
              <el-table-column prop="end" label="结束(s)" width="90" />
              <el-table-column prop="dur" label="时长(s)" width="90" />
            </el-table>
          </el-card>
        </template>
      </div>
    </div>
  </div>
</template>

<script>
import { ref, reactive, computed, onMounted, nextTick, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { UploadFilled } from '@element-plus/icons-vue'
import axios from 'axios'

// VexFlow 动态加载
let VF = null
async function loadVexFlow() {
  if (VF) return VF
  try {
    const mod = await import('vexflow')
    VF = mod.Flow || mod
    return VF
  } catch (e) {
    console.warn('VexFlow 加载失败，五线谱渲染不可用:', e)
    return null
  }
}

export default {
  name: 'Melody2ScoreView',
  components: { UploadFilled },
  setup() {
    const inputMode = ref('upload')
    const loading = ref(false)
    const progress = ref(0)
    const progressText = ref('')
    const recording = ref(false)
    const recordedDuration = ref(0)
    const pyStatus = ref('offline')
    const samples = ref([])
    const selectedSample = ref(null)
    const selectedTimbre = ref('')
    const result = ref(null)
    const uploadRef = ref(null)
    const vexflowRef = ref(null)
    const vexflowError = ref('')
    const playing = ref(false)
    const audioContext = ref(null)
    const uploadFile = ref(null)
    let mediaRecorder = null
    let audioChunks = []
    let recordTimer = null

    const params = reactive({
      model_size: 'tiny',
      denoise: true,
      robust: true,
      vocal_mode: true
    })

    const canRecognize = computed(() => {
      if (loading.value) return false
      if (inputMode.value === 'upload') return !!uploadFile.value
      if (inputMode.value === 'sample') return selectedSample.value !== null
      if (inputMode.value === 'record') return recordedDuration.value > 0
      return false
    })

    // 健康检查
    const checkHealth = async () => {
      try {
        const res = await axios.get('/api/melody2score/health', { timeout: 5000 })
        if (res.data?.status === 'ok') {
          pyStatus.value = 'online'
          ElMessage.success('旋律转谱引擎在线')
        }
      } catch (e) {
        pyStatus.value = 'offline'
        ElMessage.warning('旋律转谱引擎离线，请启动 Python 服务')
      }
    }

    // 刷新样例
    const refreshSamples = async () => {
      try {
        const res = await axios.get('/api/melody2score/samples', { timeout: 10000 })
        samples.value = res.data?.data || res.data || []
      } catch (e) {
        console.warn('获取样例列表失败:', e)
      }
    }

    // 选择样例
    const selectSample = (g) => {
      selectedSample.value = g.melody_index
      if (g.timbres?.length) {
        selectedTimbre.value = g.timbres[0].timbre
      }
    }

    const selectTimbre = (g, t) => {
      selectedSample.value = g.melody_index
      selectedTimbre.value = t.timbre
    }

    // 文件选择
    const onFileChange = (file) => {
      uploadFile.value = file.raw
    }

    const onInputModeChange = () => {
      uploadFile.value = null
      selectedSample.value = null
      recordedDuration.value = 0
    }

    // 录音
    const toggleRecord = async () => {
      if (recording.value) {
        stopRecording()
        return
      }
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
        mediaRecorder = new MediaRecorder(stream, { mimeType: 'audio/webm;codecs=opus' })
        audioChunks = []
        let startTime = Date.now()

        mediaRecorder.ondataavailable = (e) => {
          if (e.data.size > 0) audioChunks.push(e.data)
        }

        mediaRecorder.onstop = async () => {
          stream.getTracks().forEach(t => t.stop())
          recordedDuration.value = Math.round((Date.now() - startTime) / 1000)
          const blob = new Blob(audioChunks, { type: 'audio/webm' })
          // 转换为 WAV 格式
          const wavBlob = await convertToWav(blob)
          uploadFile.value = new File([wavBlob], 'recording.wav', { type: 'audio/wav' })
          inputMode.value = 'upload'
          ElMessage.success(`录音完成: ${recordedDuration.value}s`)
        }

        mediaRecorder.start()
        recording.value = true
        recordTimer = setInterval(() => {
          recordedDuration.value = Math.round((Date.now() - startTime) / 1000)
        }, 1000)
      } catch (e) {
        ElMessage.error('无法访问麦克风: ' + e.message)
      }
    }

    const stopRecording = () => {
      if (mediaRecorder && mediaRecorder.state !== 'inactive') {
        mediaRecorder.stop()
      }
      recording.value = false
      if (recordTimer) {
        clearInterval(recordTimer)
        recordTimer = null
      }
    }

    // WebM 转 WAV
    const convertToWav = async (webmBlob) => {
      const arrayBuffer = await webmBlob.arrayBuffer()
      const audioCtx = new (window.AudioContext || window.webkitAudioContext)()
      const audioBuffer = await audioCtx.decodeAudioData(arrayBuffer)
      const numChannels = audioBuffer.numberOfChannels
      const sampleRate = audioBuffer.sampleRate
      const length = audioBuffer.length
      const wavBuffer = audioBuffer.getChannelData(0)

      // WAV 编码
      const buffer = new ArrayBuffer(44 + length * 2)
      const view = new DataView(buffer)
      const writeString = (offset, str) => {
        for (let i = 0; i < str.length; i++) view.setUint8(offset + i, str.charCodeAt(i))
      }
      writeString(0, 'RIFF')
      view.setUint32(4, 36 + length * 2, true)
      writeString(8, 'WAVE')
      writeString(12, 'fmt ')
      view.setUint32(16, 16, true)
      view.setUint16(20, 1, true)
      view.setUint16(22, 1, true)
      view.setUint32(24, sampleRate, true)
      view.setUint32(28, sampleRate * 2, true)
      view.setUint16(32, 2, true)
      view.setUint16(34, 16, true)
      writeString(36, 'data')
      view.setUint32(40, length * 2, true)
      for (let i = 0; i < length; i++) {
        const s = Math.max(-1, Math.min(1, wavBuffer[i]))
        view.setInt16(44 + i * 2, s < 0 ? s * 0x8000 : s * 0x7FFF, true)
      }
      return new Blob([buffer], { type: 'audio/wav' })
    }

    // 开始识别
    const startRecognize = async () => {
      loading.value = true
      progress.value = 10
      progressText.value = '准备音频...'
      result.value = null
      vexflowError.value = ''

      try {
        let formData = new FormData()
        formData.append('model_size', params.model_size)
        formData.append('denoise', params.denoise ? 'true' : 'false')
        formData.append('robust', params.robust ? 'true' : 'false')
        formData.append('vocal_mode', params.vocal_mode ? 'true' : 'false')

        if (inputMode.value === 'upload' && uploadFile.value) {
          formData.append('file', uploadFile.value)
          progressText.value = '上传音频...'
          progress.value = 20
          const res = await axios.post('/api/melody2score/recognize', formData, {
            timeout: 120000,
            headers: { 'Content-Type': 'multipart/form-data' }
          })
          result.value = res.data?.data || res.data
        } else if (inputMode.value === 'sample' && selectedSample.value !== null) {
          const sample = samples.value.find(g => g.melody_index === selectedSample.value)
          const timbre = sample?.timbres?.find(t => t.timbre === selectedTimbre.value)
          if (!timbre) throw new Error('未选择有效样例音色')
          formData.append('name', timbre.file)
          progressText.value = `识别样例: ${sample?.title_zh || ''}...`
          progress.value = 30
          const res = await axios.post('/api/melody2score/recognize-sample', formData, {
            timeout: 120000,
            headers: { 'Content-Type': 'multipart/form-data' }
          })
          result.value = res.data?.data || res.data
        }

        progress.value = 80
        progressText.value = '渲染乐谱...'
        await nextTick()
        // 延迟一下等待 DOM 更新
        setTimeout(() => renderVexflow(), 200)
        progress.value = 100
        progressText.value = '识别完成'
        ElMessage.success('识别成功')
      } catch (e) {
        ElMessage.error('识别失败: ' + (e.response?.data?.error || e.message))
        progressText.value = '识别失败'
      } finally {
        loading.value = false
      }
    }

    // VexFlow 渲染
    const renderVexflow = async () => {
      vexflowError.value = ''
      const container = document.getElementById('vexflow-output')
      if (!container) return

      const vfData = result.value?.vexflow_data
      if (!vfData || !vfData.notes?.length) {
        vexflowError.value = '无乐谱数据可渲染'
        return
      }

      const VFModule = await loadVexFlow()
      if (!VFModule) {
        vexflowError.value = 'VexFlow 库加载失败，请检查网络连接'
        return
      }

      try {
        container.innerHTML = ''
        const { Renderer, Stave, StaveNote, Voice, Formatter, Accidental, Beam, Dot } = VFModule

        const renderer = new Renderer(container, Renderer.Backends.SVG)
        const width = Math.max(container.clientWidth || 800, 600)
        const notes = vfData.notes
        const numNotes = notes.length
        const maxNotesPerStave = 16
        const numStaves = Math.ceil(numNotes / maxNotesPerStave)
        const staveHeight = 120
        const totalHeight = Math.max(staveHeight * numStaves + 40, 150)

        renderer.resize(width, totalHeight)
        const ctx = renderer.getContext()

        // 调号映射
        const keySigMap = {
          'C': 0, 'G': 1, 'D': 2, 'A': 3, 'E': 4, 'B': 5, 'F#': 6, 'C#': 7,
          'F': -1, 'Bb': -2, 'Eb': -3, 'Ab': -4, 'Db': -5, 'Gb': -6, 'Cb': -7
        }
        const keySig = keySigMap[vfData.key_signature] || 0

        // 时值映射：VexFlow 使用 duration string
        const durMap = {
          'w': 'w', 'h': 'h', 'hd': 'hd', 'q': 'q', 'qd': 'qd',
          '8': '8', '8d': '8d', '16': '16', '16d': '16d', '32': '32'
        }

        // 按行渲染
        for (let s = 0; s < numStaves; s++) {
          const y = 30 + s * staveHeight
          const stave = new Stave(20, y, width - 40)
          stave.addClef('treble')
          stave.addKeySignature(vfData.key_signature || 'C')
          stave.addTimeSignature(vfData.time_signature || '4/4')
          stave.setContext(ctx).draw()

          const startIdx = s * maxNotesPerStave
          const endIdx = Math.min(startIdx + maxNotesPerStave, numNotes)
          const staveNotes = notes.slice(startIdx, endIdx)

          const vexNotes = []
          for (const n of staveNotes) {
            const dur = durMap[n.duration] || 'q'
            const noteParts = [n.note]
            const staveNote = new StaveNote({
              keys: noteParts,
              duration: dur,
              clef: 'treble',
              auto_stem: true
            })
            vexNotes.push(staveNote)
          }

          if (vexNotes.length === 0) continue

          const voice = new Voice({ num_beats: 4, beat_value: 4 })
          voice.addTickables(vexNotes)
          new Formatter().joinVoices([voice]).format([voice], width - 80)
          voice.draw(ctx, stave)
        }
      } catch (e) {
        console.error('VexFlow 渲染错误:', e)
        vexflowError.value = '五线谱渲染失败: ' + e.message
      }
    }

    // MIDI 播放
    const playMidi = async () => {
      const midiSeq = result.value?.midi_sequence
      if (!midiSeq?.length) return

      playing.value = true
      try {
        if (!audioContext.value) {
          audioContext.value = new (window.AudioContext || window.webkitAudioContext)()
        }
        const ctx = audioContext.value
        const bpm = result.value?.bpm || 120
        const beatDuration = 60.0 / bpm
        const gainNode = ctx.createGain()
        gainNode.gain.value = 0.3
        gainNode.connect(ctx.destination)

        const now = ctx.currentTime
        for (let i = 0; i < midiSeq.length; i++) {
          const freq = 440 * Math.pow(2, (midiSeq[i] - 69) / 12)
          const startTime = now + i * beatDuration * 0.5
          const osc = ctx.createOscillator()
          osc.type = 'sine'
          osc.frequency.value = freq
          osc.connect(gainNode)
          osc.start(startTime)
          osc.stop(startTime + beatDuration * 0.45)
        }
      } catch (e) {
        console.warn('MIDI 播放失败:', e)
      }
      setTimeout(() => { playing.value = false }, (midiSeq.length * 60.0 / (result.value?.bpm || 120) * 0.5 + 0.5) * 1000)
    }

    // 复制简谱
    const copyJianpu = () => {
      if (!result.value?.jianpu) return
      navigator.clipboard.writeText(result.value.jianpu).then(() => {
        ElMessage.success('简谱已复制到剪贴板')
      }).catch(() => {
        ElMessage.warning('复制失败，请手动复制')
      })
    }

    // 导出歌谱
    const exportSheet = async (format) => {
      if (!result.value) return
      try {
        const res = await axios.post('/api/melody2score/export-sheet', {
          result: result.value,
          title: result.value.source || '未命名旋律',
          format
        }, { timeout: 60000 })
        const data = res.data?.data || res.data
        if (data?.file) {
          const url = `/api/melody2score/download/${encodeURIComponent(data.file)}`
          window.open(url, '_blank')
          ElMessage.success(`歌谱已导出 (${format})`)
        }
      } catch (e) {
        ElMessage.error('导出失败: ' + (e.response?.data?.error || e.message))
      }
    }

    // 保存报告
    const saveReport = async () => {
      if (!result.value) return
      try {
        const res = await axios.post('/api/melody2score/save-report', {
          result: result.value,
          title: result.value.source || '未命名旋律',
          source: result.value.source || '用户上传'
        }, { timeout: 30000 })
        const data = res.data?.data || res.data
        if (data?.file) {
          ElMessage.success(`报告已保存: ${data.file}`)
        }
      } catch (e) {
        ElMessage.error('保存报告失败: ' + (e.response?.data?.error || e.message))
      }
    }

    onMounted(() => {
      checkHealth()
      refreshSamples()
    })

    // 监听结果变化，重渲染五线谱
    watch(result, () => {
      if (result.value) {
        nextTick(() => renderVexflow())
      }
    })

    return {
      inputMode, loading, progress, progressText, recording, recordedDuration,
      pyStatus, samples, selectedSample, selectedTimbre, result,
      uploadRef, vexflowRef, vexflowError, playing, params, canRecognize,
      checkHealth, refreshSamples, selectSample, selectTimbre,
      onFileChange, onInputModeChange, toggleRecord, startRecognize,
      renderVexflow, playMidi, copyJianpu, exportSheet, saveReport
    }
  }
}
</script>

<style scoped>
.melody2score {
  height: 100%;
  display: flex;
  flex-direction: column;
  padding: 16px;
  background: var(--el-bg-color-page);
  color: var(--el-text-color-primary);
  overflow-y: auto;
}

.m2s-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  padding: 12px 16px;
  background: var(--el-bg-color);
  border-radius: 12px;
  box-shadow: 0 1px 3px rgba(0,0,0,0.06);
}
.m2s-title {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 18px;
  font-weight: 600;
}
.m2s-icon {
  font-size: 24px;
  color: var(--el-color-primary);
}
.m2s-actions {
  display: flex;
  gap: 8px;
}

.m2s-layout {
  display: grid;
  grid-template-columns: 380px 1fr;
  gap: 16px;
  flex: 1;
  min-height: 0;
}

.m2s-left {
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
}
.m2s-right {
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
}

.m2s-card {
  border-radius: 10px;
  border: 1px solid var(--el-border-color-light);
}
.m2s-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 400px;
}

.card-title {
  font-size: 15px;
  font-weight: 600;
}

.m2s-upload {
  :deep(.el-upload-dragger) {
    width: 100%;
    padding: 20px;
  }
}

.sample-list {
  max-height: 320px;
  overflow-y: auto;
}
.sample-item {
  padding: 10px 12px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.2s;
  margin-bottom: 6px;
  border: 1px solid transparent;
}
.sample-item:hover {
  background: var(--el-fill-color-light);
}
.sample-item.active {
  border-color: var(--el-color-primary);
  background: var(--el-color-primary-light-9);
}
.sample-title {
  font-size: 14px;
  font-weight: 500;
  margin-bottom: 6px;
}
.sample-en {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
.sample-timbres {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

.record-area {
  text-align: center;
  padding: 20px;
}
.record-duration {
  margin-top: 12px;
  font-size: 24px;
  font-weight: 600;
  color: var(--el-color-primary);
}
.record-tip {
  margin-top: 16px;
  color: var(--el-text-color-secondary);
  font-size: 12px;
  line-height: 1.8;
}

.param-bar {
  display: flex;
  gap: 8px;
  align-items: center;
  padding: 10px 0;
  flex-wrap: wrap;
}
.action-bar {
  padding-top: 4px;
}

.progress-info {
  padding: 8px 0;
}
.progress-text {
  text-align: center;
  margin-top: 8px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.summary-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
}
.summary-item {
  padding: 8px 10px;
  background: var(--el-fill-color-lighter);
  border-radius: 8px;
}
.summary-item .label {
  display: block;
  font-size: 11px;
  color: var(--el-text-color-secondary);
  margin-bottom: 4px;
}
.summary-item .value {
  font-size: 14px;
  font-weight: 600;
}

.jianpu-display {
  font-family: 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.6;
  padding: 12px;
  background: var(--el-fill-color-lighter);
  border-radius: 6px;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 200px;
  overflow-y: auto;
}

.score-controls {
  display: flex;
  gap: 6px;
}

.vexflow-container {
  min-height: 150px;
  position: relative;
}
.vexflow-output {
  min-height: 120px;
}
.vexflow-error {
  padding: 30px;
  text-align: center;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

@media (max-width: 900px) {
  .m2s-layout {
    grid-template-columns: 1fr;
  }
  .summary-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
</styl