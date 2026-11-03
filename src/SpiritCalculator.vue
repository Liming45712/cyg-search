<script setup lang="ts">
import { computed, reactive } from 'vue';
import { calculateSpirit, defaultSpirit, qualityRates, spiritDirections, spiritLevels, spiritStats, treasureStats } from './spirit';

const input = reactive(defaultSpirit());
const result = computed(() => calculateSpirit(input));
const fmt = (value: number) => new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 6 }).format(value);
const directions = [
  { key: 'allocation', label: '等级点数分配' },
  { key: 'insightStat', label: '领悟加点方向' },
  { key: 'fixedStat', label: '慧心固定方向' },
  { key: 'huixinStat', label: '慧心分配方向' },
] as const;
const treasureGroups = [
  { key: 'treasureBase', label: '基础属性' },
  { key: 'treasureExtra', label: '附加属性' },
] as const;
</script>

<template>
  <section class="spirit-calculator" aria-label="星灵与灵宝计算器">
    <div class="calculator-heading">
      <div>
        <h2>星灵 / 灵宝计算器</h2>
        <p>按原表蓝色格子手动填写，修改后自动计算。初始值为 Excel 示例。</p>
      </div>
      <el-button @click="Object.assign(input, defaultSpirit())">恢复表中示例</el-button>
    </div>
    <div class="calculator-layout">
      <el-form class="calculator-inputs" label-position="top" @submit.prevent>
        <section class="calculator-section">
          <h3>星灵设置</h3>
          <div class="calculator-fields">
            <el-form-item label="等级">
              <el-select v-model="input.level" aria-label="星灵等级">
                <el-option v-for="level in spiritLevels" :key="level" :value="level" :label="level ? `${level}级` : '不带星灵'" />
              </el-select>
            </el-form-item>
            <el-form-item label="资质品质">
              <el-select v-model="input.quality" aria-label="资质品质">
                <el-option v-for="(rate, name) in qualityRates" :key="name" :value="name" :label="`${name}（${fmt(rate * 100)}%）`" />
              </el-select>
            </el-form-item>
            <el-form-item label="成长品质">
              <el-select v-model="input.growth" aria-label="成长品质">
                <el-option v-for="(rate, name) in qualityRates" :key="name" :value="name" :label="`${name}（${fmt(rate * 100)}%）`" />
              </el-select>
            </el-form-item>
            <el-form-item label="强化星级">
              <el-select v-model="input.stars" aria-label="强化星级">
                <el-option v-for="star in 11" :key="star" :value="star - 1" :label="`${star - 1}星`" />
              </el-select>
            </el-form-item>
            <el-form-item label="慧心">
              <el-input-number v-model="input.huixin" :min="0" :max="139" :precision="0" :controls="false" aria-label="慧心" />
            </el-form-item>
            <el-form-item label="领悟点数">
              <el-input-number v-model="input.insight" :min="0" :max="139" :precision="0" :controls="false" aria-label="领悟点数" />
            </el-form-item>
          </div>
          <div class="calculator-switches">
            <el-checkbox v-model="input.divineTreasure">神逆灵宝（启用慧心固定加点）</el-checkbox>
            <el-checkbox v-model="input.starGeneral">已转星将（角色加成 × 1.1）</el-checkbox>
          </div>
        </section>

        <section class="calculator-section">
          <h3>资质与加点</h3>
          <p>五项资质填写最终数值，包含灵宝对资质的影响。</p>
          <div class="aptitude-fields">
            <el-form-item v-for="stat in spiritStats" :key="stat" :label="`${stat}资质`">
              <el-input-number v-model="input.aptitude[stat]" :min="0" :step="0.01" :controls="false" :aria-label="`${stat}资质`" />
            </el-form-item>
          </div>
          <div class="calculator-fields direction-fields">
            <el-form-item v-for="field in directions" :key="field.key" :label="field.label">
              <el-select v-model="input[field.key]" :aria-label="field.label">
                <el-option v-for="stat in spiritDirections" :key="stat" :value="stat" :label="stat" />
              </el-select>
            </el-form-item>
          </div>
        </section>

        <section class="calculator-section">
          <h3>灵宝属性</h3>
          <p>填写灵宝面板数值；留空按 0 计算。</p>
          <div class="treasure-table">
            <table>
              <caption class="sr-only">灵宝基础属性和附加属性</caption>
              <thead><tr><th scope="col">属性</th><th scope="col">基础属性</th><th scope="col">附加属性</th></tr></thead>
              <tbody>
                <tr v-for="stat in treasureStats" :key="stat">
                  <th scope="row">{{ stat }}</th>
                  <td v-for="group in treasureGroups" :key="group.key">
                    <el-input-number v-model="input[group.key][stat]" :min="0" :controls="false" :aria-label="`灵宝${group.label}${stat}`" placeholder="0" />
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <h4>附加五维</h4>
          <div class="aptitude-fields">
            <el-form-item v-for="stat in spiritStats" :key="stat" :label="stat">
              <el-input-number v-model="input.treasureFive[stat]" :min="0" :controls="false" :aria-label="`灵宝附加${stat}`" placeholder="0" />
            </el-form-item>
          </div>
        </section>
      </el-form>

      <aside class="calculator-output" aria-label="计算结果">
        <h3>计算结果</h3>
        <el-alert v-if="result.error !== null" :title="result.error" type="warning" :closable="false" show-icon />
        <template v-else>
          <p class="calculation-ratio">加成比例 <strong>{{ fmt(result.ratio * 100) }}%</strong> × 转星将系数 <strong>{{ result.factor }}</strong></p>
          <el-table :data="result.rows" class="spirit-results">
            <el-table-column prop="name" label="属性" min-width="65" />
            <el-table-column label="星灵面板" min-width="125" align="right">
              <template #default="{ row }">{{ fmt(row.panel) }}</template>
            </el-table-column>
            <el-table-column label="角色获得加成" min-width="135" align="right">
              <template #default="{ row }"><strong>{{ fmt(row.bonus) }}</strong></template>
            </el-table-column>
          </el-table>
          <h4>五维合计</h4>
          <dl class="spirit-totals">
            <div v-for="stat in spiritStats" :key="stat"><dt>{{ stat }}</dt><dd>{{ fmt(result.total[stat]) }}</dd></div>
          </dl>
          <h4>自动生成点数</h4>
          <dl class="point-summary">
            <div><dt>每项等级基础点</dt><dd>{{ fmt(result.base) }}</dd></div>
            <div><dt>等级可分配点数</dt><dd>{{ fmt(result.points) }}</dd></div>
            <div><dt>慧心固定点数</dt><dd>{{ fmt(result.fixed) }}</dd></div>
            <div><dt>慧心可分配点数</dt><dd>{{ fmt(result.assigned) }}</dd></div>
          </dl>
        </template>
        <p class="calculator-note">按拙耳 Excel 原公式计算，显示最多 6 位小数，中间计算不取整。选择“不带星灵”只清零等级点数；如需全部归零，也请清零慧心、领悟和灵宝属性。</p>
      </aside>
    </div>
  </section>
