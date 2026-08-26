'use strict';

/**
 * 服务管理器（Service Manager）—— 遵循 [C3 单一真源] 原则
 * ============================================================
 * 唯一配置源：根目录 platform_config.json（版本 2.1+）
 * 禁止在本文件内硬编码 SERVICE_DEFINITIONS；任何服务变更只改 platform_config.json
 *
 * 冲突修复（V2.1）：
 *   - 旧版硬编码 frontend 端口 3000 → 统一读取 platform_config.json frontend.port=3020
 *   - 旧版缺失 xiaobai_voice → 统一从 platform_config.json.services 展开
 *   - 旧版 auto_start 不一致 → 以 platform_config.json 为权威
 */

const fs = require('fs');
const path = require('path');
const { exec, spawn } = require('child_process');

// ===== [C3] 唯一配置源：根目录 platform_config.json =====
const ROOT_DIR = path.resolve(__dirname, '..', '..', '..');
const PLATFORM_CONFIG_PATH = path.join(ROOT_DIR, 'platform_config.json');

function loadPlatformConfig() {
  if (!fs.existsSync(PLATFORM_CONFIG_PATH)) {
    throw new Error(`[service-manager] 缺失权威配置: ${PLATFORM_CONFIG_PATH}`);
  }
  const raw = fs.readFileSync(PLATFORM_CONFIG_PATH, 'utf8');
  const cfg = JSON.parse(raw);
  if (!cfg || !cfg.services || typeof cfg.services !== 'object') {
    throw new Error('[service-manager] platform_config.json 缺少 services 定义');
  }
  return cfg;
}

// ===== 运行时目录 =====
const RUNTIME_DIR = path.join(__dirname, '..', '.runtime');
if (!fs.existsSync(RUNTIME_DIR)) {
  fs.mkdirSync(RUNTIME_DIR, { recursive: true });
}

// ===== [C3] 从权威配置生成服务定义（禁止本地硬编码） =====
function buildServiceDefinitions() {
  const cfg = loadPlatformConfig();
  const defs = {};
  for (const [id, svc] of Object.entries(cfg.services)) {
    const workingDir = path.resolve(ROOT_DIR, svc.cwd || '.');
    // 兼容 command 字段："npm run dev" → 拆成 command=npm, args=['run','dev']
    let command, args;
    if (Array.isArray(svc.args) && svc.args.length > 0) {
      command = svc.args[0];
      args = svc.args.slice(1);
    } else if (typeof svc.command === 'string') {
      const parts = svc.command.split(/\s+/).filter(Boolean);
      command = parts[0];
      args = parts.slice(1);
    } else {
      throw new Error(`[service-manager] 服务 ${id} 缺少可执行命令定义`);
    }
    defs[id] = {
      id,
      name: svc.name || id,
      description: svc.description || '',
      command,
      args,
      workingDir,
      port: svc.port,
      healthCheck: svc.health_check || '/',
      priority: svc.startup_order_hint || 99,
      autoStart: !!svc.auto_start,
      pidFile: path.join(RUNTIME_DIR, `${id}.pid`),
      logFile: path.join(RUNTIME_DIR, `${id}.log`),
      tags: svc.tags || [],
      dependsOn: svc.depends_on || [],
      waitTime: (svc.wait_time || 10) * 1000,
      restartDelay: (svc.restart_delay || 3) * 1000,
      binaryRequires: svc.binary_requires || [],
      npmDeps: !!svc.npm_deps,
      isAdminOnly: !!svc.is_admin_only,
    };
  }
  return defs;
}

const SERVICE_DEFINITIONS = buildServiceDefinitions();

// ===== 导出原始配置加载器（供路由 /api/v1/services/config 展示权威元数据） =====
function getPlatformConfig() {
  return loadPlatformConfig();
}

class ServiceManager {
  constructor() {
    this.processes = new Map();
    this.watchdogInterval = null;
    this.init();
  }

