import { readFileSync } from 'node:fs'
import { Window } from '../../../frontend-ui/node_modules/happy-dom/lib/index.js'
const window = new Window()
globalThis.window = window
globalThis.document = window.document
const { default: mermaid } = await import('../../../frontend-ui/node_modules/mermaid/dist/mermaid.core.mjs')
mermaid.initialize({ startOnLoad: false, securityLevel: 'strict' })
let count = 0
for (const filename of ['modules/resource-knowledge/IMPLEMENTATION-STATUS.md']) {
  const source = readFileSync(new URL('../../../docs/' + filename, import.meta.url), 'utf8')
  for (const block of source.matchAll(/```mermaid\r?\n([\s\S]*?)```/g)) {
    await mermaid.parse(block[1])
    count++
  }
}
if (count < 2) throw new Error(`Expected knowledge diagrams, parsed ${count}`)
console.log(`Actual Mermaid parser validated ${count} module relationship and business flow diagrams`)
window.happyDOM.abort()
