<template>
  <div class="workflow-flows-placeholder"></div>
</template>

<script setup>
// 流程编排内容由父组件 WorkflowView 通过 v-show 渲染
// 此组件作为嵌套路由的占位符，保持 URL 结构正确
// 后续可逐步将编排内容迁移至此
</script>

<style scoped>
.workflow-flows-placeholder {
  display: none;
}
</style>
