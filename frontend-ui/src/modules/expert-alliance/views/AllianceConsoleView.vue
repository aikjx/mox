<template>
  <div class="ac">
    <header class="ac-head">
      <div class="ac-head-text">
        <h1 class="ac-title">联盟控制台</h1>
        <p class="ac-sub">专家联盟任务的创建、编排、执行与融合结果 · 契约对齐网关 :3080</p>
      </div>
      <div class="ac-head-actions">
        <el-tag :type="runtimeTag.type" effect="plain" size="small">{{ runtimeTag.text }}</el-tag>
        <el-button :icon="Refresh" :loading="store.loading.runtime" @click="reload">刷新</el-button>
        <el-button :icon="MagicStick" @click="goCollab">智能协作</el-button>
        <el-button type="primary" :icon="Plus" @click="createVisible = true">新建任务</el-button>
      </div>
    </header>

    <el-alert
      v-if="store.runtime?.simulated"
      class="ac-alert"
      type="warning"
      show-icon
      :closable="false"
      title="本地预览模式"
      :description="runtimeHint"
    />
    <el-alert
      v-if="store.error.tasks"
      class="ac-alert"
      type="error"
      show-icon
      :closable="false"
      title="任务列表加载失败"
      :description="store.error.tasks"
    />

    <section class="ack-card">
      <div class="ack-head">
        <h2 class="ack-title">调度器配置</h2>
        <el-tag :type="configTag.type" effect="plain" size="small">{{ configTag.text }}</el-tag>
        <span class="ack-note">网关内存态 · PUT 为合并式更新，只发改动过的键</span>
        <el-button
          v-role-any="['super_admin', 'tenant_admin']"
          class="ack-save"
          size="small"
          type="primary"
          :loading="store.loading.config"
          :disabled="!store.configChanged || !!store.configPatch.problem"
          @click="saveConfig"
        >保存改动</el-button>
      </div>
      <p v-if="store.error.config" class="ack-err">{{ store.error.config }}</p>
      <div class="ack-grid">
        <label v-for="row in store.configRows" :key="row.key" class="ack-field">
          <span class="ack-label">
            {{ row.label }}
            <i v-if="!row.present" class="ack-absent">后端未返回，显示契约默认值</i>
          </span>
          <el-select
            v-if="row.kind === 'select'"
            class="ack-input"
            :model-value="store.configDraft[row.key]"
            size="small"
            :disabled="store.loading.config"
            @update:model-value="(v) => store.setConfigValue(row.key, v)"
          >
            <el-option v-for="s in DISPATCH_STRATEGY" :key="s.value" :label="s.label" :value="s.value" />
          </el-select>
          <el-switch
            v-else-if="row.kind === 'switch'"
            :model-value="store.configDraft[row.key]"
            size="small"
            :disabled="store.loading.config"
            @update:model-value="(v) => store.setConfigValue(row.key, v)"
          />
          <el-input-number
            v-else
            class="ack-input"
            :model-value="store.configDraft[row.key]"
            :min="row.min"
            :max="row.max"
            :step="row.step"
            size="small"
            :disabled="store.loading.config"
            @update:model-value="(v) => store.setConfigValue(row.key, v)"
          />
          <em class="ack-hint">{{ row.key }} · {{ store.configInvalid[row.key] || row.hint }}</em>
        </label>
      </div>

      <div class="ack-run">
        <div class="ack-run-head">
          <h3 class="ack-run-title">分发实跑</h3>
          <span class="ack-run-note">
            {{ RUN_ENDPOINT.method }} {{ RUN_ENDPOINT.path }} · 走与上面同一份配置的策略引擎；
            每次实跑会写一条调度记录并发审计事件（进程内，重启即失），但不会改动任何专家的 current_load
          </span>
          <el-button
            class="ack-run-go"
            size="small"
            :loading="store.loading.dispatch"
            :disabled="!runReady"
            @click="runOnce"
          >实跑一次</el-button>
        </div>
        <div class="ack-run-form">
          <label class="ack-run-field">
            <span class="ack-run-label">需求描述（唯一真正参与匹配算分的输入）</span>
            <el-input v-model="runForm.input" size="small" placeholder="例：前端 架构 性能优化" />
            <em class="ack-run-hint">后端按空白 / 逗号 / 、 / 切词做子串包含匹配，且不按类型路由（task_type 收下不用），所以词比长句有效</em>
          </label>
          <label class="ack-run-field">
            <span class="ack-run-label">任务类型（必填但不参与选人）</span>
            <el-input v-model="runForm.taskType" size="small" placeholder="例：code_review" />
            <em class="ack-run-hint">该键无 serde 缺省，不发会被整段 JSON 拒绝，故始终随体发出</em>
          </label>
          <label class="ack-run-field">
            <span class="ack-run-label">指定专家（可选，非空即走 specified 分支）</span>
            <el-select
              v-model="runForm.expertIds"
              size="small"
              multiple
              filterable
              clearable
              :loading="store.loading.candidates"
              @visible-change="onCandidatesOpen"
              :placeholder="store.error.candidates || '不指定则由策略选人'"
            >
              <el-option v-for="c in store.dispatchCandidates" :key="c.id" :label="`${c.name}（${c.id}）`" :value="c.id" />
            </el-select>
            <em class="ack-run-hint">{{ store.error.candidates ? '候选取不到，可不指定直接实跑' : `候选来自 ${LIST_ENDPOINT.path} 的在线专家，后端还会再查熔断与并发上限` }}</em>
          </label>
        </div>
        <p v-if="store.error.dispatch" class="ack-err">{{ store.error.dispatch }}</p>
        <div v-if="store.dispatchResult" class="ack-run-out">
          <div class="ack-run-meta">
            <el-tag :type="store.dispatchResult.strategyUsed === store.dispatcherConfig?.strategy ? 'success' : 'warning'" size="small" effect="light">
              strategy_used={{ store.dispatchResult.strategyUsed }}
            </el-tag>
            <span class="ack-run-id">{{ store.dispatchResult.dispatchId }} · {{ store.dispatchResult.status }} · {{ store.dispatchResult.createdAt }}</span>
          </div>
          <ul class="ack-run-list">
            <li v-for="a in store.dispatchResult.assigned" :key="a.id" class="ack-run-item">
              <span class="ack-run-name">{{ expertNameOr(a, '（注册表里没有名字）') }}</span>
              <span class="ack-run-score">匹配 {{ a.matchScore.toFixed(3) }}</span>
              <span class="ack-run-load">负载率 {{ percent(a.loadRatio) }}</span>
              <code class="ack-run-mid">{{ a.id }}</code>
            </li>
          </ul>
          <ul class="ack-run-findings">
            <li v-for="(f, i) in store.dispatchFindings" :key="i" class="ack-run-finding" :data-tone="f.tone">{{ f.text }}</li>
          </ul>
        </div>
      </div>
    </section>

    <section class="acks-card">
      <div class="acks-head">
        <h2 class="acks-title">调度状态与负载重置</h2>
        <span class="acks-note">
          {{ STATUS_ENDPOINT.method }} {{ STATUS_ENDPOINT.path }} · 记录数读进程内 dispatch_records（重启归零），负载读注册表内存态
        </span>
        <el-button class="acks-reload" size="small" :icon="Refresh" :loading="store.loading.status" @click="store.loadDispatcherStatus()">
          重取状态
        </el-button>
      </div>
      <p v-if="store.error.status" class="acks-err">{{ store.error.status }}</p>
      <p v-else-if="!store.dispatcherStatus" class="acks-absent">
        没有拿到状态读数。两个重置入口在拿到读数前不可用——看不到"当前是多少"，就无法判断这次归零改了什么，只剩后端自报的一个布尔。
      </p>
      <template v-else>
        <div class="acks-kpi">
          <div class="acks-cell">
            <span class="acks-cell-label">引擎状态</span>
            <b class="acks-cell-value">{{ store.dispatcherStatus.engineStatus || '—' }}</b>
            <em class="acks-cell-hint">后端把 engine_status 写死为「{{ DISPATCH_ENGINE_STATUS_LITERAL }}」（experts_dispatcher.rs:536），这不是探活结果</em>
          </div>
          <div class="acks-cell">
            <span class="acks-cell-label">当前策略</span>
            <b class="acks-cell-value">{{ store.dispatcherStatus.currentStrategy || '—' }}</b>
            <em class="acks-cell-hint">与上方配置卡读同一份 dispatcher_config 内存态，可互相印证</em>
          </div>
          <div class="acks-cell">
            <span class="acks-cell-label">调度记录</span>
            <b class="acks-cell-value">{{ store.dispatcherStatus.totalDispatches }} 条 · 进行中 {{ store.dispatcherStatus.activeDispatches }}</b>
            <em class="acks-cell-hint">{{ store.successNote }}</em>
          </div>
          <div class="acks-cell">
            <span class="acks-cell-label">成功率 / 平均耗时</span>
            <b class="acks-cell-value">{{ percent(store.dispatcherStatus.successRate) }} · {{ store.dispatcherStatus.avgDispatchMs.toFixed(1) }} ms</b>
            <em class="acks-cell-hint">耗时只统计带 completed_at 的记录（:444-455），无终态记录时后端给 0.0</em>
          </div>
          <div class="acks-cell">
            <span class="acks-cell-label">最近一次调度</span>
            <b class="acks-cell-value">{{ store.dispatcherStatus.lastDispatchAt || '本次进程内还没有' }}</b>
            <em class="acks-cell-hint">读数时间 {{ store.dispatcherStatus.serverTs || '（后端未回 ts）' }}</em>
          </div>
        </div>

        <div class="acks-block">
          <h3 class="acks-block-title">熔断器</h3>
          <ul v-if="store.dispatcherStatus.circuitBreakers.length" class="acks-cb">
            <li v-for="c in store.dispatcherStatus.circuitBreakers" :key="c.expertId" class="acks-cb-item">
              <code class="acks-mid">{{ c.expertId }}</code>
              <span>失败 {{ c.failureCount }} 次</span>
              <span class="acks-cb-state">{{ breakerStateLabel(c.state) }}</span>
            </li>
          </ul>
          <p v-else class="acks-honest">{{ store.breakerNote }}</p>
        </div>

        <div class="acks-block">
          <h3 class="acks-block-title">专家负载</h3>
          <el-table v-if="store.loadRows.length" :data="store.loadRows" size="small" class="acks-table">
            <el-table-column label="专家">
              <template #default="{ row }">
                <span class="acks-name">{{ row.name || '（目录里没有名字）' }}</span>
                <code class="acks-mid">{{ row.expertId }}</code>
              </template>
            </el-table-column>
            <el-table-column label="current_load / max_concurrent" width="220">
              <template #default="{ row }">
                <span class="acks-load">{{ row.currentLoad }} / {{ row.maxConcurrent || '不限' }}</span>
                <el-progress :percentage="clampPercent(row.loadRatio * 100)" :stroke-width="4" :show-text="false" />
              </template>
            </el-table-column>
            <el-table-column label="动作" width="120" align="right">
              <template #default="{ row }">
                <el-button
                  v-role-any="['super_admin', 'tenant_admin']"
                  size="small"
                  text
                  type="warning"
                  :disabled="store.loading.reset"
                  @click="openReset({ id: row.expertId, name: row.name })"
                >重置负载</el-button>
              </template>
            </el-table-column>
          </el-table>
          <p v-else class="acks-honest">
            状态里没有 enabled 专家：该端点对禁用与软删除者一律跳过（experts_dispatcher.rs:515-517），所以空表的意思是"没有可点名单挑的对象"，不是"全部空闲"。
          </p>
        </div>

        <div class="acks-foot">
          <el-button
            v-role-any="['super_admin', 'tenant_admin']"
            class="acks-foot-btn"
            size="small"
            type="danger"
            plain
            :disabled="!store.dispatcherStatus || store.loading.reset"
            @click="openReset({ all: true })"
          >全量重置</el-button>
          <em class="acks-foot-note">覆盖整张注册表（含停用者），不是这张表里的人数——人数只在回执的 reset_count 里说。</em>
          <el-button
            v-if="store.resetReceipt"
            class="acks-receipt-clear"
            size="small"
            text
            @click="store.resetReceipt = null"
          >收起回执</el-button>
        </div>
        <div v-if="store.resetReceipt" class="acks-receipt">
          <span class="acks-receipt-title">
            最近一次重置回执 · {{ store.resetReceipt.scope === 'all' ? '全量' : store.resetReceipt.expertId }}
            <template v-if="store.resetReceipt.scope === 'one'"> · 重置前负载 {{ store.resetReceipt.previousLoad }}</template>
            <template v-else> · reset_count {{ store.resetReceipt.resetCount }}</template>
          </span>
          <p class="acks-receipt-text">{{ resetReceiptText }}</p>
        </div>
      </template>
    </section>

    <el-dialog
      v-model="resetVisible"
      class="acks-dialog"
      :title="resetTarget?.all ? '全量重置调度状态' : `重置「${expertNameOr(resetTarget, resetTarget?.id || '该专家')}」的调度状态`"
      width="560px"
      :close-on-click-modal="false"
    >
      <ol class="acks-lines">
        <li v-for="line in resetLines" :key="line">{{ line }}</li>
      </ol>
      <el-alert
        v-if="store.error.reset"
        class="acks-dialog-alert"
        type="error"
        show-icon
        :closable="false"
        title="重置未生效"
        :description="store.error.reset"
      />
      <label v-if="!resetTarget?.all" class="acks-dialog-field">
        <span class="acks-dialog-label">重置原因（可选，后端只回显不消费）</span>
        <el-input v-model="resetReason" size="small" placeholder="例：演练后清场" />
        <em class="acks-dialog-hint">ResetBody 只有 reason 一个字段（experts_dispatcher.rs:114-117），它出现在回执里但不参与任何判定；即使留空也要发一个 JSON 对象，因为 handler 的 Json 提取器不接受空体</em>
      </label>
      <label v-else class="acks-dialog-field">
        <span class="acks-dialog-label">输入 {{ DISPATCH_RESET_ALL_CONFIRM }} 以确认</span>
        <el-input v-model="resetWord" size="small" :placeholder="DISPATCH_RESET_ALL_CONFIRM" />
        <em class="acks-dialog-hint">后端没有角色判定（只认证不授权），这道手动门槛是前端唯一的减速带</em>
      </label>
      <template #footer>
        <span class="acks-dialog-foot">
          <el-button size="small" @click="resetVisible = false">取消</el-button>
          <el-button
            class="acks-dialog-go"
            size="small"
            type="danger"
            :loading="store.loading.reset"
            :disabled="!resetReady"
            @click="confirmReset"
          >确认重置</el-button>
        </span>
      </template>
    </el-dialog>

    <div class="ac-body">
      <aside class="ac-list">
        <el-input v-model="keyword" placeholder="按标题或 ID 过滤" clearable size="small" class="ac-filter" />
        <div v-if="store.loading.tasks" class="ac-loading">
          <el-skeleton :rows="5" animated />
        </div>
        <el-empty v-else-if="!filteredTasks.length" :image-size="60" :description="store.tasks.length ? '无匹配任务' : '暂无联盟任务'" />
        <el-scrollbar v-else class="ac-list-scroll">
          <button
            v-for="t in filteredTasks"
            :key="t.id"
            class="ac-task"
            :class="{ active: t.id === store.selectedId }"
            @click="store.selectTask(t.id)"
          >
            <span class="ac-task-title">{{ t.title || '(无标题)' }}</span>
            <span class="ac-task-meta">
              <el-tag :type="taskTone(t.status)" size="small" effect="light">{{ taskStatusLabel(t.status) }}</el-tag>
              <span class="ac-task-id">{{ t.id }}</span>
            </span>
            <el-progress :percentage="clampPercent(t.progress * 100)" :stroke-width="4" :show-text="false" />
          </button>
        </el-scrollbar>
        <footer class="ac-list-foot">共 {{ store.total }} 条 · 进行中 {{ store.activeTasks.length }}</footer>
      </aside>

      <section class="ac-detail">
        <el-skeleton v-if="store.loading.detail" :rows="6" animated />
        <el-empty v-else-if="!store.detail.task" :image-size="80" description="选择左侧任务查看执行详情" />
        <template v-else>
          <div class="ac-detail-head">
            <div>
              <h2 class="ac-detail-title">{{ store.detail.task.title || store.detail.task.id }}</h2>
              <p class="ac-detail-sub">
                {{ store.detail.task.description || '（无描述）' }}
              </p>
            </div>
            <div class="ac-detail-actions">
              <el-button size="small" :icon="VideoPause" :disabled="!canPause" @click="act('pause')">暂停</el-button>
              <el-button size="small" :icon="VideoPlay" :disabled="!canResume" @click="act('resume')">恢复</el-button>
              <el-button size="small" :icon="CircleClose" :disabled="!canCancel" @click="act('cancel')">取消</el-button>
              <el-button
                size="small"
                :icon="Finished"
                :loading="store.loading.toggle"
                :disabled="reopenBlocked"
                :title="reopenHint"
                @click="toggleDone"
              >{{ current?.status === TASK_STATUS.COMPLETED ? '重新打开' : '标记完成' }}</el-button>
              <el-button size="small" @click="store.selectTask(store.selectedId)">重载</el-button>
            </div>
          </div>

          <el-alert v-if="store.error.toggle" class="ac-alert" type="error" show-icon :closable="false" title="完成状态切换失败" :description="store.error.toggle" />
          <p v-if="toggleNote" class="ac-toggle-note">{{ toggleNote }}</p>

          <el-alert v-if="store.error.detail" class="ac-alert" type="error" show-icon :closable="false" :title="store.error.detail" />

          <el-tabs v-model="tab" class="ac-tabs">
            <el-tab-pane label="概览" name="overview">
              <el-descriptions :column="3" border size="small">
                <el-descriptions-item label="状态">
                  <el-tag :type="taskTone(store.detail.task.status)" size="small">{{ taskStatusLabel(store.detail.task.status) }}</el-tag>
                </el-descriptions-item>
                <el-descriptions-item label="协作模式">{{ modeLabel(store.detail.task.mode) }}</el-descriptions-item>
                <el-descriptions-item label="优先级">{{ store.detail.task.priority || '—' }}</el-descriptions-item>
                <el-descriptions-item label="进度">{{ clampPercent(store.detail.task.progress * 100) }}%</el-descriptions-item>
                <el-descriptions-item label="创建时间">{{ store.detail.task.createdAt || '—' }}</el-descriptions-item>
                <el-descriptions-item label="耗时">{{ formatDuration(store.detail.task.durationMs) }}</el-descriptions-item>
              </el-descriptions>

              <div v-if="store.detail.execution" class="ac-kpis">
                <div v-for="k in kpis" :key="k.label" class="ac-kpi">
                  <span class="ac-kpi-value">{{ k.value }}</span>
                  <span class="ac-kpi-label">{{ k.label }}</span>
                </div>
              </div>
            </el-tab-pane>

            <el-tab-pane label="节点" name="nodes">
              <el-table :data="store.detail.nodes" size="small" max-height="420">
                <el-table-column prop="id" label="节点" width="120" />
                <el-table-column prop="name" label="名称" min-width="140" />
                <el-table-column prop="expertId" label="专家" width="140" />
                <el-table-column label="状态" width="110">
                  <template #default="{ row }">
                    <el-tag :type="nodeTone(row.status)" size="small" effect="light">{{ nodeStatusLabel(row.status) }}</el-tag>
                  </template>
                </el-table-column>
                <el-table-column label="依赖" min-width="140">
                  <template #default="{ row }">{{ row.dependencies.length ? row.dependencies.join(' → ') : '—' }}</template>
                </el-table-column>
                <el-table-column label="耗时" width="100">
                  <template #default="{ row }">{{ formatDuration(row.durationMs) }}</template>
                </el-table-column>
              </el-table>
            </el-tab-pane>

            <el-tab-pane :label="`DAG${dagCount ? ` (${dagCount})` : ''}`" name="dag">
              <el-empty v-if="!store.detail.dag?.nodes?.length" :image-size="60" description="该任务尚未生成 DAG" />
              <div v-else class="ac-dag">
                <p class="ac-dag-mode">{{ dagModeNote }}</p>
                <div v-for="layer in dagView.layers" :key="layer.depth" class="ac-dag-layer">
                  <span class="ac-dag-depth">第 {{ layer.depth + 1 }} 层</span>
                  <div class="ac-dag-nodes">
                    <span v-for="n in layer.nodes" :key="n.id" class="ac-dag-node" :class="`is-${n.status}`">
                      {{ n.label || n.name }}
                      <em>{{ nodeStatusLabel(n.status) }}</em>
                    </span>
                  </div>
                </div>
                <p class="ac-dag-foot">
                  {{ dagView.drawn }} 条依赖参与分层 · 完成 {{ store.detail.dag.stats.completed }} / {{ store.detail.dag.stats.total }}<template v-if="dagView.tally.skipped"> · 跳过 {{ dagView.tally.skipped }}</template><template v-if="dagView.tally.cancelled"> · 取消 {{ dagView.tally.cancelled }}</template><template v-if="dagView.dangling"> · {{ dagView.dangling }} 条依赖指向不存在的节点，不计入</template><template v-if="dagView.backEdges"> · {{ dagView.backEdges }} 条依赖成环，分层为截断结果</template><template v-if="dagView.edgeDelta"> · 后端边数 {{ dagView.edgeTotal }} 与依赖清单 {{ dagView.declared }} 不一致</template>
                </p>
              </div>
            </el-tab-pane>

            <el-tab-pane label="融合结果" name="fusion">
              <el-empty v-if="!store.detail.fusion" :image-size="60" description="尚无融合产物" />
              <div v-else class="ac-fusion">
                <div class="ac-fusion-head">
                  <span class="ac-grade" :class="`grade-${store.detail.fusion.grade || 'none'}`">{{ store.detail.fusion.grade || '未评' }}</span>
                  <div>
                    <p class="ac-fusion-grade-label">{{ gradeLabel(store.detail.fusion.grade) }}</p>
                    <p class="ac-fusion-meta">
                      策略 {{ fusionLabel(store.detail.fusion.strategy) }} · 置信度 {{ percent(store.detail.fusion.confidence) }} · 参与节点 {{ store.detail.fusion.participatingNodes.length }}
                    </p>
                  </div>
                </div>
                <p class="ac-fusion-summary">{{ store.detail.fusion.summary || '（无摘要）' }}</p>
                <ul v-if="store.detail.fusion.keyFindings.length" class="ac-bullets">
                  <li v-for="(f, i) in store.detail.fusion.keyFindings" :key="i">{{ f }}</li>
                </ul>
                <ul v-if="store.detail.fusion.recommendations.length" class="ac-bullets ac-bullets-rec">
                  <li v-for="(r, i) in store.detail.fusion.recommendations" :key="i">{{ r }}</li>
                </ul>
                <el-table v-if="store.detail.fusion.nodeContributions.length" :data="store.detail.fusion.nodeContributions" size="small">
                  <el-table-column prop="nodeId" label="节点" width="130" />
                  <el-table-column prop="expert" label="专家" width="150" />
                  <el-table-column label="权重" width="90">
                    <template #default="{ row }">{{ percent(row.weight) }}</template>
                  </el-table-column>
                  <el-table-column prop="contribution" label="贡献" min-width="200" />
                </el-table>
              </div>
            </el-tab-pane>

            <el-tab-pane :label="`日志 (${store.logs.length})`" name="logs">
              <div class="ac-logs-bar">
                <el-tag size="small" :type="streamTag.type" effect="plain">{{ streamTag.text }}</el-tag>
                <span class="ac-logs-hint">{{ phaseHint }}</span>
              </div>
              <el-scrollbar ref="logScrollRef" class="ac-logs">
                <p v-for="l in store.logs" :key="`${l.seq}-${l.ts}`" class="ac-log" :class="`lv-${l.level}`">
                  <span class="ac-log-seq">{{ l.seq }}</span>
                  <span class="ac-log-time">{{ l.ts }}</span>
                  <span class="ac-log-node">{{ l.nodeId || '—' }}</span>
                  <span class="ac-log-msg">{{ l.message }}</span>
                </p>
                <p v-if="!store.logs.length" class="ac-log-empty">暂无日志帧</p>
              </el-scrollbar>
            </el-tab-pane>
          </el-tabs>
        </template>
      </section>
    </div>

    <el-dialog v-model="createVisible" title="新建联盟任务" width="520px">
      <el-form :model="form" label-width="88px" label-position="top">
        <el-form-item label="标题" required>
          <el-input v-model="form.title" placeholder="一句话说明要解决的问题" maxlength="80" show-word-limit />
        </el-form-item>
        <el-form-item label="需求描述" required>
          <el-input v-model="form.description" type="textarea" :rows="4" placeholder="交给联盟的完整需求，越具体匹配越准" />
        </el-form-item>
        <el-form-item label="协作模式">
          <el-select v-model="form.mode">
            <el-option v-for="o in modeOptions" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
        <el-form-item label="融合策略">
          <el-select v-model="form.fusionStrategy">
            <el-option v-for="o in fusionOptions" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
        <el-form-item label="优先级">
          <el-radio-group v-model="form.priority">
            <el-radio-button v-for="p in priorityOptions" :key="p" :value="p">{{ p }}</el-radio-button>
          </el-radio-group>
        </el-form-item>
      </el-form>
      <el-alert v-if="store.error.create" type="error" :closable="false" show-icon :title="store.error.create" />
      <template #footer>
        <el-button @click="createVisible = false">取消</el-button>
        <el-button type="primary" :loading="store.loading.create" :disabled="!formReady" @click="submit">创建</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus/es/components/message/index'
