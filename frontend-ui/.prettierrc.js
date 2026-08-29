/**
 * MOX 前端 - Prettier 配置
 * 统一代码格式化风格
 */
module.exports = {
  // 行宽
  printWidth: 100,
  // 缩进空格数
  tabWidth: 2,
  // 使用空格缩进
  useTabs: false,
  // 语句末尾不加分号
  semi: false,
  // 使用单引号
  singleQuote: true,
  // JSX 中使用双引号
  jsxSingleQuote: false,
  // 尾随逗号（多行时）
  trailingComma: 'all',
  // 大括号内空格
  bracketSpacing: true,
  // 箭头函数单参数括号
  arrowParens: 'always',
  // 行尾
  endOfLine: 'lf',
  // Vue 文件中 <script> 和 <style> 缩进
  vueIndentScriptAndStyle: false,
  // HTML 空白敏感
  htmlWhitespaceSensitivity: 'css',
  // 换行符
  proseWrap: 'preserve',

  // 针对不同文件的覆盖配置
  overrides: [
    {
      files: '*.json',
      options: {
        printWidth: 80,
        trailingComma: 'none',
      },
    },
    {
      files: '*.md',
      options: {
        printWidth: 80,
        proseWrap: 'always',
      },
    },
    {
      files: '*.vue',
      options: {
        printWidth: 100,
        htmlWhitespaceSensitivity: 'ignore',
      },
    },
  ],
}
