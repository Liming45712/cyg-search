<script setup lang="ts">
import {
  computed,
  onBeforeUnmount,
  onMounted,
  reactive,
  ref,
  shallowRef,
  watch,
} from "vue";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  ArrowRight,
  Box,
  Close,
  Download,
  Filter,
  Plus,
  Refresh,
  Search,
  VideoPause,
} from "@element-plus/icons-vue";
import RangeInput from "./RangeInput.vue";
import OfficialViewer from "./OfficialViewer.vue";
import SpiritCalculator from "./SpiritCalculator.vue";
import SpiritMarket from "./SpiritMarket.vue";
import type { SpiritSimulation } from "./spirit-simulation";
import EquipmentMarket from "./EquipmentMarket.vue";
import CurrencyMarket from "./CurrencyMarket.vue";
import { toggleComparison } from "./comparison";
import { useScrollRows } from "./scroll-rows";
import {
  applyFilters,
  defaults,
  equipTags,
  numeric,
  primary,
  rangeError,
  type Area,
  type Role,
} from "./filter";
import { serverOptions } from "./servers";
import { ElMessageBox, type CascaderValue, type CheckboxValueType, type TableInstance } from "element-plus";

interface Snapshot {
  roles: Role[];
  areas: Area[];
}
interface Progress {
  stage: string;
  done: number;
  total: number;
  message: string;
}
interface Summary {
  cancelled: boolean;
  fetched: number;
  failed: number;
  last_error: string | null;
  removed: number;
}
const desktop = isTauri();
const activeTab = ref('roles');
const spiritSimulation = shallowRef<SpiritSimulation | null>(null);
function simulateSpirit(value: SpiritSimulation) { spiritSimulation.value = value; activeTab.value = 'spirit'; }
const goods = shallowRef<Role[]>([]);
const roles = computed(() => goods.value.filter(item => (item.goods_type ?? 1) === 1));
const currencies = computed(() => goods.value.filter(item => item.goods_type === 2));
const equipment = computed(() => goods.value.filter(item => item.goods_type === 3));
const spirits = computed(() => goods.value.filter(item => item.goods_type === 5));
const areas = shallowRef<Area[]>([]);
const filters = reactive(defaults(localStorage.getItem("cyg-server") || "104"));
const busy = ref(false);
const stopping = ref(false);
const force = ref(false);
const initializing = ref(true);
const areasLoading = ref(false);
const areasError = ref("");
const areasReady = ref(false);
const noticeText = ref<HTMLTextAreaElement>();
const copyStatus = ref("");
const areaSource = ref("默认区服");
const notice = ref("");
const noticeType = ref<"info" | "error" | "warning">("info");
const progress = shallowRef<Progress>({
  stage: "idle",
  done: 0,
  total: 0,
  message: "准备就绪",
});
const sections = ref(["basic", "equipment", "attributes"]);
const newAttribute = ref("");
const selectedCode = ref<string | null>(null);
const viewerOpen = ref(false);
const viewerRoles = shallowRef<Role[]>([]);
const compareCodes = ref<string[]>([]);
const table = ref<TableInstance>();
const viewMode = ref(
  localStorage.getItem("cyg-view") === "list" ? "list" : "cards",
);
const jobs = [
  "剑客",
  "武师",
  "医师",
  "术士",
  "道士",
  "浪子",
  "弓手",
  "禁卫",
  "墨者",
  "巫煞",
  "魂武",
  "寒星",
  "花间舞",
  "剑九",
  "魔刀",
];
const priceFormat = new Intl.NumberFormat("zh-CN", {
  maximumFractionDigits: 2,
});
const fmt = (value: number | null) =>
  value === null ? "—" : priceFormat.format(value);
const time = (value: number | null) =>
  value
    ? new Date(value * 1000).toLocaleString("zh-CN", { hour12: false })
    : "尚未获取";
