<script setup lang="ts">
import { useSeuFairs } from '../composables/useSeuFairs'
const { status, refreshing, error, refresh } = useSeuFairs()
</script>

<template>
  <div class="fair-sync-bar fair-theme">
    <div class="fair-sync-info" aria-live="polite">
      <span>宣讲会 · 东南大学就业网</span>
      <small v-if="refreshing">正在同步官网…</small>
      <small v-else-if="error" class="sync-error">{{ error }}</small>
      <small v-else-if="status.updated_at" :title="`更新于 ${status.updated_at}`">{{ status.count }} 场 · {{ status.updated_at.slice(5, 16) }} 更新</small>
      <small v-else>尚未同步</small>
    </div>
    <button type="button" :disabled="refreshing" @click="refresh">{{ refreshing ? '同步中' : '刷新' }}</button>
  </div>
</template>

<style scoped>
.fair-sync-bar { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin: 4px 12px; padding: 9px 10px; border: 1px solid var(--card-border); border-radius: 10px; background: var(--card-bg); flex-shrink: 0; }
.fair-sync-info { display: flex; flex-direction: column; min-width: 0; font-size: 12px; font-weight: 500; color: var(--card-ink); }
.fair-sync-info small { font-size: 11px; font-weight: 400; color: var(--card-muted); margin-top: 2px; }
.fair-sync-info .sync-error { color: var(--accent); }
button { font-size: 12px; font-weight: 600; padding: 5px 9px; border: 1px solid var(--card-border); border-radius: 7px; color: var(--accent); background: var(--card-hover); cursor: pointer; flex-shrink: 0; }
button:disabled { cursor: default; opacity: .6; }
</style>
