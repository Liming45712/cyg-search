<script setup lang="ts">
import { computed, onUnmounted, reactive, ref, watch } from 'vue';
import HighlightText from './HighlightText.vue';
import { Search, ArrowLeft, Link } from '@element-plus/icons-vue';
import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Role } from './filter';
import { marketEquipment, equipmentAttribute, equipmentMinimum, type Item, type Stat } from './equipment';
import { useScrollRows } from './scroll-rows';
import typeCatalog from './equipment-types.json';
import attributeCatalog from './equipment-attributes.json';
const props = defineProps<{ items: Role[]; server: string; busy: boolean }>();
const emit = defineEmits<{ error: [message: string] }>();
const items = computed(()=>props.items.filter(row=>row.server_id===props.server).map(marketEquipment));
const now = ref(Date.now());
const timer = setInterval(()=>{now.value=Date.now();},60_000);
onUnmounted(()=>clearInterval(timer));
const saleStatus = (item:Item)=>item.status===1?'公示中':item.status===2?'在售':'状态未知';
const endTime = (item:Item)=>item.status===1?item.saleTime:item.status===2?item.offlineTime:undefined;
function remaining(item:Item){
  const end=endTime(item);
  const delta=end?Date.parse(end.replace(' ','T')+'+08:00')-now.value:NaN;
  if(!Number.isFinite(delta))return '剩余时间未知';
  if(delta<=0)return '时间已到，待更新';
  const minutes=Math.floor(delta/60_000),hours=Math.floor(minutes/60)%24,days=Math.floor(minutes/1440);
  return '剩余 '+(days?`${days}天${hours}时`:hours?`${hours}时${minutes%60}分`:minutes?`${minutes}分`:'不足1分钟');
}
const types = ref<string[]>([]);
const keyword = ref('');
const price = reactive<{ min: number | undefined; max: number | undefined }>({min:undefined,max:undefined});
const attribute = ref('暴击率');
const minimum = ref<number>();
const sort = ref('default');
const selected = ref<string[]>([]);
const comparing = ref(false);
const scroll = ref<HTMLElement>();
const categories = computed(()=>[...new Set([...typeCatalog.types.map(entry=>entry.name),...items.value.filter(i=>i.ready).map(i=>i.type)])].sort((a,b)=>a.localeCompare(b,'zh-CN')));
const typeGroups = [
  { name: '宠物', types: ['宠物武器', '宠物饰品'] },
  { name: '凤翼凰羽', types: ['凤翼', '凰羽'] },
  { name: '黄金甲', types: ['黄金甲'] },
  { name: '召唤兽', types: ['召唤兽护甲', '召唤兽护腕', '召唤兽项链'] },
  { name: '常规装备', types: ['武器', '帽子', '面罩', '护手', '腰带', '鞋子', '项链', '首饰', '纹章'] },
];
const groupedTypes = computed(()=>[...typeGroups,{ name: '其他', types: categories.value.filter(name=>!typeGroups.some(group=>group.types.includes(name))) }]);
const attributeNames = computed(()=>[...new Set([...attributeCatalog.attributes.map(entry=>equipmentAttribute(entry.name)),...items.value.flatMap(i=>i.stats.map(s=>s[0]))])]);
const attributeCategories = [
  { name: '基础属性', attributes: ['攻击力', '力量', '心力', '体质', '耐力', '敏捷', '精神力', '灵巧度'] },
  { name: '攻击与伤害', attributes: ['物理伤害', '魔法伤害', '暴击率', '火属性攻击', '土属性攻击', '水属性攻击', '风属性攻击'] },
  { name: '生存与防御', attributes: ['HP', 'SP', '防御力', '物理抗性', '法术抗性', '异常抗性'] },
  { name: '命中与闪避', attributes: ['命中率', '基本命中率', '闪避率', '回避率', '物理必闪', '法术必闪', '异常必闪'] },
  { name: '免疫与破免', attributes: ['激活物理免疫时效果', '激活法术免疫时效果', '激活异常免疫时效果', '激活物理破免时效果', '激活法术破免时效果', '激活异常破免时效果', '物理抗性削弱', '法术抗性削弱', '异常抗性削弱'] },
];
const attributeGroups = computed(()=>[
  ...attributeCategories.map(group=>({ name: group.name, attributes: group.attributes.filter(name=>attributeNames.value.includes(name)) })),
  { name: '其他属性', attributes: attributeNames.value.filter(name=>!attributeCategories.some(group=>group.attributes.includes(name))) },
].filter(group=>group.attributes.length));
const attributeGroup = ref('攻击与伤害');
const activeAttributeGroup = computed(()=>attributeGroups.value.find(group=>group.name===attributeGroup.value)??attributeGroups.value[0]);
const priceError = computed(()=>price.min!==undefined&&price.max!==undefined&&price.min>price.max);
const shown = computed(()=>items.value.filter(i=>(!types.value.length||types.value.includes(i.type))
  && (!keyword.value.trim()||[i.name,...i.effects,...(i.set??[]),...i.stats.map(s=>s.join(' '))].join(' ').includes(keyword.value.trim()))
  && (price.min===undefined||(i.price!==null&&i.price>=price.min)) && (price.max===undefined||(i.price!==null&&i.price<=price.max))
  && equipmentMinimum(i,attribute.value,minimum.value) && !priceError.value)
  .sort((a,b)=>sort.value==='price'?(a.price??Infinity)-(b.price??Infinity):sort.value==='score'?(b.score??-Infinity)-(a.score??-Infinity):a.code.localeCompare(b.code)));
