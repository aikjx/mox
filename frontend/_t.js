const fs = require('fs');
fs.writeFileSync('n4.txt', 'ver=' + process.version + '\nnodeWorks=true\n');
console.log('WROTE n4.txt');
