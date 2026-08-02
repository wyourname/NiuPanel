<template>
  <el-config-provider :locale="zhCn">
    <router-view />
  </el-config-provider>
</template>

<script setup lang="ts">
import zhCn from "element-plus/dist/locale/zh-cn.mjs";
</script>

<style>
/* Global styles are now handled in src/assets/styles/index.css */
</style>
