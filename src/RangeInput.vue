<script setup lang="ts">
const props = defineProps<{
  modelValue: [number | null, number | null]
  label: string
  min?: number
}>()
const emit = defineEmits<{
  'update:modelValue': [value: [number | null, number | null]]
}>()
function update(bound: 0 | 1, value: number | undefined) {
  const next: [number | null, number | null] = [...props.modelValue]
  next[bound] = value ?? null
  emit('update:modelValue', next)
}
</script>

<template>
  <div class="range-input">
    <el-input-number
      :model-value="modelValue[0] ?? undefined"
      :controls="false"
      :min="min"
      placeholder="最低"
      :aria-label="`${label}下限`"
      @update:model-value="update(0, $event)"
    />
    <span class="range-separator">至</span>
    <el-input-number
      :model-value="modelValue[1] ?? undefined"
      :controls="false"
      :min="min"
      placeholder="最高"
      :aria-label="`${label}上限`"
      @update:model-value="update(1, $event)"
    />
  </div>
</template>