import { CircleClose, Finished, MagicStick, Plus, Refresh, VideoPause, VideoPlay } from '@element-plus/icons-vue'
import { useSSE } from '@/composables'
import { allianceApi } from '@/modules/expert-alliance/api'
import { useAllianceConsoleStore } from '@/modules/expert-alliance/store'
import {
  FUSION_STRATEGY, GATE_THRESHOLDS, TASK_STATUS, NODE_STATUS, MODE_DISPLAY, MODE_WIRE, PRIORITY,
  fusionLabel, gradeLabel, modeLabel, nodeStatusLabel, taskStatusLabel
} from '@/modules/expert-alliance/contract'
import { PHASE_IDS, phaseLabel } from '@/modules/expert-alliance/contract'
import {
  DISPATCH_ENGINE_STATUS_LITERAL, DISPATCH_RESET_ALL_CONFIRM, DISPATCH_STRATEGY,
  dispatchResetAllNotice, dispatchResetLines, dispatchResetNotice, dispatchRunProblem
} from '@/modules/expert-alliance/contract'
import { ENDPOINTS } from '@/modules/expert-alliance/contract'
import { expertNameOr, breakerStateLabel } from '@/modules/expert-alliance/contract'
import { modeTopologyNote } from '@/modules/expert-alliance/contract'
import { normLogEntry } from '@/modules/expert-alliance/model'
import { layoutDag } from '@/modules/expert-alliance/model'