  init() {
    this.restoreProcesses();
    this.startWatchdog();
    const ids = Object.keys(SERVICE_DEFINITIONS);
    console.log(`[service-manager] 初始化完成，配置源=${path.basename(PLATFORM_CONFIG_PATH)}，服务数=${ids.length}(${ids.join(',')})，已恢复 ${this.processes.size} 个运行中服务`);
  }

  restoreProcesses() {
    for (const [id, svc] of Object.entries(SERVICE_DEFINITIONS)) {
      if (fs.existsSync(svc.pidFile)) {
        try {
          const pid = parseInt(fs.readFileSync(svc.pidFile, 'utf8').trim(), 10);
          if (this.isProcessRunning(pid)) {
            this.processes.set(id, { pid, serviceId: id, startedAt: this.getFileTimestamp(svc.pidFile) });
            console.log(`[service-manager] 恢复服务: ${svc.name} (PID: ${pid})`);
          } else {
            fs.unlinkSync(svc.pidFile);
          }
        } catch (e) {
          try { fs.unlinkSync(svc.pidFile); } catch (e2) {}
        }
      }
    }
  }

  getFileTimestamp(filePath) {
    try {
      const stat = fs.statSync(filePath);
      return stat.birthtime || stat.mtime;
    } catch (e) {
      return new Date();
    }
  }

  isProcessRunning(pid) {
    try {
      process.kill(pid, 0);
      return true;
    } catch (e) {
      return false;
    }
  }

  isPortInUse(port) {
    return new Promise((resolve) => {
      const netstatCmd = process.platform === 'win32'
        ? `netstat -ano | findstr :${port} | findstr LISTENING`
        : `lsof -i :${port} -t`;

      exec(netstatCmd, (error, stdout) => {
        if (error) {
          resolve(false);
        } else {
          resolve(stdout.trim().length > 0);
        }
      });
    });
  }

  async getPidOnPort(port) {
    return new Promise((resolve) => {
      const cmd = process.platform === 'win32'
        ? `netstat -ano | findstr :${port} | findstr LISTENING`
        : `lsof -i :${port} -t`;

      exec(cmd, (error, stdout) => {
        if (error) { resolve(null); return; }
        if (!stdout.trim()) { resolve(null); return; }
        const parts = stdout.trim().split(/\s+/);
        const pid = parseInt(parts[parts.length - 1], 10);
        resolve(isNaN(pid) ? null : pid);
      });
    });
  }

  async startService(serviceId, options = {}) {
    const svc = SERVICE_DEFINITIONS[serviceId];
    if (!svc) return { success: false, error: `未知服务: ${serviceId}（可用: ${Object.keys(SERVICE_DEFINITIONS).join(', ')}）` };

    if (this.processes.has(serviceId)) {
      const info = this.processes.get(serviceId);
      if (this.isProcessRunning(info.pid)) {
        return { success: true, message: `服务 ${svc.name} 已在运行 (PID: ${info.pid})`, alreadyRunning: true };
      }
    }

    if (svc.port && await this.isPortInUse(svc.port)) {
      const holderPid = await this.getPidOnPort(svc.port);
      return { success: false, error: `端口 ${svc.port} 已被占用${holderPid ? `(PID:${holderPid})` : ''}，无法启动 ${svc.name}` };
    }

    return new Promise((resolve) => {
      const shellCmd = process.platform === 'win32' ? true : false;
      let logFdOut, logFdErr;
      try {
        logFdOut = fs.openSync(svc.logFile, 'a');
        logFdErr = fs.openSync(svc.logFile, 'a');
      } catch (e) {
        logFdOut = 'ignore'; logFdErr = 'ignore';
      }
      const child = spawn(svc.command, svc.args, {
        cwd: svc.workingDir,
        detached: true,
        stdio: ['ignore', logFdOut, logFdErr],
        shell: shellCmd,
        windowsHide: true,
      });

      child.unref();

      const processInfo = {
        pid: child.pid,
        serviceId,
        startedAt: new Date(),
        command: svc.command,
        args: svc.args,
      };

      fs.writeFileSync(svc.pidFile, String(child.pid));
      this.processes.set(serviceId, processInfo);

      // 按配置 waitTime 确认存活（默认 platform_config.json.wait_time 秒）
      setTimeout(async () => {
        if (this.isProcessRunning(child.pid)) {
          resolve({
            success: true,
            message: `服务 ${svc.name} 启动成功 (PID: ${child.pid}, 端口: ${svc.port || 'N/A'})`,
            pid: child.pid,
            port: svc.port,
          });
        } else {
          try { fs.unlinkSync(svc.pidFile); } catch (e) {}
          this.processes.delete(serviceId);
          let tailErr = '';
          try {
            if (typeof svc.logFile === 'string' && fs.existsSync(svc.logFile)) {
              const lines = fs.readFileSync(svc.logFile, 'utf8').split('\n').filter(Boolean).slice(-8);
              tailErr = lines.length ? `\n  最近日志:\n    ${lines.join('\n    ')}` : '';
            }
          } catch (_) {}
          resolve({
            success: false,
            error: `服务 ${svc.name} 启动失败，进程已退出（cwd=${svc.workingDir}，cmd=${svc.command} ${svc.args.join(' ')}）${tailErr}`,
          });
        }
      }, Math.min(30000, Math.max(2000, svc.waitTime || 10000)));
    });
  }

