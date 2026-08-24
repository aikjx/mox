import { fileURLToPath } from 'node:url'
export default {
  stories: ['../src/**/*.mdx', '../src/**/*.stories.@(js|jsx|ts|tsx|mdx)'],
  addons: ['@storybook/addon-essentials'],
  framework: { name: '@storybook/vue3-vite', options: {} },
  viteFinal(config) { return config }
}
