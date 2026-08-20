const path = require('path');
const fs = require('fs');
const { migrateFromJSON } = require('./db');

const DATA_DIR = path.join(__dirname, '..', 'data');

console.log('===== OUS 数据库迁移工具 =====');
console.log(`数据目录: ${DATA_DIR}`);

if (!fs.existsSync(DATA_DIR)) {
  console.log('数据目录不存在，创建中...');
  fs.mkdirSync(DATA_DIR, { recursive: true });
}

const jsonFiles = fs.readdirSync(DATA_DIR).filter(f => f.endsWith('.json'));
console.log(`发现 ${jsonFiles.length} 个JSON文件: ${jsonFiles.join(', ')}`);

const count = migrateFromJSON(DATA_DIR);

console.log(`\n迁移完成！共迁移 ${count} 条记录到数据库。`);
console.log('数据库文件位于: ' + path.join(DATA_DIR, 'ous.db'));
console.log('\n提示：JSON文件保留在原位作为备份，可手动删除。');
