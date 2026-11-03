<script setup lang="ts">
import { computed } from 'vue';
import { highlightParts } from './highlight';
const props = defineProps<{ text: string; keyword: string }>();
const parts = computed(()=>highlightParts(props.text,props.keyword));
</script>

<template><template v-for="(part,index) in parts" :key="index"><mark v-if="part.match" class="search-hit">{{ part.text }}</mark><template v-else>{{ part.text }}</template></template></template>

<style scoped>
.search-hit{background:#ffed9e;color:#543b00;border-radius:2px;box-decoration-break:clone;-webkit-box-decoration-break:clone}
</style>
