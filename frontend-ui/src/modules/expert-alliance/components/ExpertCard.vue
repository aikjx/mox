<template>
  <article class="ae-card" :class="{ 'is-favorite': favorite }">
    <header class="ae-card-head">
      <div class="ae-avatar" :title="expert.name">{{ initial }}</div>
      <div class="ae-headline">
        <div class="ae-name-row">
          <h3 class="ae-name">{{ expert.name || '(未命名专家)' }}</h3>
          <el-tag v-if="verificationTag" :type="verificationTag.type" size="small" effect="plain">{{ verificationTag.text }}</el-tag>
        </div>
        <p class="ae-role">{{ [expert.title, expert.organization].filter(Boolean).join(' · ') || '暂无头衔' }}</p>
        <div class="ae-presence">
          <span class="ae-dot" :class="`is-${expert.availability.status || 'offline'}`"></span>
          <span>{{ availabilityLabel(expert.availability.status) }}</span>
          <span v-if="expert.availability.avgResponseMinutes" class="ae-resp">
            平均响应 {{ Math.round(expert.availability.avgResponseMinutes) }} 分钟
          </span>
        </div>
      </div>
      <button class="ae-star" :class="{ on: favorite }" :title="starTitle" @click="emit('favorite', expert)">
        <el-icon><StarFilled v-if="favorite" /><Star v-else /></el-icon>
      </button>
    </header>

    <p class="ae-bio">{{ expert.bio || '暂无简介' }}</p>

    <div class="ae-chips">
      <span v-for="d in expert.domains.slice(0, 3)" :key="d" class="ae-chip is-domain">{{ d }}</span>
      <span v-for="s in expert.skills.slice(0, 3)" :key="s" class="ae-chip">{{ s }}</span>
      <span v-if="overflow" class="ae-chip is-more">+{{ overflow }}</span>
    </div>

    <dl class="ae-metrics">
      <div class="ae-metric">
        <dt>评分</dt>
        <dd>{{ ratingText }}</dd>
      </div>
      <div class="ae-metric">
        <dt>累计咨询</dt>
        <dd>{{ expert.metrics.totalConsultations }}</dd>
      </div>
      <div class="ae-metric">
        <dt>解决率</dt>
        <dd>{{ percentText(expert.metrics.resolutionRate) }}</dd>
      </div>
      <div class="ae-metric">
        <dt>计费</dt>
        <dd>{{ pricing }}</dd>
      </div>
    </dl>

    <div class="ae-load">
      <span class="ae-load-label">并发负载</span>
      <el-progress
        v-if="expert.availability.loadRatio !== null"
        class="ae-load-bar"
        :percentage="clampPercent(expert.availability.loadRatio * 100)"
        :stroke-width="6"
        :color="loadColor"
      />
      <span class="ae-load-text">{{ loadText }}</span>
    </div>

    <footer class="ae-actions">
      <el-button size="small" text :icon="View" @click="emit('view', expert)">详情</el-button>
      <el-button size="small" :icon="Calendar" @click="emit('book', expert)">预约</el-button>
      <el-tooltip :content="consultHint" placement="top" :disabled="expert.online">
        <span>
          <el-button size="small" type="primary" :icon="ChatDotRound" :disabled="!expert.online" @click="emit('consult', expert)">
            即时咨询
          </el-button>
        </span>
      </el-tooltip>
    </footer>
  </article>
</template>

<script setup>
import { computed } from 'vue'
import { Calendar, ChatDotRound, Star, StarFilled, View } from '@element-plus/icons-vue'
import {
  VERIFICATION_STATUS, availabilityLabel, expertTypeLabel, pricingText, verificationLabel
} from '@/modules/expert-alliance/contract'

const props = defineProps({
  expert: { type: Object, required: true },
  favorite: { type: Boolean, default: false }
})
const emit = defineEmits(['view', 'book', 'consult', 'favorite'])

const initial = computed(() => (props.expert.name || '?').slice(0, 1))
const overflow = computed(() => Math.max(0, props.expert.domains.length + props.expert.skills.length - 6))

