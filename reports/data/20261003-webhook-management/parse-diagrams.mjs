import { readFileSync } from 'node:fs'
import { Window } from '../../../frontend-ui/node_modules/happy-dom/lib/index.js'
const window = new Window()
globalThis.window = window
globalThis.document = window.document
const { default: mermaid } = await import('../../../frontend-ui/node_modules/mermaid/dist/mermaid.core.mjs')
mermaid.initialize({ startOnLoad: false, securityLevel: 'strict' })
const text = readFileSync(new URL('../../../docs/expert-alliance/21-event-delivery-contract.md', import.meta.url), 'utf8')
let count = 0
for (const match of text.matchAll(/```mermaid\r?\n([\s\S]*?)```/g)) {
  await mermaid.parse(match[1])
  count++
}
if (count !== 3) throw new Error(`Expected 3 diagrams, parsed ${count}`)
console.log(`Parsed ${count} diagrams with the actual Mermaid parser`)
window.happyDOM.abort()
