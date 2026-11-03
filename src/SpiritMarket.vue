<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import type { TableInstance } from 'element-plus';
import { Search } from '@element-plus/icons-vue';
import RangeInput from './RangeInput.vue';
import OfficialViewer from './OfficialViewer.vue';
import { applyFilters, defaults, numeric, rangeError, type Role } from './filter';
import { marketSpirit, spiritAptitudes, spiritPanelFields } from './spirit-market';
import { qualityRates } from './spirit';
import { simulationFromSpirit, type SpiritSimulation } from './spirit-simulation';
import { toggleComparison } from './comparison';
import { useScrollRows } from './scroll-rows';

const props = defineProps<{ items: Role[]; server: string; busy: boolean }>();
const emit = defineEmits<{ simulate: [simulation: SpiritSimulation]; error: [message: string] }>();
const filters = reactive(defaults(props.server));
const quality = ref('');
const growth = ref('');
const starGeneral = ref(true);
const view = ref('list');
const columns = ref('bonus');
const table = ref<TableInstance>();
const sections = ref(['basic', 'minimums']);
const selection = ref<string[]>([]);
const selected = ref<string | null>(null);
const drawer = ref(false);
const viewerOpen = ref(false);
const viewerItems = ref<Role[]>([]);
const fmt = (n: number | null) => n === null ? '—' : new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 3 }).format(n);
const rows = computed(() => props.items.filter(item => item.server_id === props.server).map(item => marketSpirit(item, starGeneral.value)));
const source = (row: Role) => props.items.find(item => item.code === row.code);
const error = computed(() => rangeError(filters));
const shown = computed(() => applyFilters(rows.value.filter(row =>
  (!quality.value || row.detail?.attrs.资质?.raw === quality.value)
  && (!growth.value || row.detail?.attrs.成长?.raw === growth.value)), filters));
const { visibleRows, hasMore, loading: loadingRows, loadMore, onScroll, resetRows } = useScrollRows(shown);
const pair = computed(() => rows.value.filter(row => selection.value.includes(row.code)));
const current = computed(() => rows.value.find(row => row.code === selected.value));
const fields = computed(() => columns.value === 'bonus'
  ? spiritPanelFields.map(field => ({ key: `加成${field.name}`, label: `${field.name}加成` }))
  : spiritAptitudes.map(key => ({ key, label: key.replace('体力', '体质') })));
const minima = ['等级', '强化', '慧心', ...spiritAptitudes, ...spiritPanelFields.map(field => `加成${field.name}`)];
const sorts = ['price', '等级', '强化', '慧心', ...spiritAptitudes, ...spiritPanelFields.map(field => `加成${field.name}`)];
const sortValue = computed({ get: () => `${filters.sort}:${filters.desc ? 'desc' : 'asc'}`, set: value => {
  const [key, direction] = value.split(':'); filters.sort = key; filters.desc = direction === 'desc';
} });
function tableSort({ prop, order }: { prop: string; order: string | null }) {
  if (order && sorts.includes(prop)) { filters.sort = prop; filters.desc = order === 'descending'; }
}
watch([sortValue, fields, view], () => {
  if (['price', '等级', '强化', '慧心', ...fields.value.map(field => field.key)].includes(filters.sort))
    table.value?.sort(filters.sort, filters.desc ? 'descending' : 'ascending');
  else table.value?.clearSort();
}, { flush: 'post' });
function reset() { Object.assign(filters, defaults(props.server)); quality.value = ''; growth.value = ''; }
function inspect(row: Role) { selected.value = row.code; drawer.value = true; }
function simulate(row: Role) {
  const original = source(row);
  if (!original) return;
  try { emit('simulate',simulationFromSpirit(original,starGeneral.value)); drawer.value = false; }
  catch (error) { emit('error',String(error)); }
}
function official(items: Role[]) { viewerItems.value = items; viewerOpen.value = true; }
watch(() => props.server, value => { filters.server = value; selection.value = []; drawer.value = false; viewerOpen.value = false; });
watch([filters, quality, growth, starGeneral], resetRows, { deep: true });
watch(rows, value => { selection.value = selection.value.filter(code => value.some(row => row.code === code)); });
</script>

