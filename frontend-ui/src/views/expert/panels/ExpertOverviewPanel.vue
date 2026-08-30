<template>
  <div class="expert-overview-placeholder"></div>
</template>

<script setup>
// 概览内容由父组件 ExpertCenterView 通过 v-show 渲染
// 此组件作为嵌套路由的占位符，保持 URL 结构正确
// 后续可逐步将概览内容迁移至此
</script>

<style scoped>
.expert-overview-placeholder {
  display: none;
}
</style>