  async stopService(serviceId, options = {}) {
    const svc = SERVICE_DEFINITIONS[serviceId];
    if (!svc) return { success: false, error: `未知服务: ${serviceId}` };

    let pid = null;
    const processInfo = this.processes.get(serviceId);

    if (processInfo && this.isProcessRunning(processInfo.pid)) {
      pid = processInfo.pid;
    } else {
      if (fs.existsSync(svc.pidFile)) {
        try {
          pid = parseInt(fs.readFileSync(svc.pidFile, 'utf8').trim(), 10);
          if (!this.isProcessRunning(pid)) pid = null;
        } catch (e) { pid = null; }
      }
      if (!pid && svc.port) {
        pid = await this.getPidOnPort(svc.port);
      }
    }

    if (!pid) {
      this.processes.delete(serviceId);
      try { fs.unlinkSync(svc.pidFile); } catch (e) {}
      return { success: true, message: `服务 ${svc.name} 未在运行` };
    }

    return new Promise((resolve) => {
      const killCmd = process.platform === 'win32'
        ? `taskkill /F /PID ${pid}`
        : `kill -9 ${pid}`;

      exec(killCmd, (error) => {
        try { fs.unlinkSync(svc.pidFile); } catch (e) {}
        this.processes.delete(serviceId);

        if (error) {
          resolve({ success: false, error: `停止服务 ${svc.name} 失败: ${error.message}` });
        } else {
          resolve({ success: true, message: `服务 ${svc.name} 已停止 (PID: ${pid})` });
        }
      });
    });
  }

  async restartService(serviceId) {
    const svc = SERVICE_DEFINITIONS[serviceId];
    if (!svc) return { success: false, error: `未知服务: ${serviceId}` };

    const status = await this.getServiceStatus(serviceId);
    if (status.running) {
      const stopResult = await this.stopService(serviceId);
      if (!stopResult.success) return stopResult;
      await this.delay(1000);
    }
    return await this.startService(serviceId);
  }

  async batchStart(serviceIds = null) {
    const ids = serviceIds || Object.keys(SERVICE_DEFINITIONS).sort((a, b) =>
      (SERVICE_DEFINITIONS[a].priority || 99) - (SERVICE_DEFINITIONS[b].priority || 99)
    );
    const results = [];
    for (const id of ids) {
      const result = await this.startService(id);
      results.push({ serviceId: id, ...result });
    }
    return { total: ids.length, results };
  }

  async batchStop(serviceIds = null) {
    const ids = serviceIds || Object.keys(SERVICE_DEFINITIONS);
    const results = [];
    for (const id of ids) {
      const result = await this.stopService(id);
      results.push({ serviceId: id, ...result });
    }
    return { total: ids.length, results };
  }