const store = useAllianceConsoleStore()
const router = useRouter()
const keyword = ref('')
const tab = ref('overview')
const createVisible = ref(false)
const logScrollRef = ref(null)
const streamMode = ref('idle')
let sseCtl = null
let pollTimer = null

const form = reactive({ title: '', description: '', mode: MODE_WIRE.PARALLEL, fusionStrategy: FUSION_STRATEGY.WEIGHTED, priority: PRIORITY.NORMAL })

const priorityOptions = Object.values(PRIORITY)
const modeOptions = Object.entries(MODE_WIRE).map(([, wire]) => ({ value: wire, label: `${modeLabel(MODE_DISPLAY[Object.keys(MODE_WIRE).find(k => MODE_WIRE[k] === wire)])} (${wire})` }))
const fusionOptions = Object.entries(FUSION_STRATEGY).map(([, v]) => ({ value: v, label: fusionLabel(v) }))

// 面板上的路径/方法一律读端点表，不在此重打一遍字面量（漂移由契约测试判红）
const RUN_ENDPOINT = ENDPOINTS.dispatcherRun
const LIST_ENDPOINT = ENDPOINTS.expertsList
const STATUS_ENDPOINT = ENDPOINTS.dispatcherStatus
const runForm = reactive({ taskType: 'code_review', input: '', expertIds: [] })
const toggleNote = ref('')
const runReady = computed(() => !dispatchRunProblem(runForm) && !store.loading.dispatch)

