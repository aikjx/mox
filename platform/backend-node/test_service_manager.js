'use strict';

const { ServiceManager, SERVICE_DEFINITIONS } = require('./src/service-manager');

function log(msg, type = 'info') {
  const colors = {
    info: '\x1b[36m',
    success: '\x1b[32m',
    error: '\x1b[31m',
    warning: '\x1b[33m',
    reset: '\x1b[0m'
  };
  const color = colors[type] || colors.info;
  console.log(`${color}[service-test]\x1b[0m ${msg}`);
}

async function test() {
  log('=== 服务管理器全面测试开始 ===', 'info');

  const manager = new ServiceManager();

  log('1. 测试获取所有服务状态...', 'info');
  const allStatus = await manager.getAllStatus();
  log(`   总服务: ${allStatus.total}, 运行中: ${allStatus.running}, 已停止: ${allStatus.stopped}`, 'success');

  log('2. 测试获取单个服务状态...', 'info');
  for (const [id, def] of Object.entries(SERVICE_DEFINITIONS)) {
    const status = await manager.getServiceStatus(id);
    log(`   ${def.name} (${id}): ${status.running ? '运行中' : '已停止'}, PID: ${status.pid || 'N/A'}, 端口: ${status.port}`, status.running ? 'success' : 'warning');
  }

  log('3. 测试启动API服务...', 'info');
  const startResult = await manager.startService('api');
  if (startResult.success) {
    log(`   ${startResult.message}`, 'success');
  } else {
    log(`   启动API服务失败: ${startResult.error}`, 'error');
  }

  log('4. 等待服务启动...', 'info');
  await new Promise(resolve => setTimeout(resolve, 3000));

  log('5. 验证API服务状态...', 'info');
  const apiStatus = await manager.getServiceStatus('api');
  log(`   API服务: ${apiStatus.running ? '运行中' : '已停止'}, PID: ${apiStatus.pid || 'N/A'}`, apiStatus.running ? 'success' : 'error');

  log('6. 测试获取服务日志...', 'info');
  const logs = manager.getServiceLog('api', 5);
  log(`   获取到 ${logs.length} 条日志`, logs.length > 0 ? 'success' : 'warning');

  log('7. 测试停止API服务...', 'info');
  const stopResult = await manager.stopService('api');
  if (stopResult.success) {
    log(`   ${stopResult.message}`, 'success');
  } else {
    log(`   停止API服务失败: ${stopResult.error}`, 'error');
  }

  log('8. 测试重启服务（服务已停止状态下）...', 'info');
  const restartResult = await manager.restartService('api');
  if (restartResult.success) {
    log(`   ${restartResult.message}`, 'success');
  } else {
    log(`   重启API服务失败: ${restartResult.error}`, 'error');
  }

  log('9. 验证重启后状态...', 'info');
  await new Promise(resolve => setTimeout(resolve, 2000));
  const restartStatus = await manager.getServiceStatus('api');
  log(`   API服务: ${restartStatus.running ? '运行中' : '已停止'}`, restartStatus.running ? 'success' : 'warning');

  log('10. 清理并停止所有测试服务...', 'info');
  const cleanupResult = await manager.batchStop();
  log(`   清理完成: ${cleanupResult.total} 个服务`, 'success');

  log('\n=== 服务管理器测试完成 ===', 'success');
}

test().catch(err => {
  console.error('测试异常:', err);
  process.exit(1);
});