</template>

<style scoped>
.spirit-calculator { height: 100%; overflow: auto; padding: 24px 28px 40px; }
.calculator-heading { display: flex; align-items: center; justify-content: space-between; gap: 24px; margin-bottom: 24px; }
.calculator-heading h2 { font-size: 20px; }
.calculator-heading p, .calculator-section > p { color: var(--el-text-color-secondary); line-height: 1.7; margin-top: 6px; }
.calculator-layout { display: grid; grid-template-columns: minmax(0, 1fr) minmax(380px, 0.7fr); gap: 28px; align-items: start; max-width: 1500px; margin: 0 auto; }
.calculator-heading { max-width: 1500px; margin-left: auto; margin-right: auto; }
.calculator-section { background: white; padding: 20px 24px; margin-bottom: 16px; }
h3 { font-size: 16px; margin: 0 0 16px; }
h4 { font-size: 14px; margin: 24px 0 12px; }
.calculator-section > p { margin: -8px 0 16px; font-size: 13px; }
.calculator-fields { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 0 16px; }
.aptitude-fields { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 12px; }
.direction-fields { grid-template-columns: repeat(2, minmax(0, 1fr)); }
.calculator-inputs :deep(.el-input-number) { width: 100%; }
.calculator-inputs :deep(.el-input__wrapper), .calculator-inputs :deep(.el-select__wrapper) { background: #edf5fd; }
.calculator-switches { display: flex; flex-wrap: wrap; gap: 0 20px; }
.calculator-switches :deep(.el-checkbox) { margin-right: 0; }
.treasure-table table { width: 100%; border-collapse: collapse; text-align: left; }
.treasure-table th { font-weight: 500; font-size: 13px; }
.treasure-table td, .treasure-table th { padding: 6px 8px; }
.treasure-table thead { background: var(--el-fill-color-light); }
.treasure-table tbody th { width: 20%; }
.calculator-output { background: white; padding: 24px; position: sticky; top: 0; }
.calculation-ratio { color: var(--el-text-color-regular); line-height: 1.8; font-size: 13px; margin-bottom: 16px; }
.calculation-ratio strong, .spirit-results strong { color: var(--el-color-primary); }
.spirit-results, dd { font-variant-numeric: tabular-nums; }
.spirit-totals { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 8px; margin: 0; }
dt { color: var(--el-text-color-secondary); font-size: 13px; }
dd { margin: 6px 0 0; }
.point-summary { margin: 0; }
.point-summary > div { display: flex; justify-content: space-between; align-items: baseline; padding: 5px 0; }
.calculator-note { font-size: 12px; color: var(--el-text-color-secondary); line-height: 1.8; margin-top: 24px; }
.sr-only { position: absolute; width: 1px; height: 1px; padding: 0; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
@media (max-width: 1200px) {
  .spirit-calculator { padding: 20px; }
  .calculator-layout { gap: 16px; grid-template-columns: minmax(0, 1fr) 370px; }
  .calculator-section, .calculator-output { padding: 18px; }
  .calculator-fields { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .aptitude-fields { grid-template-columns: repeat(3, minmax(0, 1fr)); }
}
</style>