// 远程任务的完成是单向的：重开必经网关返回 409，所以本地"可逆"不能推广到远程
const reopenBlocked = computed(() => current.value?.status === TASK_STATUS.COMPLETED && store.runtime?.mode === 'remote')
const reopenHint = computed(() => (reopenBlocked.value ? '远程任务生命周期单向：已完成的任务经网关重新打开会返回 409' : ''))

async function toggleDone() {
  toggleNote.value = ''
  const res = await store.toggleTaskDone()
  if (!res) return
  if (res.branch === 'remote') {
    toggleNote.value = '远程调度器只回了成功与消息，没有回传状态，下面的状态与节点以重取结果为准'
  } else if (res.direction === 'reopened') {
    toggleNote.value = '已重新打开：状态回到 running，但被标记完成时置过的节点进度不会自动回退'
  }
  ElMessage.success(res.message || (res.direction === 'reopened' ? '已重新打开' : '已标记完成'))
}

async function runOnce() {
  const res = await store.runDispatch({ ...runForm })
  if (res) ElMessage.success(`已分发给 ${res.assigned.length} 位专家`)
}

function onCandidatesOpen(visible) {
  if (visible && !store.dispatchCandidates.length && !store.loading.candidates) store.loadDispatchCandidates()
}

const filteredTasks = computed(() => {
  const k = keyword.value.trim().toLowerCase()
  if (!k) return store.tasks
  return store.tasks.filter((t) => `${t.title} ${t.id}`.toLowerCase().includes(k))
})
const formReady = computed(() => form.title.trim().length > 1 && form.description.trim().length > 3)
const current = computed(() => store.detail.task)
const canPause = computed(() => current.value?.status === TASK_STATUS.RUNNING)
const canResume = computed(() => current.value?.status === TASK_STATUS.PAUSED)
// §5.54 F25：可取消档位交 TASK_STATUS（上面两行已经这么写了，这一行是同一本账的漏网元素表）；成员与顺序未动
const canCancel = computed(() => [TASK_STATUS.PENDING, TASK_STATUS.PLANNING, TASK_STATUS.RUNNING, TASK_STATUS.PAUSED].includes(current.value?.status))