const pair = computed(()=>selected.value.map(code=>items.value.find(i=>i.code===code)).filter((item):item is Item=>!!item));
const { visibleRows, hasMore, loading: loadingRows, loadMore, onScroll, resetRows } = useScrollRows(shown);
const compareFields = computed(()=>{
  const fields: {name:string;index:number}[]=[];
  for(const item of pair.value){const counts:Record<string,number>={}; for(const s of item.stats){const index=counts[s[0]]??0;counts[s[0]]=index+1;if(!fields.some(f=>f.name===s[0]&&f.index===index))fields.push({name:s[0],index});}}
  return fields;
});
const format = (s?: Stat)=>s?`+${s[1]}${s[2]??''}`:'—';
const money = (n:number|null)=>n===null?'—':new Intl.NumberFormat('zh-CN').format(n);
const url = (i:Item)=>`http://cyg.changyou.com/details?goodsCode=${i.code}`;
function reset(){types.value=[];attributeGroup.value='攻击与伤害';keyword.value='';price.min=undefined;price.max=undefined;attribute.value='暴击率';minimum.value=undefined;}
function toggle(code:string){selected.value=selected.value.includes(code)?selected.value.filter(c=>c!==code):selected.value.length<2?[...selected.value,code]:selected.value;}
watch(types,()=>{minimum.value=undefined;});
watch([types,keyword,price,attribute,minimum,sort],()=>{resetRows();if(scroll.value)scroll.value.scrollTop=0;});
watch(()=>props.server,()=>{reset();selected.value=[];comparing.value=false;});
watch(items,()=>{selected.value=selected.value.filter(code=>items.value.some(item=>item.code===code));if(selected.value.length<2)comparing.value=false;});
async function official(item:Item){try { if(isTauri())await invoke('open_role',{code:item.code});else window.open(url(item),'_blank','noopener'); }catch(error){emit('error',`打开官网失败：${String(error)}`);}}
</script>