const attrLabels: Record<string, string> = {
  灵巧度: "灵巧",
  装备总评分: "装备评分",
  铭石总等级: "铭石等级",
  星石总等级: "星石等级",
};
const listFields = [
  "等级",
  "生命值",
  "攻击力",
  "精神力",
  "灵巧度",
  "VIP等级",
  "装备总评分",
  "铭石总等级",
  "星石总等级",
];
const tableFields = listFields.filter((key) => key !== "VIP等级");
const canSync = computed(() => areasReady.value && areas.value.some(
  area => area.server.some(server => server.serverId === filters.server),
));
const options = computed(() => serverOptions(areas.value, goods.value));
const regionOptions = computed(() =>
  [...new Set(options.value.map((s) => s.area))].map((area) => ({
    value: area,
    label: area,
    children: options.value
      .filter((s) => s.area === area)
      .map((s) => ({ value: s.value, label: s.name })),
  })),
);
const server = computed(() =>
  options.value.find((s) => s.value === filters.server),
);
const serverPath = computed({
  get: () => (server.value ? [server.value.area, server.value.value] : []),
  set: (path: CascaderValue) => {
    if (Array.isArray(path) && typeof path[1] === "string")
      filters.server = path[1];
  },
});
const scoped = computed(() =>
  roles.value.filter((r) => r.server_id === filters.server),
);
const detailCount = computed(() => scoped.value.filter((r) => r.detail).length);
const checkedAt = computed(() =>
  Math.max(0, ...scoped.value.map((r) => r.checked_at)),
);
const error = computed(() => rangeError(filters));
const shown = computed(() => applyFilters(roles.value, filters));
const { visibleRows, hasMore, loading: loadingRows, loadMore, onScroll, resetRows } = useScrollRows(shown);
const compareRoles = computed(() =>
  compareCodes.value.flatMap((code) => {
    const role = roles.value.find((r) => r.code === code);
    return role ? [role] : [];
  }),
);
const attributes = computed(() =>
  [
    ...new Set([
      ...primary,
      ...listFields,
      ...roles.value.flatMap((r) =>
        Object.entries(r.detail?.attrs ?? {})
          .filter(([key, v]) => key !== "名称" && v.num !== null)
          .map(([k]) => k),
      ),
    ]),
  ].sort(),
);
const minimumNames = computed(() => [
  ...new Set([...primary, ...Object.keys(filters.minimums)]),
]);
const extraAttributes = computed(() =>
  attributes.value.filter((k) => !minimumNames.value.includes(k)),
);
const sortValue = computed({
  get: () => `${filters.sort}:${filters.desc ? "desc" : "asc"}`,
  set: (value: string) => {
    const [key, direction] = value.split(":");
    filters.sort = key;
    filters.desc = direction === "desc";
  },
});
const percent = computed(() =>
  progress.value.total
    ? Math.min(
        100,
        Math.round((progress.value.done / progress.value.total) * 100),
      )
    : 0,
);
const activeTags = computed(() => {
  const tags: { key: string; label: string }[] = [];
  const bounds = ([lo, hi]: [number | null, number | null]) =>
    `${lo === null ? "不限" : fmt(lo)} – ${hi === null ? "不限" : fmt(hi)}`;
  if (filters.keyword.trim())
    tags.push({ key: "keyword", label: `名称：${filters.keyword.trim()}` });
  if (filters.price.some((v) => v !== null))
    tags.push({ key: "price", label: `价格：${bounds(filters.price)}` });
  if (filters.jobs.length)
    tags.push({ key: "jobs", label: filters.jobs.join(" / ") });
  if (filters.status)
    tags.push({
      key: "status",
      label: filters.status === "1" ? "公示中" : "在售",
    });
  for (const [key, minimum] of Object.entries(filters.minimums))
    if (minimum !== null)
      tags.push({ key: `attr:${key}`, label: `${key} ≥ ${fmt(minimum)}` });
  for (const tag of filters.equipment)
    tags.push({ key: `equip:${tag}`, label: tag });
  return tags;
});
let timer: ReturnType<typeof setInterval> | undefined;
let loadingSnapshot = false;
let unlisten: UnlistenFn | undefined;
function showNotice(message: string, type: typeof noticeType.value = "error") {
  copyStatus.value = "";
  notice.value = message;
  noticeType.value = type;
}
async function copyNotice() {
  try {
    await navigator.clipboard.writeText(notice.value);
    copyStatus.value = "已复制";
  } catch {
    noticeText.value?.focus();
    noticeText.value?.select();
    copyStatus.value = "自动复制失败，请按 Ctrl+C（Mac：⌘C）复制";
  }
}
function reset() {
  Object.assign(filters, defaults(filters.server));
  newAttribute.value = "";
}
function removeTag(key: string) {
  if (key === "keyword") filters.keyword = "";
  else if (key === "price") filters.price = [null, null];
  else if (key === "jobs") filters.jobs = [];
  else if (key === "status") filters.status = "";
  else if (key.startsWith("attr:")) delete filters.minimums[key.slice(5)];
  else if (key.startsWith("equip:"))
    filters.equipment = filters.equipment.filter((tag) => tag !== key.slice(6));
}
function addAttribute() {
  if (
    newAttribute.value &&
    extraAttributes.value.includes(newAttribute.value)
  ) {
    filters.minimums[newAttribute.value] = null;
    newAttribute.value = "";
  }
}
function selectRole(role: Role) {
  selectedCode.value = role.code;
  viewerRoles.value = [role];
  viewerOpen.value = true;
}
function toggleCompare(code: string) {
  compareCodes.value = toggleComparison(compareCodes.value, code);
}
function compareSelected() {
  if (compareRoles.value.length !== 2) return;
  viewerRoles.value = [...compareRoles.value];
  viewerOpen.value = true;
}
watch(roles, (value) => {
  compareCodes.value = compareCodes.value.filter((code) =>
    value.some((role) => role.code === code),
  );
});
function tableSort({
  prop,
  order,
}: {
  prop: string | null;
  order: string | null;
}) {
  filters.sort = order && prop ? prop : "price";
  filters.desc = order === "descending";
}
function detailState(role: Role) {
  return role.detail_error
    ? { text: role.detail ? "更新失败" : "获取失败", type: "danger" as const }
    : role.needs_refresh
      ? { text: "待刷新", type: "warning" as const }
      : role.detail
        ? { text: "已缓存", type: "success" as const }
        : { text: "待获取", type: "info" as const };
}
async function refreshSnapshot() {
  if (loadingSnapshot) return;
  loadingSnapshot = true;
  try {
    const value = await invoke<Snapshot>("snapshot");
    goods.value = value.roles;
    if (value.areas.length) {
      areas.value = value.areas;
      if (areaSource.value !== "官网区服") areaSource.value = "本地区服缓存";
    }
  } finally {
    loadingSnapshot = false;
  }
}
function ensureServer() {
  if (!options.value.some((s) => s.value === filters.server))
    filters.server = options.value[0].value;
}
async function refreshAreas() {
  if (areasLoading.value || busy.value || !desktop) return;
  areasLoading.value = true;
  areasReady.value = false;
  const previousError = areasError.value;
  areasError.value = "";
  try {
    areas.value = await invoke<Area[]>("refresh_areas");
    if (!areas.value.some(area => area.server.length)) throw new Error("官网返回的区服列表为空");
    areasReady.value = true;
    areaSource.value = "官网区服";
    ensureServer();
    if (notice.value === previousError) notice.value = "";
  } catch (e) {
    if (areas.value.length) areaSource.value = "本地区服缓存";
    areasError.value = `区服获取失败，已禁止拉取数据。请刷新区服重试。原因：${String(e)}`;
    showNotice(areasError.value);
  } finally {
    areasLoading.value = false;
  }
}
async function changeFullRefresh(value: CheckboxValueType) {
  if (!value) {
    force.value = false;
    return;
  }
  try {
    await ElMessageBox.confirm(
      "全量刷新会重新请求当前区服、当前分类的全部商品详情，增加访问量。频繁或连续拉取可能触发平台监控、风控或限流，导致访问受限。建议优先使用默认增量更新，仅在确需核对数据时使用全量刷新。",
      "全量刷新风险提示",
      {
        type: "warning",
        confirmButtonText: "了解风险，启用全量刷新",
        cancelButtonText: "取消",
        closeOnClickModal: false,
      },
    );
    force.value = true;
  } catch {
    force.value = false;
  }
}
async function sync() {
  if (busy.value || areasLoading.value || initializing.value || !desktop)
    return;
  if (!canSync.value) {
    showNotice(areasError.value || "请先成功获取官网区服，并选择有效区服后再拉取数据。");
    return;
  }
  busy.value = true;
  const goodsType = activeTab.value === 'currency' ? 2 : activeTab.value === 'equipment' ? 3 : activeTab.value === 'market' ? 5 : 1;
  stopping.value = false;
  notice.value = "";
  progress.value = {
    stage: "list",
    done: 0,
    total: 0,
    message: "正在连接官网…",
  };
  timer = setInterval(
    () =>
      refreshSnapshot().catch((e) => showNotice(`读取缓存失败：${String(e)}`)),
    2000,
  );
  try {
    const operation = invoke<Summary>("crawl", {
      serverId: filters.server,
      force: goodsType === 2 ? false : force.value,
      goodsType,
    });
    force.value = false;
    const result = await operation;
    if (result.cancelled)
      progress.value = {
        ...progress.value,
        message: "已暂停，缓存已保留。再次拉取可继续获取详情。",
      };
    if (result.failed)
      showNotice(
        `${result.failed} 个详情获取失败，再次拉取会重试；已缓存的详情已保留。${result.last_error ? `最近错误：${result.last_error}` : ""}`,
        "warning",
      );
  } catch (e) {
    showNotice(String(e));
    progress.value = {
      ...progress.value,
      stage: "error",
      message: "拉取未完成，可重试",
    };
  } finally {
    clearInterval(timer);
    try {
      await refreshSnapshot();
    } catch (e) {
      showNotice(`读取缓存失败：${String(e)}`);
    }
    busy.value = false;
    stopping.value = false;
  }
}
async function cancel() {
  try {
    await invoke("cancel_crawl");
    stopping.value = true;
  } catch (e) {
    showNotice(`暂停失败：${String(e)}`);
  }
}
watch(
  () => filters.server,
  (value) => {
    localStorage.setItem("cyg-server", value);
    viewerOpen.value = false;
    selectedCode.value = null;
  },
);
watch(
  filters,
  () => {
    resetRows();
  },
  { deep: true },
);
watch(sortValue, () => {
  if (["price", ...tableFields].includes(filters.sort))
    table.value?.sort(filters.sort, filters.desc ? "descending" : "ascending");
  else table.value?.clearSort();
});
watch(viewMode, (value) => {
  localStorage.setItem("cyg-view", value);
});
onMounted(async () => {
  if (!desktop) {
    initializing.value = false;
    showNotice("浏览器仅预览界面，请打开桌面应用获取真实数据。", "info");
    return;
  }
  try {
    unlisten = await listen<Progress>("crawl-progress", (event) => {
      progress.value = event.payload;
      if (event.payload.stage === "error" || event.payload.stage === "detail-error")
        showNotice(event.payload.message, event.payload.stage === "detail-error" ? "warning" : "error");
    });
    await refreshSnapshot();
    ensureServer();
    await refreshAreas();
  } catch (e) {
    showNotice(`初始化失败：${String(e)}`);
  } finally {
    initializing.value = false;
  }
});
onBeforeUnmount(() => {
  clearInterval(timer);
  unlisten?.();
});
</script>