const dagCount = computed(() => store.detail.dag?.nodes?.length ?? 0)
const dagView = computed(() => layoutDag(store.detail.dag?.nodes ?? [], store.detail.dag?.edges ?? []))
// 模式→拓扑那句话的口径住在 contract/mode.js，视图只取值（后端换模板时先红契约，不红这里）
const dagModeNote = computed(() => modeTopologyNote(store.detail.task?.mode))

const kpis = computed(() => {
  const c = store.detail.execution?.counts
  if (!c) return []
  return [
    { label: '节点总数', value: c.total },
    { label: '已完成', value: c.completed },
    { label: '执行中', value: c.running },
    { label: '待执行', value: c.pending },
    { label: '失败', value: c.failed }
  ]
})

const runtimeTag = computed(() => {
  if (store.loading.runtime) return { type: 'info', text: '运行时检测中' }
  if (!store.runtime) return { type: 'danger', text: '运行时未知' }
  return store.runtime.executionReady
    ? { type: 'success', text: `执行引擎 ${store.runtime.mode}` }
    : { type: 'warning', text: '本地预览（模拟执行）' }
})

const configTag = computed(() => {
  if (store.loading.config) return { type: 'info', text: '配置读写中' }
  if (!store.dispatcherConfig) return { type: 'danger', text: '配置未取到' }
  if (store.configPatch.problem) return { type: 'warning', text: '有改动项越界' }
  return store.configChanged
    ? { type: 'warning', text: `待保存 ${Object.keys(store.configPatch.patch).length} 项` }
    : { type: 'success', text: '与后端一致' }
})