<template>
  <div class="equipment-market">
    <main class="equipment-workspace">
      <aside class="equipment-filters" aria-label="装备筛选条件">
        <div class="sidebar-heading"><h2>筛选条件</h2><el-button link type="primary" @click="reset">重置</el-button></div>
        <fieldset><legend class="type-heading"><span>装备类型</span><span class="type-multiple">可多选</span><el-button v-if="types.length" link type="primary" size="small" @click="types=[]">清空</el-button></legend><el-checkbox-group v-model="types" class="type-groups" aria-label="装备类型筛选"><div v-for="group in groupedTypes" :key="group.name" class="type-group" role="group" :aria-label="group.name"><h3>{{ group.name }}</h3><div class="type-checkboxes"><span v-if="!group.types.length" class="type-empty">暂无其他类型</span><el-checkbox v-for="name in group.types" :key="name" :value="name" :title="name"><span class="type-name">{{ name }}</span></el-checkbox></div></div></el-checkbox-group><p class="type-hint">未勾选时不限类型</p></fieldset>
        <fieldset><legend>售价范围（元）</legend><div class="price-inputs"><el-input-number v-model="price.min" :controls="false" :min="0" placeholder="最低" aria-label="价格下限" /><span>至</span><el-input-number v-model="price.max" :controls="false" :min="0" placeholder="最高" aria-label="价格上限" /></div><p v-if="priceError" class="error" role="alert">价格下限不能大于上限</p></fieldset>
        <fieldset><legend class="attribute-heading"><span>装备自身属性</span><el-button v-if="attribute" link type="primary" size="small" @click="attribute='';minimum=undefined">清空</el-button></legend><p v-if="attribute" class="attribute-selected">已选：{{ attribute }}</p><el-input-number :key="attribute" v-model="minimum" :controls="false" :min="0" :disabled="!attribute" placeholder="单条属性最低值" aria-label="属性最低值" /><div class="attribute-panel"><div class="attribute-categories" aria-label="属性分类"><button v-for="group in attributeGroups" :key="group.name" type="button" :class="{active:activeAttributeGroup?.name===group.name}" :aria-pressed="activeAttributeGroup?.name===group.name" @click="attributeGroup=group.name">{{ group.name }}</button></div><el-radio-group v-model="attribute" class="attribute-choices" aria-label="选择属性"><el-radio v-for="name in activeAttributeGroup?.attributes??[]" :key="name" :value="name">{{ name }}</el-radio><span v-if="!activeAttributeGroup" class="type-empty">暂无可选属性</span></el-radio-group></div><p class="filter-note">基本攻击力、基本精神力、基本防御力分别归为攻击力、精神力、防御力。重复词条不相加，宝石、星石及套装效果不计入自身属性。</p></fieldset>
      </aside>
      <section class="equipment-results" aria-label="装备选购列表">
        <div class="equipment-toolbar"><div><h1 :title="types.join(' / ')">{{ types.length===1?types[0]:types.length?`已选 ${types.length} 类装备`:'全部装备' }}</h1><span>{{ shown.length }} 件装备</span></div><el-input v-model="keyword" clearable :prefix-icon="Search" placeholder="搜索名称或效果，如：转化为HP" aria-label="搜索装备" /><el-select v-model="sort" aria-label="装备排序"><el-option value="default" label="默认顺序" /><el-option value="price" label="价格 · 低到高" /><el-option value="score" label="评分 · 高到低" /></el-select></div>
        <div ref="scroll" class="equipment-scroll" @scroll="onScroll">
          <article v-for="item in visibleRows" :key="item.code" class="equipment-card" :class="{selected:selected.includes(item.code)}" :data-code="item.code">
            <div class="equipment-card-heading"><div><h2 :title="`商品编号 ${item.code}`"><HighlightText :text="item.name" :keyword="keyword" /></h2><div class="equipment-meta"><span class="quality" :data-quality="item.quality"><HighlightText :text="item.quality" :keyword="keyword" /></span><span><HighlightText :text="item.type" :keyword="keyword" /></span><span><HighlightText :text="`${item.level??'—'}级`" :keyword="keyword" /></span><span>评分 <HighlightText :text="money(item.score)" :keyword="keyword" /></span></div></div><div class="equipment-actions"><strong class="equipment-price">¥ <HighlightText :text="money(item.price)" :keyword="keyword" /></strong><span class="equipment-sale-line" :title="`${saleStatus(item)==='公示中'?'公示结束':'下架时间'}：${endTime(item)??'未获取'}（北京时间）`"><span :class="{publicity:item.status===1}"><HighlightText :text="saleStatus(item)" :keyword="keyword" /></span><span><HighlightText :text="remaining(item)" :keyword="keyword" /></span></span></div></div>
            <p v-if="!item.ready||item.error||item.needsRefresh" class="equipment-pending">{{ item.error?`详情更新失败：${item.error}`:item.needsRefresh?'详情待更新，暂显示旧缓存':busy?'正在获取详情…':'尚未获取详情，请点击拉取 / 更新' }}</p><dl class="equipment-stats"><div v-for="(stat,index) in item.stats" :key="index" :class="{matched:attribute===stat[0]&&minimum!==undefined&&stat[1]>=minimum}"><dt><HighlightText :text="stat[0]" :keyword="keyword" /></dt><dd><HighlightText :text="format(stat)" :keyword="keyword" /></dd></div></dl>
            <div v-if="item.effects.length" class="equipment-effects"><span>其他属性与效果</span><p v-for="(effect,index) in item.effects" :key="index"><HighlightText :text="effect" :keyword="keyword" /></p></div>
            <details v-if="item.set" class="equipment-set" :open="!!keyword.trim()&&item.set.some(line=>line.includes(keyword.trim()))"><summary><HighlightText :text="item.set[0]" :keyword="keyword" /><span>查看套装条件与效果</span></summary><p class="set-note">以下效果有套装件数要求，不计入本件装备的属性筛选。</p><p v-for="(line,index) in item.set.slice(1)" :key="index"><HighlightText :text="line" :keyword="keyword" /></p></details>
            <div class="equipment-card-footer"><el-checkbox :model-value="selected.includes(item.code)" :disabled="selected.length===2&&!selected.includes(item.code)" :aria-label="`对比${item.name}${item.code}`" @change="toggle(item.code)">加入对比</el-checkbox><a :href="url(item)" @click.prevent="official(item)">官网详情<el-icon><Link /></el-icon></a></div>
          </article>
          <el-empty v-if="!shown.length" :description="busy?'正在获取装备，请稍候':!items.length?'当前区服暂无缓存，点击顶部「拉取 / 更新」获取装备':'没有符合条件的装备'"><el-button @click="reset">重置筛选</el-button></el-empty>
        </div>
        <div v-if="selected.length" class="equipment-compare-bar"><strong>装备对比 {{ selected.length }}/2</strong><el-tag v-for="item in pair" :key="item.code" closable @close="toggle(item.code)"><HighlightText :text="item.name" :keyword="keyword" /> · ¥<HighlightText :text="money(item.price)" :keyword="keyword" /></el-tag><el-button link @click="selected=[]">清空</el-button><el-button type="primary" :disabled="selected.length!==2" @click="comparing=true">开始对比</el-button></div>
        <footer class="equipment-status"><span>官方类型原样显示 · 重复词条完整保留</span><span role="status">已显示 {{ visibleRows.length }} / {{ shown.length }} 件装备</span><el-button v-if="hasMore" size="small" :loading="loadingRows" @click="loadMore">加载更多</el-button></footer>
      </section>
    </main>
    <el-dialog v-model="comparing" fullscreen destroy-on-close class="equipment-compare-dialog" title="装备属性对比">
      <template #header><div class="compare-heading"><el-button :icon="ArrowLeft" @click="comparing=false">返回列表</el-button><h2>装备属性对比</h2><span>相同属性对齐 · 缺失项显示 — · 不合并重复词条</span></div></template>
      <div class="comparison-panes"><section v-for="item in pair" :key="item.code" class="comparison-pane"><div class="equipment-card-heading"><div><h2><HighlightText :text="item.name" :keyword="keyword" /></h2><p class="equipment-meta"><span class="quality" :data-quality="item.quality"><HighlightText :text="item.quality" :keyword="keyword" /></span><span><HighlightText :text="`${item.type} · ${item.level??'—'}级 · 评分 ${money(item.score)} · ${saleStatus(item)} · ${remaining(item)}`" :keyword="keyword" /></span></p></div><strong class="equipment-price">¥ <HighlightText :text="money(item.price)" :keyword="keyword" /></strong></div><dl class="comparison-stats"><div v-for="field in compareFields" :key="`${field.name}-${field.index}`"><dt><HighlightText :text="field.name" :keyword="keyword" /><small v-if="field.index">（第 {{ field.index+1 }} 条）</small></dt><dd><HighlightText :text="format(item.stats.filter(s=>s[0]===field.name)[field.index])" :keyword="keyword" /></dd></div></dl><div class="comparison-effects"><h3>其他属性与效果</h3><p v-for="(effect,index) in item.effects" :key="index"><HighlightText :text="effect" :keyword="keyword" /></p><p v-if="!item.effects.length" class="muted">官网详情未显示其他属性与效果</p><template v-if="item.set"><h3>套装条件与效果</h3><p v-for="(line,index) in item.set" :key="index"><HighlightText :text="line" :keyword="keyword" /></p></template></div><a :href="url(item)" @click.prevent="official(item)">打开官网核对</a></section></div>
    </el-dialog>
  </div>
