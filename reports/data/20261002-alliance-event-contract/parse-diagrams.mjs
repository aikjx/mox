import { readFileSync } from 'node:fs'
import { Window } from '../../../frontend-ui/node_modules/happy-dom/lib/index.js'

const window = new Window()
globalThis.window = window
globalThis.document = window.document
const { default: mermaid } = await import('../../../frontend-ui/node_modules/mermaid/dist/mermaid.core.mjs')
mermaid.initialize({ startOnLoad: false, securityLevel: 'strict' })
let count = 0
for (const filename of ['21-event-delivery-contract.md']) {
  const file = new URL('../../../docs/expert-alliance/' + filename, import.meta.url)
  const text = readFileSync(file, 'utf8')
  for (const match of text.matchAll(/```mermaid\r?\n([\s\S]*?)```/g)) {
    await mermaid.parse(match[1])
    count++
  }
}
if (count !== 2) throw new Error(`Expected 2 diagrams, parsed ${count}`)
console.log(`Parsed ${count} diagrams with the actual Mermaid parser`)
window.happyDOM.abort()