// 网关 message 自带句末标点，直接再拼句号会得到「。。」
const runtimeHint = computed(() => {
  const raw = String(store.runtime?.message || '未配置远端调度器').replace(/[。.；;，,]+$/, '')
  return `执行引擎未就绪：${raw}。节点进度与耗时由网关按模式模拟生成，不作为真实执行指标。`
})

const streamTag = computed(() => ({
  open: { type: 'success', text: '日志流已连接' },
  polling: { type: 'warning', text: '日志轮询（流不可用）' },
  idle: { type: 'info', text: '日志流未启用' },
  closed: { type: 'info', text: '日志流已关闭' }
}[streamMode.value]))

const phaseHint = computed(() => {
  const done = store.detail.nodes?.filter((n) => n.status === NODE_STATUS.COMPLETED).length ?? 0
  return `执行阶段 ${done}/${store.detail.nodes?.length ?? 0} · 7 阶段管线：${PHASE_IDS.map(phaseLabel).join(' → ')}`
})

const clampPercent = (v) => Math.max(0, Math.min(100, Math.round(Number(v) || 0)))
const percent = (v) => (v === null || v === undefined ? '—' : `${Math.round(Number(v) * 100)}%`)
const formatDuration = (ms) => {
  if (ms === null || ms === undefined) return '—'
  const s = Math.round(ms / 1000)
  if (s < 60) return `${s}s`
  if (s < 3600) return `${Math.floor(s / 60)}m${s % 60}s`
  return `${Math.floor(s / 3600)}h${Math.floor((s % 3600) / 60)}m`
}
const taskTone = (status) => ({ completed: 'success', failed: 'danger', cancelled: 'info', running: 'primary', paused: 'warning' }[status] ?? 'info')
const nodeTone = (status) => ({ completed: 'success', failed: 'danger', skipped: 'info', cancelled: 'info', running: 'primary' }[status] ?? 'info')

function detachStream() {
  if (sseCtl) {
    sseCtl.disconnect()
    sseCtl = null
  }
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
  streamMode.value = 'idle'
}

function startPolling(id) {
  if (pollTimer) return
  streamMode.value = 'polling'
  pollTimer = setInterval(async () => {
    if (store.isTerminal(id)) {
      detachStream()
      return
    }
    const res = await allianceApi.getLogs(id).catch(() => null)
    if (res) store.replaceLogs(res.items)
  }, 4000)
}

function attachStream(id) {
  detachStream()
  if (!id || store.isTerminal(id)) return
  streamMode.value = 'open'
  sseCtl = useSSE({
    url: allianceApi.taskLogStreamUrl(id),
    method: 'GET',
    timeoutMs: 15000,
    maxRetries: 1,
    onEvent: (evt) => {
      store.pushLog(normLogEntry(evt.payload ?? evt.data))
      scrollLogs()
    },
    onError: () => {
      sseCtl = null
      startPolling(id)
    }
  })
  sseCtl.connect().catch(() => {
    sseCtl = null
    startPolling(id)
  })
}

function scrollLogs() {
  nextTick(() => {
    const bar = logScrollRef.value?.wrapRef
    if (bar) bar.scrollTop = bar.scrollHeight
  })
}

async function act(verb) {
  const res = await store.controlTask(verb)
  if (res?.success) ElMessage.success(res.message || `已${verb === 'pause' ? '暂停' : verb === 'resume' ? '恢复' : '取消'}`)
  else if (res) ElMessage.warning(res.message || '操作未生效')
}

async function submit() {
  const created = await store.createTask({ ...form, title: form.title.trim(), description: form.description.trim() })
  if (created) {
    createVisible.value = false
    ElMessage.success(`任务已创建：${created.id}`)
    form.title = ''
    form.description = ''
  }
}

function goCollab() {
  router.push('/alliance/collab')
}

async function reload() {
  await Promise.all([store.loadRuntime(), store.loadTasks(), store.loadDispatcherConfig(), store.loadDispatcherStatus()])
}

// ── 负载重置：破坏性面。二次确认清单来自契约，回执留在卡片区而不是只闪一条 toast ──
const resetVisible = ref(false)
const resetTarget = ref(null)
const resetReason = ref('')
const resetWord = ref('')
const resetLines = computed(() => dispatchResetLines(resetTarget.value || {}))
// 全量重置没有角色判定可依赖（后端只认证不授权），手动输入确认词是前端唯一的中继
const resetReady = computed(() => !resetTarget.value?.all || resetWord.value === DISPATCH_RESET_ALL_CONFIRM)
const resetReceiptText = computed(() => {
  const r = store.resetReceipt
  if (!r) return ''
  return r.scope === 'all' ? dispatchResetAllNotice(r) : dispatchResetNotice(r)
})

