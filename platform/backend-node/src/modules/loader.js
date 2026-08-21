'use strict';

require('./graph');
require('./task');
require('./storage');

const { listModules, installAll } = require('./index');

function registerAllRoutes(regFn) {
  installAll(regFn);
}

function getModuleInfo() {
  return listModules();
}

module.exports = { registerAllRoutes, getModuleInfo };
