<template>
  <div class="resources-overview-placeholder"></div>
</template>

<script setup>
// 资源概览内容由父组件 ResourcesView 通过 v-show 渲染
// 此组件作为嵌套路由的占位符，保持 URL 结构正确
</script>

<style scoped>
.resources-overview-placeholder {
  display: none;
}
</style>
