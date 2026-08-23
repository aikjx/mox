'use strict';

/**
 * 知识库域门面（AINA A3 单一真相源 / A4 依赖单向）
 * routes 只允许经此门面使用 kb 域能力，禁止直连 infrastructure。
 *
 * 文档→图谱自动化管道（全维归一化 · 云端文档资源维度）：
 *   getDocGraphPipeline() 懒装配单例 —— 域清单匹配器运行时从 project-atlas
 *   只读注入（调用期 require，规避模块加载环，保持 kb 域加载零依赖）。
 */

const { analyzeDocument, extractEntitiesFromContent, suggestCategory } = require('./domain/document-analyzer');
const { diffVersions } = require('./domain/version-differ');
const {
  extractAllEntities, mineEntityRelations, matchEntityToDomains,
  extractRequirementItems, extractBusinessRules,
  extractArchitectureNodes, extractModuleDefinitions, extractKeywords
} = require('./domain/entity-extractor');
const store = require('./infrastructure/kb-store');
const graphStore = require('./infrastructure/doc-graph-store');
const { createDocGraphPipeline } = require('./application/doc-graph-pipeline');

let pipelineInstance = null;

/** 懒装配单例：文档→图谱自动化管道（域匹配器运行时注入） */
function getDocGraphPipeline() {
  if (pipelineInstance) return pipelineInstance;
  let getDomains = () => [];
  try {
    // 运行时只读注入 project-atlas 域清单（调用期 require —— atlas 此时已完成装配）
    const atlas = require('../project-atlas');
    getDomains = () => atlas.getAtlasDomains();
  } catch (e) {
    // atlas 不可用时降级为无域匹配（纯实体图谱化仍然可用）
    getDomains = () => [];
  }
  pipelineInstance = createDocGraphPipeline({ store, graphStore, getDomains });
  return pipelineInstance;
}

module.exports = {
  analyzeDocument,
  extractEntitiesFromContent,
  suggestCategory,
  diffVersions,
  // 全维实体抽取（domain 纯函数，测试/路由可直接使用）
  extractAllEntities, mineEntityRelations, matchEntityToDomains,
  extractRequirementItems, extractBusinessRules,
  extractArchitectureNodes, extractModuleDefinitions, extractKeywords,
  // 文档→图谱自动化管道（application 用例）
  getDocGraphPipeline,
  store,
  graphStore
};
