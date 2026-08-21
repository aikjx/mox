<template>
  <div class="market">
    <div class="head">
      <div>
        <h2 class="page-title">算子商城</h2>
        <p class="page-subtitle">把「需求 + 业务流程图」作为算子包沉淀下来，别人可随机浏览、克隆后继续编辑</p>
      </div>
      <div class="head-actions">
        <el-button @click="randomOne" :loading="randoming">
          <el-icon><MagicStick /></el-icon> 随机一个
        </el-button>
        <el-button type="primary" @click="showUpload = true">
          <el-icon><Upload /></el-icon> 上传算子包
        </el-button>
      </div>
    </div>

    <!-- 过滤条 -->
    <div class="toolbar">
      <el-input
        v-model="kw"
        placeholder="搜索名称 / 简介"
        clearable
        :prefix-icon="Search"
        style="width: 240px"
      />
      <div class="cats">
        <span
          v-for="c in categories"
          :key="c"
          class="cat"
          :class="{ on: cat === c }"
          @click="cat = c"
        >{{ c === 'all' ? '全部' : c }}</span>
      </div>
    </div>

    <!-- 卡片列表 -->
    <div class="grid grid-cards">
      <div
        v-for="p in filtered"
        :key="p.id"
        class="card"
        @click="openDetail(p.id)"
      >
        <div class="card-top">
          <span class="badge primary">{{ p.category || '未分类' }}</span>
          <span class="ver">v{{ p.version }}</span>
        </div>
        <h3 class="card-name">{{ p.name }}</h3>
        <p class="card-summary">{{ p.summary || '暂无简介' }}</p>
        <div class="card-tags">
          <span v-for="t in p.tags" :key="t" class="tag">{{ t }}</span>
        </div>
        <div class="card-meta">
          <span><el-icon><Share /></el-icon> {{ p.clone_count }} 克隆</span>
          <span><el-icon><Connection /></el-icon> {{ p.node_count }} 节点</span>
          <span><el-icon><List /></el-icon> {{ p.feature_count }} 功能</span>
        </div>
        <div class="card-foot">
          <span class="author">{{ p.author || '匿名' }}</span>
          <el-button size="small" type="primary" plain @click.stop="clonePkg(p.id)">
            <el-icon><CopyDocument /></el-icon> 克隆编辑
          </el-button>
        </div>
      </div>
      <el-empty v-if="!filtered.length" description="商城暂无算子包，点右上角上传第一个吧" :image-size="70" />
    </div>

    <!-- 上传弹窗 -->
    <el-dialog v-model="showUpload" title="上传算子包" width="560px" @closed="resetForm">
      <el-form label-width="84px" :model="form">
        <el-form-item label="名称" required>
          <el-input v-model="form.name" placeholder="算子包名称" />
        </el-form-item>
        <el-form-item label="分类">
          <el-input v-model="form.category" placeholder="如：平台/编排" />
        </el-form-item>
        <el-form-item label="作者">
          <el-input v-model="form.author" placeholder="你的名字" />
        </el-form-item>
        <el-form-item label="简介">
          <el-input v-model="form.summary" type="textarea" :rows="2" placeholder="一句话说明这个算子包解决什么" />
        </el-form-item>
        <el-form-item label="标签">
          <el-select v-model="form.tags" multiple filterable allow-create default-first-option placeholder="可输入新建标签" style="width:100%">
            <el-option v-for="t in tagOptions" :key="t" :label="t" :value="t" />
          </el-select>
        </el-form-item>
        <el-form-item label="需求描述" required>
          <el-input
            v-model="form.requirement"
            type="textarea"
            :rows="5"
            placeholder="★ 这是最核心的部分：把需求写清楚，其他（流程图/功能）都可据此快速调整"
          />
        </el-form-item>
        <el-form-item label="流程图">
          <div class="hint">上传后可进入详情页用可视化编辑器拖拽编排业务流程（节点/连线可编辑）</div>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showUpload = false">取消</el-button>
        <el-button type="primary" :loading="uploading" @click="doUpload">上传</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { Search, Upload, MagicStick, CopyDocument, Share, Connection, List } from '@element-plus/icons-vue'
import { marketList, marketRandom, marketUpload, marketClone } from '@/api'

const router = useRouter()
const packages = ref([])
const kw = ref('')
const cat = ref('all')
const randoming = ref(false)
const uploading = ref(false)
const showUpload = ref(false)

