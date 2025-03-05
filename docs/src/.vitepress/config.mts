import { defineConfig } from "vitepress";

export default defineConfig({
  title: "LogCraft CLI",
  description: "Detection as Code made simple",
  themeConfig: {
    logo: "/logo.png",
    nav: [
      { text: "Home", link: "/" },
      { text: "Quick Start", link: "/essentials/quickstart" },
      { text: "Support", link: "/support" },
    ],

    sidebar: [
      {
        text: "Essentials",
        items: [
          { text: "Installation", link: "/essentials/installation" },
          { text: "Quick Start", link: "/essentials/quickstart" },
          { text: "Configuration", link: "/essentials/configuration.md" },
          { text: "GitLab Integration", link: "/essentials/gitlab.md" },
        ],
      },
      {
        text: "Concepts",
        items: [
          { text: "Detections", link: "/concepts/detections" },
          { text: "Identifiers", link: "/concepts/identifiers" },
          { text: "Plugins", link: "/concepts/plugins" },
          { text: "Policies", link: "/concepts/policies" },
        ],
      },
      {
        text: "Commands",
        collapsed: false,
        items: [
          { text: "lgc init", link: "/commands/init" },
          { text: "lgc apply", link: "/commands/apply" },
          { text: "lgc destroy", link: "/commands/destroy" },
          { text: "lgc ping", link: "/commands/ping" },
          { text: "lgc plan", link: "/commands/plan" },
          { text: "lgc services", link: "/commands/services" },
          { text: "lgc validate", link: "/commands/validate" },
        ],
      },
      {
        text: "Plugins",
        collapsed: false,
        items: [
          { text: "CrowdStrike", link: "/plugins/crowdstrike" },
          { text: "Elastic", link: "/plugins/elastic" },
          {
            text: "Google Chronicle (SecOps)",
            link: "/plugins/google-chronicle",
          },
          { text: "LimaCharlie", link: "/plugins/limacharlie" },
          { text: "Microsoft Sentinel", link: "/plugins/microsoft-sentinel" },
          { text: "Palo Alto Cortex", link: "/plugins/paloalto-cortex" },
          { text: "Sigma", link: "/plugins/sigma" },
          { text: "Splunk", link: "/plugins/splunk" },
          { text: "Sekoia", link: "/plugins/sekoia" },
          { text: "Tanium", link: "/plugins/tanium" },
          { text: "Yara", link: "/plugins/yara" },
        ],
      },
      {
        text: "Developers",
        collapsed: false,
        items: [
          {
            text: "Docker images",
            link: "developers/docker-images.md",
          },
          {
            text: "State",
            link: "developers/state.md",
          },
          {
            text: "Compiling",
            link: "developers/compiling.md",
          },
          {
            text: "Custom plugins",
            link: "/developers/how-to-create-plugins",
          },
        ],
      },
      { text: "Getting Help", link: "/support" },
    ],

    socialLinks: [
      { icon: "twitter", link: "https://twitter.com/LogCraftIO" },
      { icon: "github", link: "https://github.com/LogCraftIO/logcraft-cli" },
      { icon: "linkedin", link: "https://www.linkedin.com/company/logcraft" },
      {
        icon: "slack",
        link: "https://join.slack.com/t/logcraft/shared_invite/zt-2jdw7ntts-yVhw8rIji5ZFpPt_d6HM9w",
      },
    ],
    footer: {
      copyright: "Copyright © 2023-present LogCraft, SAS",
    },
  },
  vite: {
    ssr: {
      noExternal: ["vuetify"],
    },
  },
  sitemap: {
    hostname: "https://docs.logcraft.io",
  },
  head: [
    ["link", { rel: "icon", type: "image/png", href: "/logo.png" }],
    ["meta", { property: "og:type", content: "website" }],
    ["meta", { property: "og:locale", content: "en" }],
    [
      "meta",
      {
        property: "og:title",
        content: "LogCraft | Detection-as-Code for Modern Security Operations",
      },
    ],
    ["meta", { property: "og:site_name", content: "LogCraft" }],
    ["meta", { property: "og:url", content: "https://docs.logcraft.io/" }],
  ],
});