const VERIFICATION_TAG = {
  [VERIFICATION_STATUS.CERTIFIED]: { type: 'success', text: '官方认证' },
  [VERIFICATION_STATUS.VERIFIED]: { type: 'info', text: '已认证' },
  [VERIFICATION_STATUS.UNVERIFIED]: { type: 'warning', text: '未认证' }
}
const verificationTag = computed(() => VERIFICATION_TAG[props.expert.verificationStatus] ?? null)

// rating_count 为 0 时后端 avg_rating 是默认 0，不能显示成"0 分"
const ratingText = computed(() => (
  props.expert.metrics.ratingCount > 0 ? `${props.expert.metrics.avgRating.toFixed(1)} / 5` : '暂无评分'
))
const pricing = computed(() => pricingText(props.expert.pricingModel, props.expert.hourlyRateCents))
const loadText = computed(() => {
  const a = props.expert.availability
  return a.maxConcurrent > 0 ? `${a.currentLoad} / ${a.maxConcurrent}` : `${a.currentLoad} · 未设上限`
})
const loadColor = computed(() => {
  const r = props.expert.availability.loadRatio
  if (r === null) return 'var(--text-muted)'
  if (r >= 0.8) return 'var(--danger)'
  if (r >= 0.5) return 'var(--warning)'
  return 'var(--success)'
})
const consultHint = computed(() => {
  const type = expertTypeLabel(props.expert.expertType)
  return `仅在线专家可即时接入（当前：${availabilityLabel(props.expert.availability.status)} · ${type}）`
})
const starTitle = computed(() => (props.favorite ? '取消收藏' : '收藏专家（收藏状态仅保留在本次会话）'))

const percentText = (v) => (v > 0 ? `${Math.round(v * 100)}%` : '—')
const clampPercent = (v) => Math.max(0, Math.min(100, Math.round(Number(v) || 0)))
</script>

<style scoped>
.ae-card {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  transition: border-color .15s ease, transform .15s ease;
}
.ae-card:hover {
  border-color: var(--accent);
  transform: translateY(-1px);
}
.ae-card.is-favorite {
  border-color: var(--warning);
}
.ae-card-head {
  display: flex;
  gap: 10px;
}
.ae-avatar {
  width: 42px;
  height: 42px;
  flex-shrink: 0;
  border-radius: var(--radius-md);
  display: grid;
  place-items: center;
  font-size: 17px;
  font-weight: 600;
  color: var(--on-brand);
  background: linear-gradient(135deg, var(--brand-fill), var(--brand-fill-hover));
}
.ae-headline {
  flex: 1;
  min-width: 0;
}
.ae-name-row {
  display: flex;
  align-items: center;
  gap: 6px;
}
.ae-name {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ae-role {
  margin: 2px 0 0;
  font-size: 12px;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ae-presence {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 4px;
  font-size: 11px;
  color: var(--text-muted);
}
.ae-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--text-muted);
}
.ae-dot.is-online { background: var(--success); }
.ae-dot.is-busy { background: var(--warning); }
.ae-dot.is-away { background: var(--text-secondary); }
.ae-dot.is-offline { background: var(--text-muted); opacity: .5; }
.ae-resp { margin-left: auto; }
.ae-star {
  border: 0;
  background: transparent;
  cursor: pointer;
  color: var(--text-muted);
  font-size: 16px;
  padding: 2px;
}
.ae-star.on { color: var(--warning); }
.ae-bio {
  margin: 0;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-secondary);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  min-height: 38px;
}
.ae-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}
.ae-chip {
  padding: 1px 6px;
  border-radius: var(--radius-xs);
  font-size: 11px;
  background: var(--bg-hover);
  color: var(--text-secondary);
}
.ae-chip.is-domain {
  background: var(--accent-dim);
  color: var(--accent-light);
}
.ae-chip.is-more { background: transparent; color: var(--text-muted); }
.ae-metrics {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 6px;
  margin: 0;
  padding: 8px 0;
  border-top: 1px dashed var(--border);
}
.ae-metric dt {
  font-size: 11px;
  color: var(--text-muted);
}
.ae-metric dd {
  margin: 2px 0 0;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ae-load {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
  color: var(--text-muted);
}
.ae-load-bar { flex: 1; }
.ae-load-label { flex-shrink: 0; }
.ae-load-text { flex-shrink: 0; min-width: 52px; text-align: right; }
.ae-actions {
  display: flex;
  justify-content: flex-end;
  gap: 6px;
}
</style>