<template>
  <main class="workspace spirit-market">
    <aside class="filter-panel" aria-label="星灵筛选条件">
      <div class="panel-title"><h2>星灵筛选</h2><el-button link type="primary" @click="reset">重置</el-button></div>
      <el-scrollbar class="filter-scroll">
        <el-collapse v-model="sections">
          <el-collapse-item name="basic" title="基本条件">
            <el-form label-position="top" @submit.prevent>
              <el-form-item label="价格范围（元）"><RangeInput v-model="filters.price" label="星灵价格" :min="0" /></el-form-item>
              <el-form-item label="资质品质"><el-select v-model="quality" placeholder="不限" aria-label="星灵资质品质"><el-option label="不限" value="" /><el-option v-for="(_, name) in qualityRates" :key="name" :label="name" :value="name" /></el-select></el-form-item>
              <el-form-item label="成长品质"><el-select v-model="growth" placeholder="不限" aria-label="星灵成长品质"><el-option label="不限" value="" /><el-option v-for="(_, name) in qualityRates" :key="name" :label="name" :value="name" /></el-select></el-form-item>
              <el-form-item label="商品状态"><el-radio-group v-model="filters.status" aria-label="星灵商品状态"><el-radio-button value="">全部</el-radio-button><el-radio-button value="2">在售</el-radio-button><el-radio-button value="1">公示中</el-radio-button></el-radio-group></el-form-item>
            </el-form>
          </el-collapse-item>
          <el-collapse-item name="minimums" title="属性与加成最低值">
            <el-form class="attribute-minimums" label-position="left" label-width="112px" @submit.prevent>
              <el-form-item v-for="key in minima" :key="key" :label="key.replace('体力', '体质')">
                <el-input-number :model-value="filters.minimums[key] ?? undefined" :min="0" :controls="false" placeholder="最低" :aria-label="`星灵${key}最低值`" @update:model-value="filters.minimums[key] = $event ?? null" />
              </el-form-item>
            </el-form>
          </el-collapse-item>
        </el-collapse>
        <p class="filter-tip">缺少属性的星灵不会满足该项最低值条件。</p>
      </el-scrollbar>
    </aside>
    <section class="results" aria-label="星灵选购列表" @scroll.capture.passive="onScroll">
      <div class="results-header">
        <div class="market-heading"><h2>星灵选购</h2><el-checkbox v-model="starGeneral">已转星将（加成 × 1.1）</el-checkbox></div>
        <el-radio-group v-model="view" size="small" aria-label="星灵显示方式"><el-radio-button value="cards">卡片</el-radio-button><el-radio-button value="list">表格</el-radio-button></el-radio-group>
      </div>
      <el-alert v-if="error" :title="error" type="error" :closable="false" show-icon />
      <div class="data-surface">
        <div class="table-toolbar market-toolbar">
          <el-input v-model="filters.keyword" :prefix-icon="Search" clearable placeholder="搜索星灵名称" aria-label="搜索星灵名称" class="name-search" />
          <el-radio-group v-model="columns" size="small" aria-label="星灵属性显示"><el-radio-button value="bonus">角色加成</el-radio-button><el-radio-button value="aptitude">五项资质</el-radio-button></el-radio-group>
          <el-select v-model="sortValue" aria-label="星灵排序方式" class="market-sort">
            <template v-for="key in sorts" :key="key"><el-option :value="`${key}:asc`" :label="`${key === 'price' ? '价格' : key} · 低到高`" /><el-option :value="`${key}:desc`" :label="`${key === 'price' ? '价格' : key} · 高到低`" /></template>
          </el-select>
        </div>
        <el-table v-if="view === 'list'" ref="table" :data="visibleRows" :default-sort="{ prop: filters.sort, order: filters.desc ? 'descending' : 'ascending' }" :sort-orders="['ascending', 'descending']" @sort-change="tableSort" height="100%" row-key="code" stripe class="role-table" @row-click="inspect">
          <el-table-column label="对比" width="52" fixed="left" align="center"><template #default="{ row }"><el-checkbox :model-value="selection.includes(row.code)" :disabled="selection.length === 2 && !selection.includes(row.code)" :aria-label="`对比星灵${row.name}`" @click.stop @change="selection = toggleComparison(selection, row.code)" /></template></el-table-column>
          <el-table-column label="星灵 / 品质" min-width="175" fixed="left"><template #default="{ row }">
            <el-button link class="role-name" :title="row.name" @click.stop="inspect(row)">{{ row.name }}</el-button>
            <div class="market-meta"><span>{{ row.detail?.attrs.资质?.raw ?? '—' }} / {{ row.detail?.attrs.成长?.raw ?? '—' }}</span><el-tag size="small" effect="plain">{{ row.status === 1 ? '公示中' : '在售' }}</el-tag></div>
            <span v-if="!source(row)?.detail && !source(row)?.detail_error" class="market-pending">待获取详情</span>
            <span v-else-if="source(row)?.needs_refresh && !source(row)?.detail_error" class="market-pending">待刷新，显示旧缓存</span>
            <span v-if="source(row)?.detail_error" class="market-detail-error">{{ source(row)?.detail ? '详情更新失败，显示旧缓存' : '详情获取失败，可重试' }}</span>
          </template></el-table-column>
          <el-table-column prop="price" sortable="custom" label="价格（元）" min-width="95" align="center"><template #default="{ row }"><span class="price">{{ fmt(row.price) }}</span></template></el-table-column>
          <el-table-column v-for="key in ['等级', '强化', '慧心']" :key="key" :prop="key" sortable="custom" :label="key" min-width="64" align="center"><template #default="{ row }">{{ fmt(numeric(row, key)) }}</template></el-table-column>
          <el-table-column v-for="field in fields" :key="field.key" :prop="field.key" sortable="custom" :label="field.label" min-width="102" align="center"><template #default="{ row }"><span :class="{ missing: numeric(row, field.key) === null }">{{ fmt(numeric(row, field.key)) }}</span></template></el-table-column>
          <el-table-column width="116" fixed="right"><template #default="{ row }"><el-button link type="primary" :disabled="!source(row)?.detail" @click.stop="simulate(row)">模拟</el-button><el-button link type="primary" @click.stop="inspect(row)">详情</el-button></template></el-table-column>
          <template #empty><el-empty :image-size="70" :description="busy ? '正在获取星灵，请稍候' : !rows.length ? '选择区服后，点击顶部「拉取 / 更新」获取星灵' : '没有符合条件的星灵'" /></template>
        </el-table>
        <el-scrollbar v-else class="role-scroll">
          <div v-if="visibleRows.length" class="role-grid">
            <article v-for="row in visibleRows" :key="row.code" class="role-summary">
              <div class="role-summary-header"><el-button link class="role-name" @click="inspect(row)">{{ row.name }}</el-button><span class="price">¥ {{ fmt(row.price) }}</span></div>
              <div class="market-meta">{{ row.detail?.attrs.资质?.raw ?? '—' }} / {{ row.detail?.attrs.成长?.raw ?? '—' }} · {{ fmt(numeric(row, '等级')) }}级 · {{ fmt(numeric(row, '强化')) }}星 · 慧心 {{ fmt(numeric(row, '慧心')) }} · {{ row.status === 1 ? '公示中' : '在售' }}</div>
              <dl class="role-metrics"><div v-for="field in fields" :key="field.key" class="role-metric"><dt>{{ field.label }}</dt><dd>{{ fmt(numeric(row, field.key)) }}</dd></div></dl>
              <div class="market-card-actions"><el-checkbox :model-value="selection.includes(row.code)" :disabled="selection.length === 2 && !selection.includes(row.code)" :aria-label="`对比星灵${row.name}`" @change="selection = toggleComparison(selection, row.code)">加入对比</el-checkbox><el-button link type="primary" :disabled="!source(row)?.detail" @click="simulate(row)">模拟</el-button><el-button link type="primary" @click="inspect(row)">计算明细</el-button></div>
              <p v-if="source(row)?.detail_error" class="market-detail-error">详情获取失败，再次点击顶部「拉取 / 更新」即可重试</p>
              <p v-else-if="!source(row)?.detail" class="market-pending">待获取详情后计算加成</p>
              <p v-else-if="source(row)?.needs_refresh" class="market-pending">待刷新，显示旧缓存</p>
            </article>
          </div>
          <el-empty v-else :description="busy ? '正在获取星灵，请稍候' : !rows.length ? '选择区服后，点击顶部「拉取 / 更新」获取星灵' : '没有符合条件的星灵'" />
        </el-scrollbar>
        <div v-if="pair.length" class="compare-bar"><span>星灵对比 {{ pair.length }}/2</span><div class="compare-names"><el-tag v-for="item in pair" :key="item.code" closable @close="selection = selection.filter(code => code !== item.code)">{{ item.name }}</el-tag></div><el-button link @click="selection = []">清空</el-button><el-button type="primary" :disabled="pair.length !== 2" @click="official(pair)">官网对比</el-button></div>
        <div class="table-footer"><div class="cache-summary"><span>已缓存 <b>{{ rows.length }}</b></span><span>已有详情 <b>{{ items.filter(item => item.server_id === server && item.detail).length }}</b></span></div><div class="list-load-status"><span role="status">已显示 {{ visibleRows.length }} / {{ shown.length }} 条</span><el-button v-if="hasMore" size="small" :loading="loadingRows" @click="loadMore">加载更多</el-button><span v-else>{{ shown.length ? '已全部显示' : '暂无结果' }}</span></div></div>
      </div>
    </section>
  </main>
  <el-drawer v-model="drawer" title="星灵属性与计算明细" size="640px" destroy-on-close>
    <template v-if="current">
      <div class="market-detail-heading"><h2>{{ current.name }}</h2><strong class="price">¥ {{ fmt(current.price) }}</strong><el-button type="primary" :disabled="!source(current)?.detail" @click="simulate(current)">模拟</el-button><el-button @click="official([current])">官网详情</el-button></div>
      <el-alert v-if="source(current)?.detail_error" :title="source(current)?.detail_error ?? ''" type="warning" :closable="false" show-icon />
      <p class="market-detail-note">{{ current.detail?.attrs.资质?.raw ?? '—' }}资质 + {{ current.detail?.attrs.成长?.raw ?? '—' }}成长 + {{ fmt(numeric(current, '强化')) }}星强化 = {{ fmt(numeric(current, '加成比例')) }}%；转星将系数 {{ starGeneral ? '1.1' : '1' }}。</p>
      <el-table :data="spiritPanelFields.map(field => ({ name: field.name, panel: numeric(current!, field.source), bonus: numeric(current!, `加成${field.name}`) }))">
        <el-table-column prop="name" label="属性" /><el-table-column label="官网当前面板" align="right"><template #default="{ row }">{{ fmt(row.panel) }}</template></el-table-column><el-table-column label="角色获得加成" align="right"><template #default="{ row }"><strong>{{ fmt(row.bonus) }}</strong></template></el-table-column>
      </el-table>
      <h3 class="market-detail-subtitle">五项资质</h3><el-descriptions :column="2" border><el-descriptions-item v-for="key in spiritAptitudes" :key="key" :label="key.replace('体力', '体质')">{{ fmt(numeric(current, key)) }}</el-descriptions-item></el-descriptions>
      <p class="market-detail-note">使用官网当前面板，慧心、加点及灵宝效果不再重复叠加。结果保留 3 位小数展示，未模拟重新加点或升级。缺失数据以 — 显示。</p>
      <p class="market-detail-note">详情缓存：{{ current.detail_at ? new Date(current.detail_at * 1000).toLocaleString('zh-CN', { hour12: false }) : '尚未获取' }}。购买前请在官网核对。</p>
    </template>
  </el-drawer>
  <OfficialViewer v-model="viewerOpen" :roles="viewerItems" kind="星灵" />
