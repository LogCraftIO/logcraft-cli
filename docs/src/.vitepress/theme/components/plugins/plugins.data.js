import { createContentLoader } from "vitepress";

const pages = createContentLoader("plugins/*.md", {
  includeSrc: false,
  render: false,
  excerpt: false,
  transform(rawData) {
    return rawData
      .filter((item) => {
        // do not return the index page
        return item.url !== "/plugins/";
      })
      .sort((a, b) => {
        // sort by title alphabetically
        return a.frontmatter.title.localeCompare(b.frontmatter.title);
      });
  },
});

export default pages;