  async batchRestart(serviceIds = null) {
    const ids = serviceIds || Object.keys(SERVICE_DEFINITIONS);
    const results = [];
    for (const id of ids) {
      const result = await this.restartService(id);
      results.push({ serviceId: id, ...result });
    }
    return { total: ids.length, results };
  }

  async getServiceStatus(serviceId) {
    const svc = SERVICE_DEFINITIONS[serviceId];
    if (!svc) return { id: serviceId, error: '未知服务' };

    let pid = null, running = false;
    const info = this.processes.get(serviceId);

    if (info) {
      running = this.isProcessRunning(info.pid);
      pid = running ? info.pid : null;
      if (!running) {
        this.processes.delete(serviceId);
        try { fs.unlinkSync(svc.pidFile); } catch (e) {}
      }
    } else if (fs.existsSync(svc.pidFile)) {
      try {
        pid = parseInt(fs.readFileSync(svc.pidFile, 'utf8').trim(), 10);
        running = this.isProcessRunning(pid);
        if (running) {
          this.processes.set(serviceId, { pid, serviceId, startedAt: this.getFileTimestamp(svc.pidFile) });
        } else {
          try { fs.unlinkSync(svc.pidFile); } catch (e) {}
        }
      } catch (e) {}
    }

    return {
      id: serviceId,
      name: svc.name,
      description: svc.description,
      running,
      pid,
      port: svc.port,
      priority: svc.priority,
      logFile: svc.logFile,
      pidFile: svc.pidFile,
      autoStart: svc.autoStart,
      tags: svc.tags,
      dependsOn: svc.dependsOn,
      cwd: svc.workingDir,
    };
  }

  async getAllStatus() {
    const statuses = [];
    for (const id of Object.keys(SERVICE_DEFINITIONS)) {
      statuses.push(await this.getServiceStatus(id));
    }
    return {
      total: statuses.length,
      running: statuses.filter(s => s.running).length,
      stopped: statuses.filter(s => !s.running).length,
      services: statuses,
    };
  }

  startWatchdog() {
    if (this.watchdogInterval) return;
    this.watchdogInterval = setInterval(async () => {
      for (const [id, info] of this.processes.entries()) {
        if (!this.isProcessRunning(info.pid)) {
          console.log(`[service-manager] 检测到服务异常退出: ${id} (PID: ${info.pid})`);
          this.processes.delete(id);
          const svc = SERVICE_DEFINITIONS[id];
          if (svc && fs.existsSync(svc.pidFile)) {
            try { fs.unlinkSync(svc.pidFile); } catch (e) {}
          }
        }
      }
    }, 10000);
  }

  delay(ms) { return new Promise(resolve => setTimeout(resolve, ms)); }

  getServiceLog(serviceId, lines = 50) {
    const svc = SERVICE_DEFINITIONS[serviceId];
    if (!svc) return [];
    try {
      if (!fs.existsSync(svc.logFile)) return [];
      const content = fs.readFileSync(svc.logFile, 'utf8');
      const allLines = content.split('\n').filter(l => l.trim());
      return allLines.slice(-lines).map(line => ({ line, timestamp: new Date().toISOString() }));
    } catch (e) {
      return [{ line: '读取日志失败: ' + e.message, timestamp: new Date().toISOString() }];
    }
  }

  clearServiceLog(serviceId) {
    const svc = SERVICE_DEFINITIONS[serviceId];
    if (!svc) return false;
    try {
      fs.writeFileSync(svc.logFile, '');
      return true;
    } catch (e) {
      return false;
    }
  }
}

let managerInstance = null;

function getServiceManager() {
  if (!managerInstance) {
    managerInstance = new ServiceManager();
  }
  return managerInstance;
}

module.exports = {
  ServiceManager,
  getServiceManager,
  SERVICE_DEFINITIONS,   // 来自 platform_config.json 的运行时构建结果
  getPlatformConfig,     // 暴露权威配置加载器（供路由层展示）
  PLATFORM_CONFIG_PATH,  // 暴露配置文件路径（诊断用）
  ROOT_DIR,              // 暴露项目根目录
};
