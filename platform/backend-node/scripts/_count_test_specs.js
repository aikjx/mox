'use strict';
const fs = require('fs');
const path = require('path');
const testDir = path.join(__dirname, '..', 'test');
const files = fs.readdirSync(testDir).filter(f => f.endsWith('.js'));
let total = 0;
for (const f of files) {
  const t = fs.readFileSync(path.join(testDir, f), 'utf8');
  const m = t.match(/\bit\s*\(\s*['"`]/g);
  const c = (m || []).length;
  console.log(f.padEnd(60), 'it()=', c);
  total += c;
}
console.log('\nTOTAL it() specs in test/ =', total);
if (total < 250) {
  console.log(`WARNING: ${total} < 250, threshold not reached`);
  process.exit(1);
} else {
  console.log(`OK: ${total} ≥ 250 ✔`);
}
