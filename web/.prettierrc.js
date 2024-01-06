module.exports = {
  trailingComma: 'es5',
  printWidth: 80,
  tabWidth: 2,
  useTabs: false,
  semi: true,
  singleQuote: true,
  jsxSingleQuote: true,
  plugins: [require.resolve('@trivago/prettier-plugin-sort-imports')],
  importOrder: ['^(?!([./]|client//).*)$', '^[./]', '^(client)/(.*)$'],
  importOrderSeparation: true,
  importOrderSortSpecifiers: true,
};
