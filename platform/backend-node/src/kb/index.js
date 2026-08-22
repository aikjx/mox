'use strict';

/**
 * 知识库域门面（AINA A3 单一真相源 / A4 依赖单向）
 * routes 只允许经此门面使用 kb 域能力，禁止直连 infrastructure。
 */

const { analyzeDocument, extractEntitiesFromContent, suggestCategory } = require('./domain/document-analyzer');
const { diffVersions } = require('./domain/version-differ');
const store = require('./infrastructure/kb-store');

module.exports = {
  analyzeDocument,
  extractEntitiesFromContent,
  suggestCategory,
  diffVersions,
  store
};
