'use strict';

/**
 * 引擎内核（Engine Kernel）域门面
 * ------------------------------------------------------------------
 * 一切皆可插件化：AI 引擎/存储/搜索/音高检测……全部经标准槽位契约接入，
 * 切换引擎 = 换绑定（零代码改动，瞬间生效），三层商城供给插件，
 * AI 可依据需求自动配置引擎组合。
 */

const switchService = require('./application/switch-service');
const marketplaceService = require('./application/marketplace-service');
const aiConfigureService = require('./application/ai-configure-service');

module.exports = {
  // 槽位与契约
  getSlots: switchService.getSlots,
  getContract: switchService.getContract,
  getBindingsView: switchService.getBindingsView,
  // 瞬间切换
  switchEngine: switchService.switchEngine,
  validateEngine: switchService.validateEngine,
  // 三层商城
  getMarketplace: marketplaceService.getMarketplace,
  listSystemMarket: marketplaceService.listSystemMarket,
  listCloudMarket: marketplaceService.listCloudMarket,
  listLocalMarket: marketplaceService.listLocalMarket,
  installPlugin: marketplaceService.installPlugin,
  uninstallPlugin: marketplaceService.uninstallPlugin,
  getMarketplaceConfig: marketplaceService.getMarketplaceConfig,
  saveMarketplaceConfig: marketplaceService.saveMarketplaceConfig,
  // AI 自动配置
  aiConfigure: aiConfigureService.aiConfigure
};
