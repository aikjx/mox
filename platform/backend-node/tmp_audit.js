// tmp_audit.js — one-off audit to find duplicated domain owners
const atlas = require('./src/project-atlas');
const { auditDomainOwnership } = require('./src/project-atlas/domain/project-registry');
const a = atlas.getAtlas();
const governableIds = new Set();
atlas.DOMAINS.forEach(d => governableIds.add(d.id));
atlas.MODULES.forEach(m => governableIds.add(m.id));
governableIds.delete('atlas-auto');
const o = auditDomainOwnership(a.projects, governableIds);
console.log('DUP:', JSON.stringify(o.duplicated));
console.log('ORPHANS (first 8):', JSON.stringify(o.orphans.slice(0, 8)));
console.log('ORPHANS len:', o.orphans.length);
