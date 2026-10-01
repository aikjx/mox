<template>
  <section class="ar">
    <el-tabs v-model="board" class="ar-tabs">
      <el-tab-pane v-for="b in RANK_BOARDS" :key="b.key" :name="b.key" :label="b.label" />
    </el-tabs>

    <p class="ar-rule">{{ active.rule }}</p>
    <p class="ar-caption">
      样本仅当前页 {{ result.sampleSize }} 位专家，其中 {{ result.eligible }} 位有可排名数据；榜单不是全库排行。
    </p>

    <el-empty v-if="!result.rows.length" :image-size="70" :description="emptyText" />

    <ol v-else class="ar-list">
      <li v-for="row in result.rows" :key="row.id || row.name" class="ar-item">
        <span class="ar-rank" :class="{ 'is-top': row.rank <= 3 }">{{ row.rank }}</span>
        <span class="ar-main">
          <span class="ar-name">{{ row.name }}</span>
          <span class="ar-sub">{{ row.subtitle }}</span>
        </span>
        <span class="ar-metric">
          <b class="ar-value">{{ row.display }}</b>
          <i class="ar-secondary">{{ row.secondary }}</i>
        </span>
      </li>
    </ol>
  </section>
</template>

<script setup>
import { computed, ref } from 'vue'
import { RANK_BOARDS, RANK_BOARD, buildBoard } from '@/modules/expert-alliance/model'

const props = defineProps({
  experts: { type: Array, default: () => [] }
})

const board = ref(RANK_BOARD.CONSULTATIONS)

const active = computed(() => RANK_BOARDS.find((b) => b.key === board.value) || RANK_BOARDS[0])
const result = computed(() => buildBoard(board.value, props.experts))

const EMPTY_TEXT = {
  [RANK_BOARD.CONSULTATIONS]: '本页专家累计咨询次数均为 0，没有真实成交量可排',
  [RANK_BOARD.RATING]: '本页专家还没有一条真实评分，评分榜宁可为空也不拿默认零值凑数',
  [RANK_BOARD.NEWCOMERS]: '后端未回传注册时间，新晋榜无从排起'
}
const emptyText = computed(() => EMPTY_TEXT[board.value] || '没有可排名的专家')
</script>

<style scoped>
.ar { display: flex; flex-direction: column; gap: 10px; }
.ar-tabs { margin-top: -8px; }
.ar-rule { margin: 0; font-size: 12px; color: var(--text-secondary); }
.ar-caption { margin: 0; font-size: 11px; color: var(--text-muted); }
.ar-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.ar-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.ar-rank {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  flex-shrink: 0;
  border-radius: var(--radius-xs);
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  background: var(--bg-tertiary);
}
.ar-rank.is-top {
  color: var(--on-accent);
  background: var(--accent-fill);
}
.ar-main { flex: 1; min-width: 0; display: flex; flex-direction: column; }
.ar-name {
  font-size: 13px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ar-sub {
  font-size: 11px;
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ar-metric { display: flex; flex-direction: column; align-items: flex-end; flex-shrink: 0; }
.ar-value { font-size: 14px; font-weight: 600; color: var(--text-primary); }
.ar-secondary { font-size: 11px; color: var(--text-muted); }
</style>