</template>

<style scoped>
.equipment-market{height:100%;min-height:0;display:flex;flex-direction:column;background:#f3f5f7;color:#273933;font-size:14px}
.equipment-workspace{display:flex;flex:1;min-height:0}.equipment-filters{width:280px;flex:none;overflow-y:auto;scrollbar-gutter:stable;padding:20px 16px;background:#fff;border-right:1px solid #dce3e1}.sidebar-heading{display:flex;justify-content:space-between;align-items:center;margin-bottom:20px}.sidebar-heading h2{font-size:15px;margin:0}
fieldset{border:0;padding:0;margin:0 0 28px;min-width:0}legend{font-size:13px;font-weight:600;margin-bottom:12px}.type-heading{display:flex;align-items:center;gap:8px;width:100%;margin-bottom:12px}.type-heading .el-button{margin-left:auto}.type-multiple{font-size:12px;font-weight:400;color:#69776f}.type-groups{display:flex;flex-direction:column}.type-group{display:grid;grid-template-columns:48px minmax(0,1fr);gap:12px;padding:10px 0;border-bottom:1px solid #edf0ef}.type-group:first-child{padding-top:0}.type-group:last-child{border-bottom:0;padding-bottom:0}.type-group h3{font-size:12px;font-weight:500;line-height:24px;color:#596b63;margin:0;overflow-wrap:anywhere}.type-checkboxes{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:4px 6px;align-items:start}.type-checkboxes .el-checkbox{margin:0;height:24px;min-width:0;padding:0 2px;border-radius:4px}.type-checkboxes .el-checkbox:hover{background:#f2f6f4}.type-checkboxes .el-checkbox.is-checked{background:#eaf4ef}.type-checkboxes :deep(.el-checkbox__label){display:inline-flex;min-width:0;max-width:calc(100% - 14px);padding-left:5px;font-size:13px}.type-name{min-width:0;overflow:hidden;text-overflow:ellipsis}.type-empty{grid-column:1/-1;font-size:12px;line-height:24px;color:#69776f}.type-hint{color:#69776f;font-size:12px;margin:12px 0 0}.equipment-filters fieldset+fieldset{padding-top:16px;border:0}.equipment-filters fieldset{margin-bottom:20px}.price-inputs{display:flex;align-items:center;gap:8px}.price-inputs .el-input-number{flex:1;width:0;min-width:0}.attribute-heading{display:flex;align-items:center;justify-content:space-between;width:100%}.attribute-panel{display:grid;grid-template-columns:84px minmax(0,1fr);border:1px solid #dce3e1;border-radius:6px;overflow:hidden;margin-top:10px;height:204px}.attribute-categories{min-height:0;overflow-y:auto;background:#f5f7f6;padding:4px 0;border-right:1px solid #dce3e1}.attribute-categories button{display:block;width:100%;border:0;background:transparent;text-align:left;padding:8px;font:inherit;font-size:12px;color:#4e5e58;cursor:pointer}.attribute-categories button:hover{background:#eaf1ed}.attribute-categories button.active{background:#e2eee7;color:#195b50;font-weight:600}.attribute-choices{display:flex;flex-direction:column;flex-wrap:nowrap;align-items:stretch;justify-content:flex-start;gap:0;padding:4px 8px;min-height:0;overflow-y:auto;scrollbar-gutter:stable}.attribute-choices .el-radio{margin:0;min-height:28px;height:auto;white-space:normal;align-items:center}.attribute-choices :deep(.el-radio__label){font-size:12px;padding-left:5px;white-space:normal;overflow-wrap:anywhere;line-height:1.6}.attribute-selected{margin:0 0 8px;font-size:12px;color:#247466}.equipment-filters fieldset>.el-input-number{width:100%}.filter-note{color:#69776f;font-size:12px;line-height:1.8;margin:10px 0 0}.error{font-size:12px;color:#b34a40}
.equipment-results{flex:1;min-width:0;min-height:0;display:flex;flex-direction:column}.equipment-toolbar{display:flex;gap:16px;padding:10px 20px;align-items:center;background:#fff;border-bottom:1px solid #e1e7e3;flex:none}.equipment-toolbar>div:first-child{display:flex;align-items:baseline;gap:14px;flex:1;min-width:180px}.equipment-toolbar h1{font-size:19px;margin:0;white-space:nowrap;min-width:0;overflow:hidden;text-overflow:ellipsis}.equipment-toolbar>div:first-child>span{color:#69776f;font-size:12px;white-space:nowrap}.equipment-toolbar>.el-input{width:300px}.equipment-toolbar>.el-select{width:168px}
.equipment-scroll>.el-empty{grid-column:1/-1}.equipment-scroll{flex:1;min-height:0;overflow-y:auto;scrollbar-gutter:stable;padding:14px;display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:12px;align-content:start;align-items:stretch}.equipment-pending{font-size:12px;line-height:1.7;color:#97651e;overflow-wrap:anywhere}.equipment-card{background:#fff;border:1px solid #dce3e1;border-radius:12px;padding:12px 14px;scroll-margin:12px;min-width:0;display:flex;flex-direction:column}.equipment-card.selected{border-color:#247466}.equipment-card-heading{display:flex;justify-content:space-between;gap:12px;align-items:flex-start}.equipment-card h2,.comparison-pane h2{font-size:15px;line-height:1.4;margin:0 0 5px;overflow-wrap:anywhere}.equipment-meta{display:flex;flex-wrap:wrap;gap:3px 9px;font-size:12px;color:#69776f;line-height:1.6;margin:0}.quality{display:inline-flex;align-items:center;flex:none;min-height:20px;padding:0 5px;border:1px solid #dce3e1;border-radius:4px;background:#f5f7f6;color:#4e5e58;font-size:12px;font-weight:650;line-height:18px}.quality[data-quality="精致"]{color:#0000f8;background:#f0f3ff;border-color:#b7c3ef}.quality[data-quality="传说"]{color:#b000b0;background:#fff2ff;border-color:#e2b5e2}.quality[data-quality="逆天"]{color:#d81448;background:#fff1f4;border-color:#edb3c3}.quality[data-quality="神工"]{color:#737300;background:#fafaf0;border-color:#d8d898}.quality[data-quality="白色"]{color:#59635f;background:#f5f6f5;border-color:#d5dad7}.equipment-actions{display:flex;flex-direction:column;align-items:flex-end;gap:5px;flex:none}.equipment-sale-line{display:flex;gap:7px;align-items:baseline;color:#69776f;font-size:12px;white-space:nowrap;font-variant-numeric:tabular-nums}.equipment-sale-line>span:first-child{color:#247466}.equipment-sale-line>span.publicity{color:#97651e}.equipment-price{font-size:17px;font-weight:600;color:#b34a40;font-variant-numeric:tabular-nums;white-space:nowrap}.equipment-actions .el-checkbox{height:26px;margin:0}
.equipment-stats{display:grid;grid-template-columns:1fr 1fr;gap:0 20px;margin:9px 0 0}.equipment-stats>div{display:flex;justify-content:space-between;align-items:baseline;gap:8px;padding:3px 0;min-width:0}.equipment-stats dt{color:#4e5e58;overflow-wrap:anywhere}.equipment-stats dd{margin:0;font-weight:600;font-variant-numeric:tabular-nums;white-space:nowrap}.matched{background:#eff6f3;color:#195b50;border-radius:4px}.equipment-effects{margin-top:8px;padding-top:8px;border-top:1px solid #edf0ef;display:block}.equipment-effects>span{display:block;margin-bottom:3px;font-size:12px;color:#69776f;flex:none}.equipment-effects p{margin:0;line-height:1.6;overflow-wrap:anywhere}
.equipment-set{margin-top:9px;border-top:1px solid #edf0ef;padding-top:8px}.equipment-set summary{cursor:pointer;color:#42685c;line-height:1.8;font-size:13px}.equipment-set summary span{display:block;margin-left:0;color:#69776f;font-size:12px}.equipment-set p{margin:8px 0 0;line-height:1.8;font-size:13px;overflow-wrap:anywhere}.equipment-set .set-note{color:#69776f;font-size:12px}.equipment-card-footer{display:flex;justify-content:space-between;align-items:center;gap:10px;margin-top:auto;padding-top:10px;font-size:12px;color:#69776f}.equipment-card-footer .el-checkbox{margin:0;height:24px;flex:none}.equipment-card-footer a{flex:none;display:inline-flex;align-items:center;gap:5px}.equipment-market a{color:#247466;text-underline-offset:3px}.equipment-card-footer a{text-decoration:none}
.equipment-status{display:flex;justify-content:space-between;gap:16px;padding:10px 20px;background:#fff;border-top:1px solid #dce3e1;color:#69776f;font-size:12px;flex:none}.equipment-compare-bar{display:flex;align-items:center;gap:12px;padding:12px 24px;background:#eff6f3;border-top:1px solid #dce9e4;flex:none}.equipment-compare-bar strong{font-size:13px}.equipment-compare-bar>.el-button:first-of-type{margin-left:auto}
.compare-heading{display:flex;align-items:center;gap:20px}.compare-heading h2{font-size:18px;margin:0}.compare-heading>span{font-size:12px;color:#69776f}.comparison-panes{display:grid;grid-template-columns:1fr 1fr;gap:32px;max-width:1320px;margin:0 auto}.comparison-pane{min-width:0;padding:20px;background:#fff;border:1px solid #dce3e1;border-radius:12px}.comparison-stats{margin:24px 0}.comparison-stats>div{display:flex;justify-content:space-between;gap:16px;padding:12px 0;border-bottom:1px solid #edf0ef}.comparison-stats dd{font-weight:600;font-variant-numeric:tabular-nums;margin:0}.comparison-stats small{font-size:12px;color:#69776f}.comparison-effects{min-height:130px}.comparison-effects h3{font-size:13px;margin:24px 0 10px}.comparison-effects p{line-height:1.8;font-size:13px;overflow-wrap:anywhere}.muted{color:#69776f}button:focus-visible,a:focus-visible,summary:focus-visible{outline:2px solid #247466;outline-offset:3px}
@media(max-width:1350px){.equipment-scroll{grid-template-columns:repeat(2,minmax(0,1fr))}}
@media(max-width:1150px){.equipment-toolbar{gap:12px;padding:10px 16px}.equipment-toolbar>div:first-child{min-width:140px;gap:9px}.equipment-toolbar>.el-input{width:240px}.equipment-toolbar>.el-select{width:148px}.equipment-scroll{padding:12px}.equipment-card{padding:12px 14px}.equipment-actions{gap:5px}.equipment-stats{column-gap:18px}.equipment-meta{gap:5px 10px}}
@media(max-width:760px){.equipment-scroll{grid-template-columns:minmax(0,1fr)}.equipment-filters{width:190px;padding:18px 14px}.price-inputs{flex-wrap:wrap}.price-inputs .el-input-number{width:66px}.equipment-toolbar{flex-wrap:wrap}.equipment-toolbar>div:first-child{flex-basis:100%}.equipment-toolbar>.el-input{flex:1;width:auto;min-width:120px}.equipment-toolbar>.el-select{width:135px}.equipment-card-heading{gap:12px}.equipment-actions{flex-direction:column;gap:4px;align-items:flex-end}.equipment-stats{grid-template-columns:1fr}.equipment-effects{display:block}.equipment-compare-bar{flex-wrap:wrap}.comparison-panes{gap:12px}.comparison-pane{padding:14px}.compare-heading>span{display:none}.equipment-status>span:first-child{display:none}}
</style>
