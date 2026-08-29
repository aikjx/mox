/**
 * MOX 前端 - ESLint 配置
 * 统一代码风格和质量门禁
 *
 * 执行命令：
 *   npm run lint          # 检查代码
 *   npm run lint:fix      # 自动修复
 */
module.exports = {
  root: true,
  env: {
    browser: true,
    node: true,
    es2022: true,
  },
  extends: [
    'eslint:recommended',
    'plugin:vue/vue3-recommended',
    'plugin:vue/essential',
  ],
  parserOptions: {
    ecmaVersion: 'latest',
    sourceType: 'module',
  },
  rules: {
    // —— 可能的错误 ——
    'no-console': process.env.NODE_ENV === 'production' ? 'warn' : 'off',
    'no-debugger': process.env.NODE_ENV === 'production' ? 'warn' : 'off',
    'no-unused-vars': ['warn', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
    'no-undef': 'error',

    // —— 最佳实践 ——
    'eqeqeq': ['error', 'always', { null: 'ignore' }],
    'no-eval': 'error',
    'no-implied-eval': 'error',
    'no-return-assign': 'error',
    'no-self-compare': 'error',
    'no-throw-literal': 'error',
    'no-unmodified-loop-condition': 'error',
    'no-useless-call': 'error',
    'no-useless-concat': 'error',
    'no-useless-escape': 'warn',
    'prefer-promise-reject-errors': 'error',

    // —— 变量 ——
    'no-var': 'error',
    'prefer-const': 'warn',
    'prefer-spread': 'warn',

    // —— 风格 ——
    'indent': ['warn', 2, { SwitchCase: 1 }],
    'quotes': ['warn', 'single', { avoidEscape: true }],
    'semi': ['warn', 'never'],
    'comma-dangle': ['warn', 'always-multiline'],
    'no-trailing-spaces': 'warn',
    'no-multiple-empty-lines': ['warn', { max: 2 }],
    'eol-last': 'warn',
    'space-before-function-paren': ['warn', 'always'],
    'space-in-parens': ['warn', 'never'],
    'object-curly-spacing': ['warn', 'always'],
    'array-bracket-spacing': ['warn', 'never'],
    'key-spacing': ['warn', { beforeColon: false, afterColon: true }],
    'comma-spacing': ['warn', { before: false, after: true }],

    // —— ES6+ ——
    'arrow-spacing': ['warn', { before: true, after: true }],
    'no-confusing-arrow': 'warn',
    'no-useless-computed-key': 'warn',
    'no-useless-constructor': 'warn',
    'no-useless-rename': 'warn',
    'prefer-arrow-callback': 'warn',
    'prefer-template': 'warn',
    'template-curly-spacing': 'warn',

    // —— Vue 3 规则 ——
    'vue/multi-word-component-names': 'off',
    'vue/no-v-html': 'off',
    'vue/script-setup-uses-vars': 'error',
    'vue/no-mutating-props': 'warn',
    'vue/attribute-hyphenation': ['warn', 'always'],
    'vue/html-self-closing': ['warn', {
      html: { void: 'always', normal: 'always', component: 'always' },
      svg: 'always',
      math: 'always',
    }],
    'vue/max-attributes-per-line': ['warn', {
      singleline: { max: 3 },
      multiline: { max: 1 },
    }],
    'vue/singleline-html-element-content-newline': 'off',
    'vue/multiline-html-element-content-newline': 'warn',
    'vue/html-closing-bracket-spacing': 'warn',
    'vue/html-closing-bracket-newline': ['warn', {
      singleline: 'never',
      multiline: 'always',
    }],
    'vue/prop-name-casing': ['warn', 'camelCase'],
    'vue/component-name-in-template-casing': ['warn', 'PascalCase'],
    'vue/custom-event-name-casing': ['warn', 'kebab-case'],
    'vue/v-on-event-hyphenation': ['warn', 'always'],
    'vue/v-bind-style': ['warn', 'shorthand'],
    'vue/v-on-style': ['warn', 'shorthand'],
    'vue/v-slot-style': ['warn', 'shorthand'],
    'vue/order-in-components': ['warn', {
      order: [
        ['name', 'inheritAttrs', 'components', 'directives'],
        ['props', 'emits'],
        ['setup'],
        ['data', 'computed', 'watch'],
        ['beforeCreate', 'created', 'beforeMount', 'mounted',
         'beforeUpdate', 'updated', 'beforeUnmount', 'unmounted'],
        ['methods'],
        ['template'],
      ],
    }],
    'vue/attributes-order': ['warn', {
      order: [
        'DEFINITION',
        'LIST_RENDERING',
        'CONDITIONALS',
        'RENDER_MODIFIERS',
        'GLOBAL',
        'UNIQUE',
        'TWO_WAY_BINDING',
        'OTHER_DIRECTIVES',
        'OTHER_ATTR',
        'EVENTS',
        'CONTENT',
      ],
    }]),
  },
  globals: {
    defineProps: 'readonly',
    defineEmits: 'readonly',
    defineExpose: 'readonly',
    withDefaults: 'readonly',
  },
}
