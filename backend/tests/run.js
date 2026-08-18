'use strict'
/**
 * 测试入口：在隔离临时数据目录中运行全部测试。
 * 顺序：引擎单元测试 -> HTTP 集成测试。最终打印汇总并以退出码反映结果。
 *
 * 用法：node tests/run.js    （或通过 npm test）
 */
const fs = require('fs')
const os = require('os')
const path = require('path')

// 测试隔离：指向临时目录，避免污染 backend/data
const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'ous-test-'))
process.env.OUS_DATA_DIR = tmpDir

const { SUITE } = require('./_harness')
const runUnit = require('./unit.test')
const runIntegration = require('./integration.test')

;(async () => {
  console.log('\n========================================')
  console.log('  OUS 后端测试套件  (零依赖 Node.js)')
  console.log('  临时数据目录: ' + tmpDir)
  console.log('========================================')

  try {
    await runUnit()
  } catch (e) {
    console.error('单元测试崩溃:', e)
    SUITE.fail++
  }

  try {
    await runIntegration()
  } catch (e) {
    console.error('集成测试崩溃:', e)
    SUITE.fail++
  }

  console.log('\n========================================')
  console.log(`  结果: ${SUITE.pass} 通过 / ${SUITE.fail} 失败`)
  console.log('========================================')

  // 清理临时目录
  try {
    fs.rmSync(tmpDir, { recursive: true, force: true })
  } catch (e) {}

  process.exit(SUITE.fail === 0 ? 0 : 1)
})()