</template>

<style scoped>
.spirit-market { height: 100%; }
.market-heading { display: flex; align-items: center; gap: 20px; }
.market-heading h2 { white-space: nowrap; }
.market-heading .el-checkbox { margin-right: 0; }
.role-grid { align-items: stretch; }
.role-summary { display: flex; flex-direction: column; }
.role-metrics { margin-bottom: 8px; }
.role-metric { padding: 4px 0; }
.market-toolbar { flex-wrap: wrap; }
.market-sort { width: 190px; margin-left: auto; }
.market-meta { font-size: 12px; color: var(--el-text-color-secondary); display: flex; align-items: center; flex-wrap: wrap; gap: 6px; margin: 5px 0; }
.market-detail-error { font-size: 12px; color: var(--el-color-danger); }
.market-pending { font-size: 12px; color: var(--el-text-color-secondary); }
.market-card-actions { display: flex; justify-content: space-between; align-items: center; margin-top: auto; }
.market-detail-heading { display: flex; gap: 16px; align-items: center; flex-wrap: wrap; }
.market-detail-heading h2 { font-size: 18px; margin-right: auto; }
.market-detail-note { line-height: 1.8; color: var(--el-text-color-secondary); margin: 20px 0; font-size: 13px; }
.market-detail-subtitle { font-size: 16px; margin: 28px 0 16px; }
</style>