function openReset(target) {
  resetTarget.value = target
  resetReason.value = ''
  resetWord.value = ''
  store.error.reset = ''
  resetVisible.value = true
}

async function confirmReset() {
  if (!resetReady.value) return
  const all = !!resetTarget.value?.all
  const res = all ? await store.resetAllLoads() : await store.resetExpertLoad(resetTarget.value?.id, resetReason.value)
  // 失败不关窗：错误条要在发起这个动作的弹窗里呈现，遮罩外的页面级横幅用户看不见
  if (!res) return
  ElMessage.success(all ? '已请求全量重置' : '已请求重置该专家负载')
  resetVisible.value = false
}

async function saveConfig() {
  const saved = await store.saveDispatcherConfig()
  if (saved) ElMessage.success('调度配置已更新')
  else if (store.error.config) ElMessage.error(store.error.config)
}

watch(() => store.selectedId, (id) => attachStream(id))
onMounted(async () => {
  await reload()
  if (store.error.runtime) ElMessage.error(store.error.runtime)
})
onBeforeUnmount(detachStream)

// GATE_THRESHOLDS 供融合面板分级提示，避免魔法数散落
void GATE_THRESHOLDS
</script>

<style scoped>
.ac {
  display: flex;
  flex-direction: column;
  gap: 14px;
  height: 100%;
  min-height: 0;
}
.ac-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}
.ac-title {
  margin: 0;
  font-size: 19px;
  font-weight: 600;
  color: var(--text-primary);
}
.ac-sub {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--text-muted);
}
.ac-head-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.ac-hint { margin-right: 10px; font-size: 11px; color: var(--text-muted); }
.ac-alert {
  margin: 0;
}
.ack-card {
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-card);
  padding: 12px 14px;
  margin-bottom: 14px;
}
.ack-head {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.ack-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
}
.ack-note {
  font-size: 12px;
  font-style: normal;
  color: var(--text-muted);
}
.ack-save {
  margin-left: auto;
}
.ack-err {
  margin: 8px 0 0;
  font-size: 12px;
  color: var(--danger);
}
.ack-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 10px 14px;
  margin-top: 10px;
}
.ack-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.ack-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-secondary);
}
.ack-absent {
  font-style: normal;
  font-size: 11px;
  color: var(--warning);
}
.ack-hint {
  font-size: 11px;
  font-style: normal;
  color: var(--text-muted);
}
.ack-input {
  width: 100%;
}
.ac-body {
  display: grid;
  grid-template-columns: 280px minmax(0, 1fr);
  gap: 14px;
  flex: 1;
  min-height: 0;
}
.ac-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-height: 0;
  padding: 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.ac-list-scroll {
  flex: 1;
  min-height: 0;
}
.ac-task {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 100%;
  margin-bottom: 6px;
  padding: 9px 10px;
  text-align: left;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  color: inherit;
  cursor: pointer;
  transition: border-color var(--dur-2) var(--ease), background var(--dur-2) var(--ease);
}
.ac-task:hover {
  border-color: var(--border-light);
  background: var(--bg-hover);
}
.ac-task.active {
  border-color: var(--accent);
  background: var(--accent-dim);
}
.ac-task-title {
  font-size: 13px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ac-task-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
}
.ac-task-id {
  font-size: 11px;
  color: var(--text-muted);
  font-family: ui-monospace, Menlo, Consolas, monospace;
}
.ac-list-foot {
  font-size: 11px;
  color: var(--text-muted);
}
.ac-loading,
.ac-detail {
  min-height: 0;
}
.ac-detail {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.ac-detail-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
.ac-detail-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}
.ac-detail-sub {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--text-secondary);
}
.ac-detail-actions {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}
.ac-tabs {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.ac-kpis {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(112px, 1fr));
  gap: 10px;
  margin-top: 14px;
}
.ac-kpi {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-card);
}
.ac-kpi-value {
  font-size: 22px;
  font-weight: 600;
  color: var(--accent-light);
}
.ac-kpi-label {
  font-size: 11px;
  color: var(--text-muted);
}
.ac-dag {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.ac-dag-layer {
  display: flex;
  align-items: center;
  gap: 10px;
}
.ac-dag-mode {
  margin: 0;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-secondary);
  font-size: 12px;
  line-height: 1.7;
  color: var(--text-secondary);
}
.ac-dag-depth {
  width: 52px;
  flex-shrink: 0;
  font-size: 11px;
  color: var(--text-muted);
}
.ac-dag-nodes {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.ac-dag-node {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 10px;
  font-size: 12px;
  border: 1px solid var(--border);
  border-left-width: 3px;
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  color: var(--text-primary);
}
.ac-dag-node em {
  font-style: normal;
  font-size: 10px;
  color: var(--text-muted);
}
.ac-dag-node.is-completed { border-left-color: var(--success); }
.ac-dag-node.is-running { border-left-color: var(--accent); }
.ac-dag-node.is-failed { border-left-color: var(--danger); }
.ac-dag-node.is-pending { border-left-color: var(--text-tertiary); }
.ac-dag-node.is-skipped { border-left-color: var(--warning); }
.ac-dag-node.is-cancelled { border-left-color: var(--text-quaternary); }
.ac-dag-foot {
  margin: 0;
  font-size: 11px;
  color: var(--text-muted);
}
.ac-fusion-head {
  display: flex;
  align-items: center;
  gap: 14px;
}
.ac-grade {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 46px;
  height: 46px;
  font-size: 21px;
  font-weight: 700;
  border-radius: var(--radius);
  background: var(--bg-card);
  color: var(--text-secondary);
  border: 1px solid var(--border);
}
.ac-grade.grade-A { color: var(--success); border-color: var(--success); }
.ac-grade.grade-B { color: var(--accent-light); border-color: var(--accent); }
.ac-grade.grade-C { color: var(--warning); border-color: var(--warning); }
.ac-grade.grade-D { color: var(--danger); border-color: var(--danger); }
.ac-fusion-grade-label {
  margin: 0;
  font-size: 13px;
  color: var(--text-primary);
}
.ac-fusion-meta {
  margin: 2px 0 0;
  font-size: 11px;
  color: var(--text-muted);
}
.ac-fusion-summary {
  margin: 12px 0;
  font-size: 13px;
  line-height: 1.7;
  color: var(--text-secondary);
  white-space: pre-wrap;
}
.ac-bullets {
  margin: 0 0 10px;
  padding-left: 18px;
  font-size: 12px;
  line-height: 1.8;
  color: var(--text-secondary);
}
.ac-bullets-rec li {
  color: var(--accent-light);
}
.ac-logs-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}
.ac-logs-hint {
  font-size: 11px;
  color: var(--text-muted);
}
.ac-logs {
  height: 320px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-tertiary);
}
.ac-log {
  display: grid;
  grid-template-columns: 40px 150px 90px minmax(0, 1fr);
  gap: 8px;
  margin: 0;
  padding: 3px 0;
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 11px;
  color: var(--text-secondary);
}
.ac-log.lv-warn .ac-log-msg { color: var(--warning); }
.ac-log.lv-error .ac-log-msg,
.ac-log.lv-error .ac-log-msg { color: var(--danger); }
.ac-log-node { color: var(--text-muted); }
.ac-log-empty {
  margin: 0;
  font-size: 12px;
  color: var(--text-muted);
}
.ac-toggle-note {
  margin: 0;
  padding: 6px 10px;
  font-size: 12px;
  color: var(--text-secondary);
  background: var(--bg-secondary);
  border-left: 2px solid var(--accent-dim);
  border-radius: var(--radius-sm);
}
.ack-run {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px dashed var(--border-light);
}
.ack-run-head {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.ack-run-title {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}
.ack-run-note {
  flex: 1 1 320px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--text-muted);
}
.ack-run-form {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 10px;
  margin-top: 10px;
}
.ack-run-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.ack-run-label {
  font-size: 12px;
  color: var(--text-secondary);
}
.ack-run-hint {
  font-size: 11px;
  font-style: normal;
  line-height: 1.5;
  color: var(--text-muted);
}
.ack-run-out {
  margin-top: 12px;
  padding: 10px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-md);
}
.ack-run-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.ack-run-id {
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 11px;
  color: var(--text-muted);
}
.ack-run-list {
  margin: 8px 0 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.ack-run-item {
  display: flex;
  align-items: baseline;
  gap: 10px;
  flex-wrap: wrap;
  font-size: 12px;
  color: var(--text-primary);
}
.ack-run-name {
  font-weight: 600;
}
.ack-run-score,
.ack-run-load {
  color: var(--text-secondary);
}
.ack-run-mid {
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 11px;
  color: var(--text-muted);
}
.ack-run-findings {
  margin: 10px 0 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.ack-run-finding {
  font-size: 12px;
  line-height: 1.5;
  color: var(--text-secondary);
  padding-left: 8px;
  border-left: 2px solid var(--border);
}
.ack-run-finding[data-tone='success'] {
  color: var(--success);
  border-left-color: var(--success);
}
.ack-run-finding[data-tone='warning'] {
  color: var(--warning);
  border-left-color: var(--warning);
}
.ack-run-finding[data-tone='danger'] {
  color: var(--danger);
  border-left-color: var(--danger);
}
.ack-run-finding[data-tone='info'] {
  color: var(--text-secondary);
  border-left-color: var(--accent-dim);
}
.acks-card {
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-card);
  padding: 12px 14px;
  margin-bottom: 14px;
}
.acks-head {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.acks-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
}
.acks-note {
  font-size: 12px;
  font-style: normal;
  color: var(--text-muted);
}
.acks-reload {
  margin-left: auto;
}
.acks-err {
  margin: 8px 0 0;
  font-size: 12px;
  color: var(--danger);
}
.acks-absent {
  margin: 8px 0 0;
  font-size: 12px;
  color: var(--warning);
}
.acks-kpi {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 10px 14px;
  margin-top: 10px;
}
.acks-cell {
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.acks-cell-label {
  font-size: 12px;
  color: var(--text-secondary);
}
.acks-cell-value {
  font-size: 13px;
  color: var(--text-primary);
}
.acks-cell-hint {
  font-size: 11px;
  font-style: normal;
  color: var(--text-muted);
}
.acks-block {
  margin-top: 12px;
}
.acks-block-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 6px;
}
.acks-cb {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.acks-cb-item {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 12px;
  color: var(--text-secondary);
}
.acks-cb-state {
  color: var(--danger);
}
.acks-honest {
  margin: 0;
  padding: 8px 10px;
  border-left: 2px solid var(--accent-dim);
  background: var(--bg-secondary);
  border-radius: var(--radius-sm);
  font-size: 12px;
  color: var(--text-secondary);
}
.acks-table {
  width: 100%;
}
.acks-name {
  font-size: 12px;
  color: var(--text-primary);
  margin-right: 6px;
}
.acks-mid {
  font-size: 11px;
  color: var(--text-muted);
}
.acks-load {
  display: block;
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 2px;
}
.acks-foot {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  margin-top: 12px;
}
.acks-foot-note {
  font-size: 11px;
  font-style: normal;
  color: var(--text-muted);
}
.acks-receipt-clear {
  margin-left: auto;
}
.acks-receipt {
  margin-top: 8px;
  padding: 8px 10px;
  border: 1px solid var(--border-light);
  border-radius: var(--radius-sm);
  background: var(--bg-secondary);
}
.acks-receipt-title {
  display: block;
  font-size: 12px;
  color: var(--text-secondary);
}
.acks-receipt-text {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--text-muted);
}
.acks-lines {
  margin: 0 0 10px;
  padding-left: 18px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.acks-lines li {
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-secondary);
}
.acks-dialog-alert {
  margin-bottom: 10px;
}
.acks-dialog-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 8px;
}
.acks-dialog-label {
  font-size: 12px;
  color: var(--text-secondary);
}
.acks-dialog-hint {
  font-size: 11px;
  font-style: normal;
  color: var(--text-muted);
}
.acks-dialog-foot {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
@media (max-width: 1024px) {
  .ac-body {
    grid-template-columns: minmax(0, 1fr);
  }
  .ac-list {
    max-height: 260px;
  }
}
</style>
