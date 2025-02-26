<template>
  <div>
    <ul v-for="(p, index) in plugins" :key="index" class="my-2">
      <li>
        <a class="mr-4" :href="withBase(p.url)"> {{ p.frontmatter.title }}</a>
        <v-chip
          :color="getCategoryColor(p.frontmatter.category)"
          size="small"
          >{{ p.frontmatter.category }}</v-chip
        >
      </li>
    </ul>
  </div>
</template>

<script setup>
import { data as plugins } from "./plugins.data.js";
import { withBase } from "vitepress";
import { getCategoryColor } from "./common.js";
</script>
