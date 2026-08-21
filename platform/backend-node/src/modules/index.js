'use strict';

const { uid } = require('../utils');

const modules = new Map();
const moduleOrder = [];

function registerModule(name, routes, options = {}) {
  if (modules.has(name)) {
    console.warn(`[modules] 模块 ${name} 已存在，将被覆盖`);
  }
  modules.set(name, { name, routes, options, registeredAt: Date.now() });
  if (!moduleOrder.includes(name)) moduleOrder.push(name);
  console.log(`[modules] 注册模块: ${name} (${routes.length} 个路由)`);
}

function getModule(name) { return modules.get(name); }
function listModules() { return moduleOrder.map(n => ({ name: n, ...modules.get(n) })); }
function getModuleRoutes(name) { return modules.get(name)?.routes || []; }

function installAll(registerFn) {
  let totalRoutes = 0;
  for (const name of moduleOrder) {
    const mod = modules.get(name);
    if (mod?.routes) {
      for (const route of mod.routes) {
        registerFn(route.method, route.path, route.handler);
        totalRoutes++;
      }
    }
  }
  console.log(`[modules] 已安装 ${modules.size} 个模块，共 ${totalRoutes} 个路由`);
}

const BaseModule = {
  uid,
  ok(res, data) {
    const body = JSON.stringify({ success: true, data });
    res.writeHead(200, { 'Content-Type': 'application/json; charset=utf-8', 'Access-Control-Allow-Origin': '*' });
    res.end(body);
  },
  fail(res, code, msg, extra) {
    const body = JSON.stringify({ success: false, error: msg, ...(extra || {}) });
    res.writeHead(code, { 'Content-Type': 'application/json; charset=utf-8', 'Access-Control-Allow-Origin': '*' });
    res.end(body);
  },
  async readBody(req) {
    return new Promise((resolve) => {
      let data = '';
      req.on('data', (c) => { data += c; });
      req.on('end', () => {
        if (!data) return resolve({});
        try { resolve(JSON.parse(data)); } catch (e) { resolve({}); }
      });
      req.on('error', () => resolve({}));
    });
  }
};

module.exports = { registerModule, getModule, listModules, getModuleRoutes, installAll, BaseModule };
