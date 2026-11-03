<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { Link, Refresh } from "@element-plus/icons-vue";
import { officialRoleUrl } from "./comparison";
import type { Role } from "./filter";

const open = defineModel<boolean>({ required: true });
const props = withDefaults(defineProps<{ roles: Role[]; kind?: string }>(), { kind: '角色' });
const desktop = isTauri();
const zoom = ref(100);
const loaded = ref<Record<string, boolean>>({});
const versions = ref<Record<string, number>>({});
const error = ref("");
const pages = computed(() =>
  props.roles.map((role) => ({ role, url: officialRoleUrl(role.code) })),
);

watch(open, (value) => {
  if (!value) return;
  zoom.value = 100;
  loaded.value = {};
  versions.value = {};
  error.value = "";
});

function reload(code: string) {
  loaded.value[code] = false;
  versions.value[code] = (versions.value[code] ?? 0) + 1;
}

async function openExternal(code: string) {
  try {
    await invoke("open_role", { code });
  } catch (e) {
    error.value = `打开浏览器失败：${String(e)}`;
  }
}
</script>

<template>
  <el-dialog
    v-model="open"
    fullscreen
    destroy-on-close
    class="official-viewer"
    :title="roles.length === 2 ? `${kind}对比` : '官网详情'"
  >
    <template #header>
      <div class="official-viewer-heading">
        <h2>{{ roles.length === 2 ? `${kind}对比` : "官网详情" }}</h2>
        <span>官网页面{{ roles.length === 2 ? " · 左右独立滚动" : "" }}</span>
        <label for="official-zoom">页面缩放</label>
        <el-select id="official-zoom" v-model="zoom" aria-label="官网页面缩放">
          <el-option
            v-for="value in [100, 80, 60]"
            :key="value"
            :label="`${value}%`"
            :value="value"
          />
        </el-select>
      </div>
    </template>
    <el-alert
      v-if="error"
      :title="error"
      type="error"
      show-icon
      @close="error = ''"
    />
    <div
      class="official-pages"
      :class="{ 'is-comparison': roles.length === 2 }"
    >
      <section
        v-for="{ role, url } in pages"
        :key="role.code"
        class="official-pane"
        :aria-label="`${role.name}官网详情`"
      >
        <header class="official-pane-heading">
          <div class="official-role-name">
            <strong :title="role.name">{{ role.name || "未命名角色" }}</strong
            ><span>{{ role.code }}</span>
          </div>
          <span
            v-if="url && !loaded[role.code]"
            class="official-loading"
            role="status"
            >加载中…</span
          >
          <el-button
            :icon="Refresh"
            :aria-label="`重新加载${role.name}`"
            :disabled="!url"
            @click="reload(role.code)"
          />
          <el-button
            :icon="Link"
            :disabled="!desktop || !url"
            @click="openExternal(role.code)"
            >浏览器打开</el-button
          >
        </header>
        <div v-if="url" class="official-frame-viewport">
          <iframe
            :key="`${role.code}-${versions[role.code] ?? 0}`"
            :src="url"
            :title="`${role.name}官网详情页面`"
            sandbox="allow-scripts allow-same-origin allow-forms"
            referrerpolicy="strict-origin-when-cross-origin"
            :style="{
              width: `${10000 / zoom}%`,
              height: `${10000 / zoom}%`,
              transform: `scale(${zoom / 100})`,
            }"
            @load="loaded[role.code] = true"
          />
        </div>
        <el-empty v-else description="角色编号格式异常，无法打开官网" />
      </section>
    </div>
  </el-dialog>
</template>