<template>
  <div class="application">
    <header class="app-header">
      <div class="brand">
        <div class="brand-mark">
          <el-icon :size="23"><Filter /></el-icon>
        </div>
        <div>
          <h1>畅易阁角色筛选</h1>
          <p>新水浒Q传</p>
        </div>
      </div>
      <div v-show="activeTab !== 'spirit'" class="source-bar">
        <div class="server-control">
          <el-form-item label="查询区服" class="server-field">
            <el-cascader
              v-model="serverPath"
              :options="regionOptions"
              filterable
              :disabled="busy || initializing || areasLoading"
              placeholder="搜索大区、区服名称"
              popper-class="server-cascader"
            />
          </el-form-item>
          <el-tooltip
            :content="
              areasLoading ? '正在刷新官网区服' : `${areaSource} · 点击刷新`
            "
            ><el-button
              id="refresh-areas"
              :icon="Refresh"
              :loading="areasLoading"
              :disabled="busy || initializing || !desktop"
              aria-label="刷新区服列表"
              @click="refreshAreas"
          /></el-tooltip>
        </div>
        <div class="source-divider" />
        <el-checkbox v-if="activeTab !== 'currency'" v-model="force" :disabled="busy" @change="changeFullRefresh">全量刷新详情</el-checkbox>
        <el-button
          v-if="busy"
          id="cancel"
          :icon="VideoPause"
          :loading="stopping"
          @click="cancel"
          >{{ stopping ? "正在暂停" : "暂停拉取" }}</el-button
        >
        <el-button
          v-else
          id="sync"
          type="primary"
          :icon="Download"
          :disabled="!desktop || initializing || areasLoading || !canSync"
          :title="!canSync ? '请先成功获取官网区服并选择区服' : '拉取当前区服数据'"
          @click="sync"
          >拉取 / 更新</el-button
        >
      </div>
    </header>
    <el-tabs v-model="activeTab" class="application-tabs">
      <el-tab-pane label="角色筛选" name="roles">
    <main class="workspace">
      <aside class="filter-panel" aria-label="角色筛选条件">
        <div class="panel-title">
          <h2>
            <el-icon><Filter /></el-icon>筛选条件
          </h2>
          <el-button id="reset" link type="primary" @click="reset"
            >重置</el-button
          >
        </div>
        <el-scrollbar class="filter-scroll">
          <el-collapse v-model="sections">
            <el-collapse-item name="basic" title="基本条件">
              <el-form
                @submit.prevent
                label-position="top"
                :hide-required-asterisk="true"
              >
                <el-form-item label="价格范围（元）"
                  ><RangeInput v-model="filters.price" label="价格" :min="0"
                /></el-form-item>
                <el-form-item label="职业（可多选）">
                  <el-checkbox-group
                    v-model="filters.jobs"
                    class="job-checkboxes"
                    aria-label="职业筛选"
                  >
                    <el-checkbox v-for="job in jobs" :key="job" :value="job">{{
                      job
                    }}</el-checkbox>
                  </el-checkbox-group>
                </el-form-item>
                <el-form-item label="商品状态"
                  ><el-radio-group
                    v-model="filters.status"
                    class="status-choice"
                    ><el-radio-button value="">全部</el-radio-button
                    ><el-radio-button value="2">在售</el-radio-button
                    ><el-radio-button value="1"
                      >公示中</el-radio-button
                    ></el-radio-group
                  ></el-form-item
                >
              </el-form>
            </el-collapse-item>
            <el-collapse-item name="equipment" title="装备要求">
              <el-radio-group
                v-model="filters.matchAll"
                class="equipment-mode"
                aria-label="装备条件匹配方式"
              >
                <el-radio :value="true">全部满足</el-radio>
                <el-radio :value="false">满足任一</el-radio>
              </el-radio-group>
              <el-checkbox-group
                v-model="filters.equipment"
                class="equipment-checkboxes"
                aria-label="装备筛选"
              >
                <el-checkbox v-for="tag in equipTags" :key="tag" :value="tag">{{
                  tag
                }}</el-checkbox>
              </el-checkbox-group>
            </el-collapse-item>
            <el-collapse-item name="attributes"
              ><template #title
                ><span>角色属性</span
                ><span class="section-note">数值仅设最低值</span></template
              >
              <el-form
                @submit.prevent
                label-position="left"
                label-width="112px"
                class="attribute-minimums"
              >
                <el-form-item v-for="key in minimumNames" :key="key">
                  <template #label>
                    <span class="attribute-label" :title="key">{{ key }}</span>
                    <el-button
                      v-if="!primary.includes(key)"
                      link
                      :icon="Close"
                      :aria-label="`移除${key}筛选`"
                      @click="delete filters.minimums[key]"
                    />
                  </template>
                  <el-input-number
                    :model-value="filters.minimums[key] ?? undefined"
                    :controls="false"
                    placeholder="最低"
                    :aria-label="`${key}最低值`"
                    @update:model-value="filters.minimums[key] = $event ?? null"
                  />
                </el-form-item>
              </el-form>
              <div class="add-attribute">
                <el-select
                  v-model="newAttribute"
                  filterable
                  clearable
                  placeholder="搜索其他属性"
                  aria-label="其他属性名称"
                  ><el-option
                    v-for="key in extraAttributes"
                    :key="key"
                    :value="key"
                    :label="key" /></el-select
                ><el-button
                  id="add-attr"
                  :icon="Plus"
                  :disabled="!newAttribute"
                  aria-label="添加属性筛选"
                  @click="addAttribute"
                />
              </div>
            </el-collapse-item>
          </el-collapse>
          <p class="filter-tip">
            <el-icon><Box /></el-icon>设置属性条件后，缺少该属性的角色会被排除。
          </p>
        </el-scrollbar>
      </aside>
      <section class="results" aria-label="筛选结果" @scroll.capture.passive="onScroll">
        <div class="results-header">
          <div class="results-heading">
            <div class="results-title">
              <h2>角色列表</h2>
            </div>
            <p>
              {{ server?.area }}<el-icon><ArrowRight /></el-icon
              ><span :title="server?.name">{{ server?.name }}</span>
            </p>
          </div>
          <el-radio-group
            v-model="viewMode"
            size="small"
            class="view-switch"
            aria-label="结果显示方式"
          >
            <el-radio-button value="cards">卡片</el-radio-button>
            <el-radio-button value="list">表格</el-radio-button>
          </el-radio-group>
        </div>
        <el-alert
          v-if="error"
          :title="error"
          type="error"
          show-icon
          :closable="false"
        />
        <div class="data-surface">
          <div class="table-toolbar">
            <el-input
              id="keyword"
              v-model="filters.keyword"
              :prefix-icon="Search"
              clearable
              placeholder="搜索角色名称"
              aria-label="搜索角色名称"
              class="name-search"
            />
            <div class="sort-control">
              <span>排序</span
              ><el-select v-model="sortValue" aria-label="排序方式"
                ><el-option label="价格 · 低到高" value="price:asc" /><el-option
                  label="价格 · 高到低"
                  value="price:desc" /><el-option
                  label="上架时间 · 最新"
                  value="listed_at:desc" /><el-option
                  label="剩余时间 · 最短"
                  value="offline_at:asc" /><el-option-group label="按角色属性"
                  ><template v-for="key in attributes" :key="key"
                    ><el-option
                      :label="`${key} · 高到低`"
                      :value="`${key}:desc`" /><el-option
                      :label="`${key} · 低到高`"
                      :value="`${key}:asc`" /></template></el-option-group
              ></el-select>
            </div>
          </div>
          <div v-if="activeTags.length" class="active-filters">
            <span>已选</span
            ><el-tag
              v-for="tag in activeTags"
              :key="tag.key"
              closable
              effect="plain"
              @close="removeTag(tag.key)"
              >{{ tag.label }}</el-tag
            ><el-button link type="primary" @click="reset">清空条件</el-button>
          </div>
          <el-table
            v-if="viewMode === 'list'"
            ref="table"
            :data="visibleRows"
            row-key="code"
            height="100%"
            stripe
            highlight-current-row
            :current-row-key="selectedCode ?? undefined"
            class="role-table"
            :default-sort="
              ['price', ...tableFields].includes(filters.sort)
                ? {
                    prop: filters.sort,
                    order: filters.desc ? 'descending' : 'ascending',
                  }
                : undefined
            "
            :sort-orders="['ascending', 'descending']"
            @sort-change="tableSort"
            @row-click="selectRole"
          >
            <el-table-column
              label="对比"
              width="52"
              align="center"
              fixed="left"
            >
              <template #default="{ row }: { row: Role }">
                <el-checkbox
                  :model-value="compareCodes.includes(row.code)"
                  :disabled="
                    compareCodes.length === 2 &&
                    !compareCodes.includes(row.code)
                  "
                  :aria-label="`对比${row.name}`"
                  @click.stop
                  @change="toggleCompare(row.code)"
                />
              </template>
            </el-table-column>
            <el-table-column label="角色 / 职业" min-width="160" fixed="left">
              <template #default="{ row }: { row: Role }">
                <el-button
                  link
                  class="role-name"
                  :title="row.name"
                  @click.stop="selectRole(row)"
                  >{{ row.name || "未命名角色" }}</el-button
                >
                <div class="table-role-meta">
                  <span>{{ row.job || "职业未知" }}</span>
                  <el-tag
                    class="vip-badge"
                    type="warning"
                    effect="light"
                    size="small"
                    :title="`VIP等级：${row.detail?.attrs['VIP等级']?.raw || '未知'}`"
                    >VIP {{ row.detail?.attrs["VIP等级"]?.num ?? "—" }}</el-tag
                  >
                  <el-tag
                    :type="row.status === 1 ? 'warning' : 'info'"
                    size="small"
                    effect="plain"
                    >{{
                      row.status === 1
                        ? "公示中"
                        : row.status === 2
                          ? "在售"
                          : "状态未知"
                    }}</el-tag
                  >
                </div>
              </template>
            </el-table-column>
            <el-table-column
              prop="price"
              label="价格（元）"
              min-width="100"
              align="center"
              sortable="custom"
            >
              <template #default="{ row }: { row: Role }"
                ><span class="price">{{ fmt(row.price) }}</span></template
              >
            </el-table-column>
            <el-table-column
              v-for="key in tableFields"
              :key="key"
              :prop="key"
              :label="attrLabels[key] || key"
              :min-width="key === '等级' ? 70 : 100"
              align="center"
              sortable="custom"
            >
              <template #default="{ row }: { row: Role }">
                <span
                  :class="{ missing: numeric(row, key) === null }"
                  :title="row.detail?.attrs[key]?.raw"
                  >{{ fmt(numeric(row, key)) }}</span
                >
              </template>
            </el-table-column>
            <el-table-column width="54" fixed="right">
              <template #default="{ row }: { row: Role }"
                ><el-button link type="primary" @click.stop="selectRole(row)"
                  >详情</el-button
                ></template
              >
            </el-table-column>
            <template #empty>
              <el-empty
                :image-size="70"
                :description="
                  busy
                    ? '正在获取角色，列表将逐步显示'
                    : !scoped.length
                      ? '选择区服后拉取角色列表'
                      : error
                        ? '请先修正筛选区间'
                        : '没有符合条件的角色'
                "
              />
            </template>
          </el-table>
          <el-scrollbar v-else class="role-scroll">
            <div v-if="visibleRows.length" class="role-grid">
              <template v-for="row in visibleRows" :key="row.code">
                <article
                  class="role-summary"
                  :class="{ 'is-selected': selectedCode === row.code }"
                  @click="selectRole(row)"
                  :aria-label="row.name || '未命名角色'"
                >
                  <div class="role-summary-header">
                    <el-button
                      link
                      class="role-name"
                      :title="row.name"
                      @click.stop="selectRole(row)"
                    >
                      {{ row.name || "未命名角色" }}
                    </el-button>
                    <span class="price">{{
                      row.price === null ? "—" : `¥ ${fmt(row.price)}`
                    }}</span>
                  </div>
                  <div class="role-identity">
                    <span class="role-job">{{ row.job || "职业未知" }}</span>
                    <span class="role-level">{{
                      numeric(row, "等级") === null
                        ? "等级未知"
                        : `${fmt(numeric(row, "等级"))}级`
                    }}</span>
                    <el-tag
                      :type="row.status === 1 ? 'warning' : 'info'"
                      size="small"
                      effect="plain"
                    >
                      {{
                        row.status === 1
                          ? "公示中"
                          : row.status === 2
                            ? "在售"
                            : "状态未知"
                      }}
                    </el-tag>
                    <el-checkbox
                      class="card-compare"
                      :model-value="compareCodes.includes(row.code)"
                      :disabled="
                        compareCodes.length === 2 &&
                        !compareCodes.includes(row.code)
                      "
                      :aria-label="`对比${row.name}`"
                      @click.stop
                      @change="toggleCompare(row.code)"
                      >对比</el-checkbox
                    >
                    <el-button link type="primary" @click.stop="selectRole(row)"
                      >详情</el-button
                    >
                  </div>
                  <dl class="role-metrics">
                    <div
                      v-for="key in listFields.slice(1)"
                      :key="key"
                      class="role-metric"
                    >
                      <dt :title="key">{{ attrLabels[key] || key }}</dt>
                      <dd
                        :class="{ missing: numeric(row, key) === null }"
                        :title="row.detail?.attrs[key]?.raw"
                      >
                        {{
                          key === "VIP等级"
                            ? row.detail?.attrs[key]?.raw || "—"
                            : fmt(numeric(row, key))
                        }}
                      </dd>
                    </div>
                  </dl>
                  <el-tooltip
                    v-if="!row.detail || row.detail_error || row.needs_refresh"
                    :content="
                      row.detail_error ||
                      (row.detail_at
                        ? `获取于 ${time(row.detail_at)}`
                        : '等待获取角色详情')
                    "
                  >
                    <el-tag
                      class="role-detail-state"
                      :type="detailState(row).type"
                      effect="light"
                      size="small"
                    >
                      {{ detailState(row).text }}
                    </el-tag>
                  </el-tooltip>
                </article>
              </template>
            </div>
            <el-empty
              v-else
              :image-size="90"
              :description="
                busy
                  ? '正在获取角色，列表将逐步显示'
                  : !scoped.length
                    ? '从一个区服开始，获取你的角色列表'
                    : error
                      ? '请先修正筛选区间'
                      : '没有符合条件的角色'
              "
              ><p v-if="!scoped.length" class="empty-hint">
                选择区服后点击右上角「拉取 / 更新」<br />获取一次，即可在本地反复筛选
              </p>
              <el-button v-else-if="activeTags.length" @click="reset"
                >清空筛选条件</el-button
              ></el-empty
            >
          </el-scrollbar>
          <div
            v-if="compareRoles.length"
            class="compare-bar"
            aria-label="已选对比角色"
          >
            <span>角色对比 {{ compareRoles.length }}/2</span>
            <div class="compare-names">
              <el-tag
                v-for="role in compareRoles"
                :key="role.code"
                effect="plain"
                :title="`${role.name} · ${role.job} · ${role.server_id}`"
                ><span>{{ role.name }}</span
                ><el-button
                  link
                  :icon="Close"
                  :aria-label="`移除对比${role.name}`"
                  @click="toggleCompare(role.code)"
              /></el-tag>
              <span v-if="compareRoles.length === 1" class="compare-hint"
                >再选择一个角色</span
              >
            </div>
            <el-button link @click="compareCodes = []">清空</el-button>
            <el-button
              type="primary"
              :disabled="compareRoles.length !== 2"
              @click="compareSelected"
              >开始对比</el-button
            >
          </div>
          <div class="table-footer">
            <div class="cache-summary">
              <span
                >已缓存 <b>{{ scoped.length }}</b></span
              ><span
                >已有详情 <b>{{ detailCount }}</b></span
              ><el-tooltip
                :content="
                  checkedAt
                    ? `列表最近检查：${time(checkedAt)}`
                    : '尚未获取该区服数据'
                "
                ><el-tag
                  :type="checkedAt ? 'success' : 'info'"
                  effect="light"
                  size="small"
                  >{{ checkedAt ? "本地缓存" : "尚未同步" }}</el-tag
                ></el-tooltip
              >
            </div>
            <div class="list-load-status">
              <span role="status">已显示 {{ visibleRows.length }} / {{ shown.length }} 条</span>
              <el-button v-if="hasMore" size="small" :loading="loadingRows" @click="loadMore">加载更多</el-button>
              <span v-else>{{ shown.length ? '已全部显示' : '暂无结果' }}</span>
            </div>
          </div>
        </div>
      </section>
    </main>
      </el-tab-pane>
      <el-tab-pane label="星灵选购" name="market">
        <SpiritMarket :items="spirits" :server="filters.server" :busy="busy" @simulate="simulateSpirit" @error="showNotice" />
      </el-tab-pane>
      <el-tab-pane label="装备选购" name="equipment">
        <EquipmentMarket :items="equipment" :server="filters.server" :busy="busy" @error="showNotice" />
      </el-tab-pane>
      <el-tab-pane label="游戏币" name="currency">
        <CurrencyMarket :items="currencies" :server="filters.server" :busy="busy" @error="showNotice" />
      </el-tab-pane>
      <el-tab-pane label="星灵 / 灵宝计算器" name="spirit">
        <SpiritCalculator :simulation="spiritSimulation" />
      </el-tab-pane>
    </el-tabs>
    <footer v-show="activeTab !== 'spirit'" class="status-bar">
      <div class="task-state">
        <span class="status-dot" :class="{ running: busy }" /><span
          role="status"
          >{{
            stopping
              ? "正在暂停，等待当前请求结束（最多约 25 秒）…"
              : progress.message
          }}</span
        ><span v-if="busy && progress.total" class="progress-count"
          >{{ progress.done }} / {{ progress.total }}</span
        ><el-progress
          v-if="busy"
          :percentage="percent"
          :indeterminate="!progress.total"
          :duration="2"
          :show-text="false"
          :stroke-width="4"
        />
      </div>
      <span class="status-note"
        >{{ areaSource }} · 价格及交易状态以官网为准</span
      >
    </footer>
    <Teleport to="body">
      <el-alert v-if="notice" class="error-notice" :type="noticeType" show-icon
        :title="noticeType === 'error' ? '操作失败' : '提示'" @close="notice = ''">
        <textarea ref="noticeText" class="notice-text" :value="notice" readonly aria-label="完整提示及错误原因" />
        <div class="notice-actions">
          <el-button size="small" @click="copyNotice">复制提示</el-button>
          <el-button v-if="areasError && !areasReady" size="small" :loading="areasLoading"
            :disabled="busy || initializing" @click="refreshAreas">重试获取区服</el-button>
          <span role="status">{{ copyStatus }}</span>
        </div>
      </el-alert>
    </Teleport>
    <OfficialViewer v-model="viewerOpen" :roles="viewerRoles" />
  </div>
</template>
