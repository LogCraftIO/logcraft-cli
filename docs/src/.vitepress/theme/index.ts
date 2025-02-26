import type { Theme } from "vitepress";
import DefaultTheme from "vitepress/theme";

// Custom components
import PluginsIndexPage from "./components/plugins/PluginsIndexPage.vue";
import PluginStatus from "./components/plugins/PluginStatus.vue";

// Vuetify
import "vuetify/styles";
import { createVuetify } from "vuetify";
import { VChip } from "vuetify/components";

const vuetify = createVuetify({
  components: {
    VChip,
  },
});

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.use(vuetify);
    app.component("PluginsIndexPage", PluginsIndexPage);
    app.component("PluginStatus", PluginStatus);
  },
} satisfies Theme;