const tagOptions = ['编排', '流程图', '企业级', 'AI', '数据', '算法', '可视化']
const form = ref(emptyForm())

function emptyForm() {
  return {
    name: '', category: '', author: '', summary: '', tags: [], requirement: ''
  }
}
function resetForm() {
  form.value = emptyForm()
}

const categories = computed(() => {
  const set = new Set(packages.value.map((p) => p.category || '未分类'))
  return ['all', ...Array.from(set)]
})
const filtered = computed(() => {
  const k = kw.value.trim().toLowerCase()
  return packages.value.filter((p) => {
    const matchK = !k || p.name.toLowerCase().includes(k) || (p.summary || '').toLowerCase().includes(k)
    const matchC = cat.value === 'all' || (p.category || '未分类') === cat.value
    return matchK && matchC
  })
})

async function load() {
  try {
    const r = await marketList()
    packages.value = r.packages || []
  } catch (e) {
    ElMessage.error('加载商城失败：' + e.message)
  }
}
function openDetail(id) {
  router.push(`/market/${id}`)
}
async function clonePkg(id) {
  try {
    const r = await marketClone(id)
    ElMessage.success('已克隆，进入编辑')
    router.push(`/market/${r.id}`)
  } catch (e) {
    ElMessage.error(e.message)
  }
}
async function randomOne() {
  randoming.value = true
  try {
    const r = await marketRandom()
    if (r.success && r.package) {
      router.push(`/market/${r.package.id}`)
    } else {
      ElMessage.info(r.error || '暂无包')
    }
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    randoming.value = false
  }
}
async function doUpload() {
  if (!form.value.name.trim()) return ElMessage.warning('请填写名称')
  if (!form.value.requirement.trim()) return ElMessage.warning('需求描述不能为空（这是最核心的）')
  uploading.value = true
  try {
    const r = await marketUpload({ ...form.value })
    ElMessage.success('上传成功')
    showUpload.value = false
    await load()
    router.push(`/market/${r.id}`)
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    uploading.value = false
  }
}

onMounted(load)
</script>

<style scoped>
.market { display: flex; flex-direction: column; gap: 16px; }
.head { display: flex; align-items: center; justify-content: space-between; }
.head-actions { display: flex; gap: 10px; }
.toolbar { display: flex; align-items: center; gap: 14px; flex-wrap: wrap; }
.cats { display: flex; gap: 6px; flex-wrap: wrap; }
.cat {
  font-size: 12px; padding: 4px 12px; border-radius: 999px;
  background: var(--bg-page); color: var(--text-2); cursor: pointer; transition: all 0.15s;
}
.cat:hover { color: var(--brand); }
.cat.on { background: var(--brand); color: #fff; }
.grid-cards {
  display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 16px;
}
.card {
  background: #fff; border: 1px solid var(--border); border-radius: 14px; padding: 16px 18px;
  cursor: pointer; transition: all 0.2s; display: flex; flex-direction: column; gap: 10px;
}
.card:hover { box-shadow: 0 12px 30px rgba(15, 23, 42, 0.1); transform: translateY(-2px); border-color: var(--brand-light); }
.card-top { display: flex; justify-content: space-between; align-items: center; }
.ver { font-size: 11px; color: var(--text-3); }
.card-name { font-size: 15px; font-weight: 700; color: var(--text-1); }
.card-summary { font-size: 12px; color: var(--text-3); line-height: 1.6; min-height: 38px; }
.card-tags { display: flex; gap: 5px; flex-wrap: wrap; }
.tag { font-size: 11px; padding: 2px 8px; border-radius: 6px; background: var(--brand-soft); color: var(--brand-dark); }
.card-meta { display: flex; gap: 14px; font-size: 12px; color: var(--text-2); }
.card-meta span { display: inline-flex; align-items: center; gap: 4px; }
.card-foot { display: flex; justify-content: space-between; align-items: center; border-top: 1px solid var(--border-light); padding-top: 10px; }
.author { font-size: 12px; color: var(--text-3); }
.hint { font-size: 12px; color: var(--text-3); line-height: 1.5; }
.badge { font-size: 11px; padding: 2px 9px; border-radius: 999px; }
.badge.primary { background: var(--brand); color: #fff; }
</style>
