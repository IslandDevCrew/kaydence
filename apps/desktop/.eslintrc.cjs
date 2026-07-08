/* Presentation-only frontend lint (root AGENTS §9: no business logic here). */
module.exports = {
  root: true,
  env: { browser: true, es2022: true },
  parser: "@typescript-eslint/parser",
  parserOptions: { ecmaVersion: 2022, sourceType: "module" },
  plugins: ["@typescript-eslint"],
  extends: ["eslint:recommended", "plugin:@typescript-eslint/recommended"],
  ignorePatterns: ["dist", "src-tauri", "node_modules"],
  rules: {
    "@typescript-eslint/no-explicit-any": "error"
  }
};
