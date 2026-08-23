'use strict';

const fs = require('fs');
const path = require('path');
const { exec, spawn } = require('child_process');
const { config } = require('./config');

const RUNTIME_DIR = path.join(__dirname, '..', '.runtime');
if (!fs.existsSync(RUNTIME_DIR)) {
  fs.mkdirSync(RUNTIME_DIR, { recursive: true });
}

const SERVICE_DEFINITIONS = {
  api: {
    id: 'api',
    name: 'API 网关服务',
    description: '主 API 网关，端口 3010',
    command: 'node',
    args: ['src/api-server.js'],
    workingDir: path.join(__dirname, '..'),
    port: 3010,
    pidFile: path.join(RUNTIME_DIR, 'api.pid'),
    logFile: path.join(RUNTIME_DIR, 'api.log'),
    healthCheck: '/health',
    priority: 1,
    autoStart: true
  },
  frontend: {
    id: 'frontend',
    name: '前端静态服务',
    description: '前端静态托管，端口 3000',
    command: 'node',
    args: ['src/server.js'],
    workingDir: path.join(__dirname, '..'),
    port: 3000,
    pidFile: path.join(RUNTIME_DIR, 'frontend.pid'),
    logFile: path.join(RUNTIME_DIR, 'frontend.log'),
    healthCheck: '/',
    priority: 2,
    autoStart: false
  }
};

class ServiceManager {
  constructor() {
    this.processes = new Map();
    this.watchdogInterval = null;
    this.init();
  }

  init() {
    this.restoreProcesses();
    this.startWatchdog();
    console.log('[service-manager] 初始化完成，已恢复 ' + this.processes.size + ' 个服务');
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
        if (error) {
          resolve(null);
          return;
        }
        if (!stdout.trim()) {
          resolve(null);
          return;
        }
        const parts = stdout.trim().split(/\s+/);
        const pid = parseInt(parts[parts.length - 1], 10);
        resolve(isNaN(pid) ? null : pid);
      });
    });
  }

  async startService(serviceId, options = {}) {
    const svc = SERVICE_DEFINITIONS[serviceId];
    if (!svc) {
      return { success: false, error: `未知服务: ${serviceId}` };
    }

    if (this.processes.has(serviceId)) {
      const info = this.processes.get(serviceId);
      if (this.isProcessRunning(info.pid)) {
        return { success: true, message: `服务 ${svc.name} 已在运行 (PID: ${info.pid})`, alreadyRunning: true };
      }
    }

    if (await this.isPortInUse(svc.port)) {
      return { success: false, error: `端口 ${svc.port} 已被占用，无法启动 ${svc.name}` };
    }

    return new Promise((resolve) => {
      const shellCmd = process.platform === 'win32' ? true : false;
      const child = spawn(svc.command, svc.args, {
        cwd: svc.workingDir,
        detached: true,
        stdio: ['ignore', fs.openSync(svc.logFile, 'a'), fs.openSync(svc.logFile, 'a')],
        shell: shellCmd,
        windowsHide: true
      });

      child.unref();

      const processInfo = {
        pid: child.pid,
        serviceId,
        startedAt: new Date(),
        command: svc.command,
        args: svc.args
      };

      fs.writeFileSync(svc.pidFile, String(child.pid));
      this.processes.set(serviceId, processInfo);

      setTimeout(async () => {
        if (this.isProcessRunning(child.pid)) {
          resolve({
            success: true,
            message: `服务 ${svc.name} 启动成功 (PID: ${child.pid}, 端口: ${svc.port})`,
            pid: child.pid,
            port: svc.port
          });
        } else {
          try { fs.unlinkSync(svc.pidFile); } catch (e) {}
          this.processes.delete(serviceId);
          resolve({
            success: false,
            error: `服务 ${svc.name} 启动失败，进程已退出`
          });
        }
      }, 2000);
    });
  }

  async stopService(serviceId, options = {}) {
    const svc = SERVICE_DEFINITIONS[serviceId];
    if (!svc) {
      return { success: false, error: `未知服务: ${serviceId}` };
    }

    let pid = null;
    let processInfo = this.processes.get(serviceId);

    if (processInfo && this.isProcessRunning(processInfo.pid)) {
      pid = processInfo.pid;
    } else {
      if (fs.existsSync(svc.pidFile)) {
        try {
          pid = parseInt(fs.readFileSync(svc.pidFile, 'utf8').trim(), 10);
          if (!this.isProcessRunning(pid)) pid = null;
        } catch (e) { pid = null; }
      }
      if (!pid) {
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

      exec(killCmd, (error, stdout, stderr) => {
        try { fs.unlinkSync(svc.pidFile); } catch (e) {}
        this.processes.delete(serviceId);

        if (error) {
          resolve({
            success: false,
            error: `停止服务 ${svc.name} 失败: ${error.message}`
          });
        } else {
          resolve({
            success: true,
            message: `服务 ${svc.name} 已停止 (PID: ${pid})`
          });
        }
      });
    });
  }

  async restartService(serviceId) {
    const svc = SERVICE_DEFINITIONS[serviceId];
    if (!svc) {
      return { success: false, error: `未知服务: ${serviceId}` };
    }

    const status = await this.getServiceStatus(serviceId);
    if (status.running) {
      const stopResult = await this.stopService(serviceId);
      if (!stopResult.success) {
        return stopResult;
      }
      await this.delay(1000);
    }

    return await this.startService(serviceId);
  }

  async batchStart(serviceIds = null) {
    const ids = serviceIds || Object.keys(SERVICE_DEFINITIONS);
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
    if (!svc) {
      return { id: serviceId, error: '未知服务' };
    }

    let pid = null;
    let running = false;
    let info = this.processes.get(serviceId);

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
      autoStart: svc.autoStart
    };
  }

  async getAllStatus() {
    const statuses = [];
    for (const id of Object.keys(SERVICE_DEFINITIONS)) {
      const status = await this.getServiceStatus(id);
      statuses.push(status);
    }
    return {
      total: statuses.length,
      running: statuses.filter(s => s.running).length,
      stopped: statuses.filter(s => !s.running).length,
      services: statuses
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

  delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

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
  SERVICE_DEFINITIONS
};
