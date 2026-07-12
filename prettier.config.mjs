/** @type {import("prettier").Config} */
export default {
  endOfLine: "lf",
  plugins: ["prettier-plugin-tailwindcss"],
  printWidth: 80,
  semi: true,
  singleQuote: false,
  tabWidth: 2,
  tailwindStylesheet: "./src/index.css",
  trailingComma: "all",
  useTabs: false,
};
