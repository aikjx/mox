// 自定义构建脚本：绕过 esbuild service（在受限 Windows 环境下 service 子进程会崩溃）
// 流程：用 @vue/compiler-sfc 预编译 .vue -> .vue.mjs，再用 esbuild CLI 单次进程 bundle
import { parse, compileScript, compileTemplate, compileStyle } from '@vue/compiler-sfc'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const root = __dirname
const srcDir = path.join(root, 'src')
const buildDir = path.join(root, '.build-src')
const distDir = path.join(root, 'dist')
const esbuildBin = path.join(root, 'node_modules/@esbuild/win32-x64/esbuild.exe')

function rmrf(p) {
  if (fs.existsSync(p)) fs.rmSync(p, { recursive: true, force: true })
}

function copyTree(src, dest) {
  fs.mkdirSync(dest, { recursive: true })
  for (const entry of fs.readdirSync(src, { withFileTypes: true })) {
    const s = path.join(src, entry.name)
    const d = path.join(dest, entry.name)
    if (entry.isDirectory()) copyTree(s, d)
    else fs.copyFileSync(s, d)
  }
}

function findVueFiles(dir) {
  const out = []
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, entry.name)
    if (entry.isDirectory()) out.push(...findVueFiles(p))
    else if (entry.name.endsWith('.vue')) out.push(p)
  }
  return out
}

// 1) 复制 src -> .build-src
rmrf(buildDir)
copyTree(srcDir, buildDir)

// 2) 编译所有 .vue 为同目录 .vue.mjs
const vueFiles = findVueFiles(buildDir)
for (const file of vueFiles) {
  const source = fs.readFileSync(file, 'utf-8')
  const { descriptor, errors } = parse(source, { filename: file })
  if (errors && errors.length) {
    console.error('SFC parse error in', file, errors)
    process.exit(1)
  }
  const script = compileScript(descriptor, { id: file })
  let templateCode = ''
  if (descriptor.template) {
    const t = compileTemplate({
      source: descriptor.template.content,
      filename: file,
      id: file,
      compilerOptions: { bindingMetadata: script.bindings },
    })
    if (t.errors && t.errors.length) {
      console.error('template compile error in', file, t.errors)
      process.exit(1)
    }
    templateCode = t.code
  }
  const css = descriptor.styles
    .map((s) => compileStyle({ source: s.content, filename: file, id: file, scoped: s.scoped }).code)
    .join('\n')

  let scriptCode = script.content.replace(/export\s+default\s+/, 'const __def = ')
  templateCode = templateCode.replace(/export\s+function\s+render/, 'function render')
  const out = `
${scriptCode}
${templateCode}
__def.render = render;
export default __def;
`
  // 样式作为单独 css 文件（同名 .vue.css），由入口收集
  const cssPath = file + '.css'
  if (css.trim()) fs.writeFileSync(cssPath, css)
  // 改写组件内对样式的引用为同名 css
  fs.writeFileSync(file + '.mjs', out)
}

// 3) 把 .build-src 内所有 .js/.mjs/.vue.mjs 中对 './x.vue' 的引用改为 './x.vue.mjs'
function rewriteImports(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      rewriteImports(p)
    } else if (/\.(js|mjs|vue\.mjs)$/.test(entry.name)) {
      let code = fs.readFileSync(p, 'utf-8')
      code = code.replace(/(from\s+['"])(\.[^'"]*?)\.vue(['"])/g, '$1$2.vue.mjs$3')
      fs.writeFileSync(p, code)
    }
  }
}
rewriteImports(buildDir)

// 4) esbuild CLI bundle（单次进程，绕开 service）
rmrf(distDir)
fs.mkdirSync(path.join(distDir, 'assets'), { recursive: true })

// 收集所有组件 css 到一个文件
let allCss = ''
for (const f of vueFiles) {
  const cp = f + '.css'
  if (fs.existsSync(cp)) allCss += '\n/* ' + path.basename(f) + ' */\n' + fs.readFileSync(cp, 'utf-8')
}
if (allCss.trim()) fs.writeFileSync(path.join(distDir, 'assets', 'components.css'), allCss)

const entry = path.join(buildDir, 'main.js')
const args = [
  entry,
  '--bundle',
  '--format=esm',
  '--outfile=' + path.join(distDir, 'assets', 'index.js'),
  '--loader:.vue=empty',
  '--loader:.css=css',
  '--define:__VUE_OPTIONS_API__=true',
  '--define:__VUE_PROD_DEVTOOLS__=false',
  '--define:__VUE_PROD_HYDRATION_MISMATCH_DETAILS__=false',
  '--log-level=warning',
]
const { spawnSync } = await import('node:child_process')
const res = spawnSync(esbuildBin, args, { cwd: root, encoding: 'utf-8' })
if (res.stdout) process.stdout.write(res.stdout)
if (res.stderr) process.stderr.write(res.stderr)
if (res.status !== 0) {
  console.error('esbuild bundle failed, status', res.status)
  process.exit(1)
}

// 5) 生成 index.html
const html = `<!DOCTYPE html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🧠</text></svg>" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>算子统一系统 · Operator Unified System</title>
    <link rel="stylesheet" href="/assets/components.css" />
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/assets/index.js"></script>
  </body>
</html>
`
fs.writeFileSync(path.join(distDir, 'index.html'), html)

console.log('BUILD OK -> dist/index.html')
