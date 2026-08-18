'use strict'
/**
 * 极简零依赖测试框架。
 * - test(name, fn): 执行 fn（可 async），失败记录错误
 * - section(title): 分组标题
 * - SUITE: 全局统计 { pass, fail }
 * 用法见 unit.test.js / integration.test.js，统一由 run.js 驱动。
 */
const assert = require('assert')

const SUITE = { pass: 0, fail: 0 }

async function test(name, fn) {
  try {
    await fn()
    SUITE.pass++
    console.log('  \u2713 ' + name)
  } catch (e) {
    SUITE.fail++
    console.log('  \u2717 ' + name + '   -> ' + (e && e.message ? e.message : String(e)))
  }
}

function section(title) {
  console.log('\n=== ' + title + ' ===')
}

// 透传 assert 常用断言（带自定义消息）
function assertEqual(a, b, msg) {
  assert.strictEqual(a, b, msg)
}
function assertDeep(a, b, msg) {
  assert.deepStrictEqual(a, b, msg)
}
function assertIncludes(arr, item, msg) {
  assert.ok(arr.includes(item), msg || `期望包含 ${JSON.stringify(item)}，实际 ${JSON.stringify(arr)}`)
}
function assertRange(v, lo, hi, msg) {
  assert.ok(v >= lo && v <= hi, msg || `值 ${v} 不在 [${lo}, ${hi}]`)
}

module.exports = { test, section, assert, assertEqual, assertDeep, assertIncludes, assertRange, SUITE }
