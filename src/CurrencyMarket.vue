<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { TableInstance } from 'element-plus';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { Search, Link } from '@element-plus/icons-vue';
import type { Role } from './filter';
import { currencyQuote } from './currency';
import { useScrollRows } from './scroll-rows';
import RangeInput from './RangeInput.vue';
import HighlightText from './HighlightText.vue';

const props = defineProps<{ items: Role[]; server: string; busy: boolean }>();
const emit = defineEmits<{error:[message:string]}>();
const keyword = ref('');
const price = ref<[number|null,number|null]>([null,null]);
const status = ref('');
const sort = ref('rate-desc');
const table = ref<TableInstance>();
const scoped = computed(()=>props.items.filter(row=>row.server_id===props.server).map(currencyQuote));
const priceError = computed(()=>price.value[0]!==null && price.value[1]!==null && price.value[0]>price.value[1]);
const shown = computed(()=>scoped.value.filter(row=>!priceError.value && (!keyword.value.trim()||row.name.includes(keyword.value.trim()))
  && (!status.value||row.status===Number(status.value))
  && (price.value[0]===null || (row.price!==null&&row.price>=price.value[0]))
  && (price.value[1]===null || (row.price!==null&&row.price<=price.value[1])))
  .sort((a,b)=>{
    const av=sort.value==='price'?a.price:a.coinsPerYuan,bv=sort.value==='price'?b.price:b.coinsPerYuan;
    if(av===null)return bv===null?0:1;
    if(bv===null)return -1;
    return sort.value==='rate-desc'?bv-av:av-bv;
  }));
const {visibleRows,hasMore,loading,loadMore,onScroll,resetRows} = useScrollRows(shown);
const fmt = (value:number|null,digits=2)=>value===null?'—':new Intl.NumberFormat('zh-CN',{maximumFractionDigits:digits}).format(value);
function reset(){keyword.value='';price.value=[null,null];status.value='';sort.value='rate-desc';}
watch([keyword,price,status,sort],resetRows,{deep:true});
watch(()=>props.server,reset);
function sortUnitPrice({prop,order}:{prop:string;order:string|null}) {
  if(prop==='coinsPerYuan'&&order)sort.value=order==='ascending'?'rate-asc':'rate-desc';
}
watch(sort,value=>{
  if(value==='price')table.value?.clearSort();
  else table.value?.sort('coinsPerYuan',value==='rate-asc'?'ascending':'descending');
});
async function official(row:Role){try{await invoke('open_role',{code:row.code});}catch(error){emit('error',`打开官网失败：${String(error)}`);}}
</script>

<template>
  <section class="currency-market" aria-label="游戏币列表" @scroll.capture.passive="onScroll">
    <header class="currency-heading"><h2>游戏币 <span>银两</span></h2></header>
    <div class="currency-toolbar">
      <el-input v-model="keyword" :prefix-icon="Search" clearable placeholder="搜索游戏币" aria-label="搜索游戏币" />
      <div class="currency-price"><span>售价（元）</span><RangeInput v-model="price" label="游戏币售价" :min="0" /></div>
      <el-radio-group v-model="status" aria-label="游戏币状态"><el-radio-button value="">全部</el-radio-button><el-radio-button value="2">在售</el-radio-button><el-radio-button value="1">公示中</el-radio-button></el-radio-group>
      <el-select v-model="sort" aria-label="游戏币排序"><el-option value="rate-desc" label="单价 · 高到低" /><el-option value="rate-asc" label="单价 · 低到高" /><el-option value="price" label="总价 · 低到高" /></el-select>
      <el-button link type="primary" @click="reset">重置</el-button>
    </div>
    <el-alert v-if="priceError" title="价格下限不能大于上限" type="error" :closable="false" show-icon />
    <el-table ref="table" :data="visibleRows" :default-sort="{prop:'coinsPerYuan',order:'descending'}" @sort-change="sortUnitPrice" row-key="code" height="100%" stripe class="currency-table">
      <el-table-column label="商品" min-width="175" show-overflow-tooltip><template #default="{row}"><strong><HighlightText :text="row.name" :keyword="keyword" /></strong></template></el-table-column>
      <el-table-column label="数量（亿银两）" min-width="120" align="right"><template #default="{row}">{{ fmt(row.yi,8) }}</template></el-table-column>
      <el-table-column label="总价（元）" min-width="110" align="right"><template #default="{row}"><span class="price">¥ {{ fmt(row.price) }}</span></template></el-table-column>
      <el-table-column label="单价（万银两/元）" prop="coinsPerYuan" sortable="custom" :sort-orders="['descending','ascending']" min-width="230" align="right"><template #default="{row}"><template v-if="row.coinsPerYuan!==null"><strong class="currency-rate">≈ {{ fmt(row.coinsPerYuan/10_000,4) }} 万银两/元</strong></template><span v-else class="currency-unknown">数量单位或售价无法确认</span></template></el-table-column>
      <el-table-column label="状态" min-width="90" align="center"><template #default="{row}"><el-tag :type="row.status===1?'warning':row.status===2?'success':'info'" effect="plain">{{ row.status===1?'公示中':row.status===2?'在售':'状态未知' }}</el-tag></template></el-table-column>
      <el-table-column label="截止时间（北京时间）" min-width="170"><template #default="{row}">{{ (row.status===1?row.listed_at:row.offline_at)||'—' }}</template></el-table-column>
      <el-table-column width="112" align="center"><template #default="{row}"><el-button link type="primary" :icon="Link" :disabled="!isTauri()" @click="official(row)">官网详情</el-button></template></el-table-column>
      <template #empty><el-empty :image-size="70" :description="busy?'正在获取游戏币报价，请稍候':!scoped.length?'选择区服后，点击顶部「拉取 / 更新」获取游戏币':'没有符合条件的游戏币报价'" /></template>
    </el-table>
    <footer v-if="hasMore" class="currency-footer"><el-button size="small" :loading="loading" @click="loadMore">加载更多</el-button></footer>
  </section>
</template>

<style scoped>
.currency-market{display:flex;flex-direction:column;height:100%;min-height:0;background:#fff;color:var(--el-text-color-primary)}.currency-heading{padding:20px 24px 16px}.currency-heading h2{margin:0;font-size:20px}.currency-heading h2 span{font-size:13px;font-weight:400;margin-left:8px;color:var(--el-text-color-secondary)}.currency-toolbar{display:flex;align-items:center;gap:16px;padding:16px 24px;flex-wrap:wrap;border-bottom:1px solid var(--el-border-color-lighter)}.currency-toolbar>.el-input{width:180px}.currency-price{display:flex;align-items:center;gap:10px;font-size:13px;white-space:nowrap}.currency-price :deep(.range-input){width:220px}.currency-toolbar>.el-select{width:190px}.currency-table{flex:1;min-height:0}.currency-table :deep(.cell){white-space:nowrap}.currency-rate{color:#195b50;font-variant-numeric:tabular-nums}.currency-unknown{color:var(--el-text-color-secondary);font-size:12px}.currency-footer{display:flex;gap:16px;align-items:center;padding:12px 24px;border-top:1px solid var(--el-border-color-lighter);font-size:12px;color:var(--el-text-color-regular)}
</style>
