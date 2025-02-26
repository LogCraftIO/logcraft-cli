<template>
  <div class="d-flex my-2">
    <div class="d-flex align-center">
      <div class="font-weight-bold pr-4">Category</div>
      <div>
        <v-chip :color="getCategoryColor(category)">{{ category }}</v-chip>
      </div>
    </div>
    <div class="d-flex align-center">
      <div class="font-weight-bold pr-4 pl-10">Stage</div>
      <div class="text-capitalize">
        {{ stage }}
      </div>
    </div>
  </div>
</template>

<script setup>
import { getCategoryColor } from "./common.js";

const props = defineProps({
  category: String,
  stage: String,
});
</script>

<style scoped>
.colwidth {
  width: 100px;
  min-width: 100px;
  max-width: 100px;
}
</style>
