#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
璇玑系统 - 一键服务管理器
==============================
功能：一键启动/关闭/重启所有服务
服务列表：
  1. API 后端服务 (port 3010) - platform/backend-node/src/api-server.js
  2. 用户前端界面 (port 3020) - frontend-ui（含系统管理区 /admin）

使用方法：
  python service_manager.py start      # 启动所有服务
  python service_manager.py stop       # 关闭所有服务
  python service_manager.py restart    # 重启所有服务
  python service_manager.py status     # 查看服务状态
  python service_manager.py start api  # 启动单个服务
"""

import os
import sys
import signal
import time
import socket
import subprocess
import json
import shutil
from pathlib import Path
from datetime import datetime

# ============ 配置 ============
PROJECT_ROOT = Path(__file__).parent.resolve()
PID_DIR = PROJECT_ROOT / '.runtime'
LOG_DIR = PROJECT_ROOT / '.logs'

# 确保目录存在
PID_DIR.mkdir(exist_ok=True)
LOG_DIR.mkdir(exist_ok=True)

# 服务配置
SERVICES = {
    'api': {
        'name': 'API 后端服务',
        'port': 3010,
        'cwd': str(PROJECT_ROOT / 'platform' / 'backend-node'),
        'command': ['node', 'src/api-server.js'],
        'pid_file': PID_DIR / 'api.pid',
        'log_file': LOG_DIR / 'api.log',
        'wait_time': 5,
    },
    'frontend': {
        'name': '用户前端界面',
        'port': 3020,
        'cwd': str(PROJECT_ROOT / 'frontend-ui'),
        'command': ['npm', 'run', 'dev'],
        'pid_file': PID_DIR / 'frontend.pid',
        'log_file': LOG_DIR / 'frontend.log',
        'wait_time': 8,
    },
}

# ============ 工具函数 ============

def print_header():
    """打印标题"""
    print("\n" + "=" * 60)
    print("  璇玑系统 - 一键服务管理器")
    print("=" * 60)
    print()

def print_status_bar(service_key, status, color_code=0):
    """打印状态行"""
    service = SERVICES[service_key]
    status_map = {
        'running': ('[运行中]', '\033[92m'),
        'stopped': ('[已停止]', '\033[90m'),
        'starting': ('[启动中]', '\033[93m'),
        'error': ('[错误]', '\033[91m'),
        'ready': ('[就绪]', '\033[92m'),
    }
    text, color = status_map.get(status, ('[未知]', '\033[0m'))
    print(f"  {color}{text}\033[0m {service['name']} (端口 {service['port']})")

def is_port_in_use(port):
    """检查端口是否被占用"""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.settimeout(1)
        result = sock.connect_ex(('127.0.0.1', port))
        return result == 0

def check_http_service(port, endpoint='/', timeout=3):
    """检查 HTTP 服务是否可访问"""
    import urllib.request
    url = f'http://localhost:{port}{endpoint}'
    try:
        req = urllib.request.Request(url, method='HEAD')
        urllib.request.urlopen(req, timeout=timeout)
        return True
    except:
        # 尝试 GET 请求
        try:
            urllib.request.urlopen(url, timeout=timeout)
            return True
        except:
            return False

def get_pid(service_key):
    """获取服务 PID"""
    service = SERVICES[service_key]
    pid_file = service['pid_file']
    if pid_file.exists():
        try:
            return int(pid_file.read_text().strip())
        except:
            return None
    return None

def is_process_running(pid):
    """检查进程是否在运行"""
    if pid is None:
        return False
    try:
        if sys.platform == 'win32':
            # Windows: 使用 tasklist 检查
            result = subprocess.run(
                ['tasklist', '/FI', f'PID eq {pid}'],
                capture_output=True, timeout=3
            )
            try:
                stdout = result.stdout.decode('gbk')
            except:
                stdout = result.stdout.decode('utf-8', errors='replace')
            return str(pid) in stdout
        else:
            # Unix: 使用 kill 信号检查
            os.kill(pid, 0)
            return True
    except:
        return False

def get_service_status(service_key):
    """获取服务状态"""
    service = SERVICES[service_key]
    pid = get_pid(service_key)
    
    # 检查 PID 文件中的进程是否存活
    if pid and is_process_running(pid):
        # 进一步检查端口是否在监听
        if is_port_in_use(service['port']):
            return 'running'
        else:
            return 'starting'  # 进程存活但端口还没准备好
    elif pid:
        # PID 文件存在但进程已死，清理
        service['pid_file'].unlink(missing_ok=True)
    
    return 'stopped'

# ============ 启动服务 ============

def start_service(service_key):
    """启动单个服务"""
    service = SERVICES[service_key]
    status = get_service_status(service_key)
    
    if status == 'running':
        print(f"  ✓ {service['name']} 已在运行 (PID: {get_pid(service_key)})")
        return True
    
    print(f"  → 正在启动 {service['name']}...")
    
    # 检查端口是否被其他进程占用
    if is_port_in_use(service['port']):
        pid = get_pid(service_key)
        if pid and is_process_running(pid):
            # 本服务的其他实例正在运行
            print(f"    端口 {service['port']} 已被本服务占用")
        else:
            print(f"    ⚠ 端口 {service['port']} 被其他进程占用，尝试释放...")
            if not _free_port(service['port']):
                print(f"    ✗ 无法释放端口 {service['port']}")
                return False
    
    # 检查 Node.js 依赖是否已安装
    if service_key in ['api', 'frontend']:
        node_modules = Path(service['cwd']) / 'node_modules'
        if not node_modules.exists():
            print(f"    ⚠ 检测到 node_modules 不存在，正在安装依赖...")
            npm_install = subprocess.Popen(
                ['npm', 'install'],
                cwd=service['cwd'],
                shell=sys.platform == 'win32'
            )
            npm_install.wait()
            if npm_install.returncode != 0:
                print(f"    ✗ npm install 失败")
                return False
            print(f"    ✓ 依赖安装完成")
    
    # 创建日志文件
    log_file = service['log_file'].open('w', encoding='utf-8')
    log_file.write(f"[{datetime.now().isoformat()}] Starting {service['name']}\n")
    
    # 启动进程
    try:
        if sys.platform == 'win32':
            # Windows 上使用 CREATE_NEW_PROCESS_GROUP 创建新进程组
            DETACHED_PROCESS = 0x00000008
            CREATE_NEW_PROCESS_GROUP = 0x00000200
            
            # 构建命令
            cmd = ' '.join(service['command'])
            
            proc = subprocess.Popen(
                service['command'],
                cwd=service['cwd'],
                shell=True,
                creationflags=DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP,
                stdout=log_file,
                stderr=log_file,
                stdin=subprocess.DEVNULL,
                close_fds=True,
            )
        else:
            proc = subprocess.Popen(
                service['command'],
                cwd=service['cwd'],
                stdout=log_file,
                stderr=log_file,
                stdin=subprocess.DEVNULL,
                close_fds=True,
                start_new_session=True,
            )
    except Exception as e:
        print(f"    ✗ 启动失败: {e}")
        log_file.close()
        return False
    
    # 保存 PID
    service['pid_file'].write_text(str(proc.pid))
    log_file.write(f"[{datetime.now().isoformat()}] Process started with PID {proc.pid}\n")
    log_file.close()
    
    # 等待服务就绪
    wait_time = service['wait_time']
    print(f"    等待服务就绪 (最多 {wait_time}s)...", end='', flush=True)
    
    ready = False
    for i in range(wait_time * 2):  # 每 0.5s 检查一次
        time.sleep(0.5)
        if is_port_in_use(service['port']):
            ready = True
            break
        # 检查进程是否已退出
        if not is_process_running(proc.pid):
            break
    
    if ready:
        print(f" \033[92m✓ 成功\033[0m (PID: {proc.pid}, 端口: {service['port']})")
        return True
    else:
        # 检查进程是否存活
        if is_process_running(proc.pid):
            print(f" \033[93m⏳ 运行中\033[0m (PID: {proc.pid}, 端口可能还在初始化)")
            print(f"    查看日志: {service['log_file']}")
            return True  # 进程存活，视为成功
        else:
            print(f" \033[91m✗ 失败\033[0m 进程已退出")
            service['pid_file'].unlink(missing_ok=True)
            print(f"    查看日志: {service['log_file']}")
            return False

def start_all():
    """启动所有服务"""
    print_header()
    print("正在启动所有服务...\n")
    
    results = {}
    for key in ['api', 'frontend']:
        results[key] = start_service(key)
        time.sleep(1)  # 服务间启动间隔
    
    print("\n" + "-" * 60)
    print("启动结果汇总:")
    print("-" * 60)
    for key, success in results.items():
        status = get_service_status(key)
        print_status_bar(key, status)
    
    return all(results.values())

# ============ 关闭服务 ============

def stop_service(service_key, force=False):
    """关闭单个服务"""
    service = SERVICES[service_key]
    pid = get_pid(service_key)
    status = get_service_status(service_key)
    
    if status == 'stopped':
        # 确保没有残留进程
        if pid and is_process_running(pid):
            print(f"  → 清理残留进程 (PID: {pid})...")
            _kill_process(pid, force=True)
        service['pid_file'].unlink(missing_ok=True)
        print(f"  ✓ {service['name']} 已停止")
        return True
    
    print(f"  → 正在关闭 {service['name']} (PID: {pid})...")
    
    if pid and is_process_running(pid):
        # 优雅关闭
        _graceful_shutdown(pid, force=force)
        
        # 等待进程退出
        max_wait = 10 if not force else 3
        waited = 0
        while waited < max_wait:
            if not is_process_running(pid):
                break
            time.sleep(0.5)
            waited += 0.5
        
        # 如果还在运行，强制关闭
        if is_process_running(pid):
            if not force:
                print(f"    进程未响应，尝试强制关闭...")
            _kill_process(pid, force=True)
            time.sleep(1)
    elif pid:
        print(f"    PID {pid} 已不存在")
    
    # 清理 PID 文件
    service['pid_file'].unlink(missing_ok=True)
    
    # 确认端口已释放
    if is_port_in_use(service['port']):
        print(f"    ⚠ 端口 {service['port']} 仍被占用")
        if not _free_port(service['port']):
            print(f"    请手动检查占用端口 {service['port']} 的进程")
            return False
    
    print(f"  ✓ {service['name']} 已停止")
    return True

def stop_all(force=False):
    """关闭所有服务"""
    print_header()
    print("正在关闭所有服务...\n")
    
    # 按依赖关系关闭：前端先关，后端后关
    stop_order = ['frontend', 'api']
    
    results = {}
    for key in stop_order:
        results[key] = stop_service(key, force=force)
        time.sleep(0.5)
    
    print("\n" + "-" * 60)
    print("关闭结果汇总:")
    print("-" * 60)
    for key in ['api', 'frontend']:
        print_status_bar(key, 'stopped')
    
    return all(results.values())

# ============ 重启服务 ============

def restart_service(service_key):
    """重启单个服务"""
    print(f"\n  --- 重启 {SERVICES[service_key]['name']} ---")
    stop_service(service_key, force=False)
    time.sleep(1)
    return start_service(service_key)

def restart_all():
    """重启所有服务"""
    print_header()
    print("正在重启所有服务...\n")
    
    # 先全部关闭
    stop_all(force=False)
    print()
    time.sleep(2)
    
    # 再全部启动
    return start_all()

# ============ 服务状态检查 ============

def show_status():
    """显示所有服务状态"""
    print_header()
    print("服务状态检查:\n")
    
    # 定义每个服务的健康检查端点
    health_endpoints = {
        'api': '/health',          # API 服务健康检查端点
        'frontend': '/',           # 前端 Vite 根路径
            }
    
    all_running = True
    for key in ['api', 'frontend']:
        status = get_service_status(key)
        pid = get_pid(key)
        service = SERVICES[key]
        
        if status == 'running':
            # 检查 HTTP 可用性 - 使用特定端点
            endpoint = health_endpoints.get(key, '/')
            http_ok = check_http_service(service['port'], endpoint)
            if http_ok:
                status = 'running'
            else:
                # 端口在监听但特定端点不可用，尝试根路径
                http_ok = check_http_service(service['port'], '/')
                if http_ok:
                    status = 'running'  # 根路径可访问也算正常
                else:
                    status = 'starting'  # 端口在监听但 HTTP 未就绪
        
        if status != 'running':
            all_running = False
        
        print_status_bar(key, status)
        if pid:
            print(f"      PID: {pid} | 端口: {service['port']}")
        else:
            print(f"      端口: {service['port']}")
    
    print("\n" + "-" * 60)
    if all_running:
        print("  \033[92m所有服务运行正常 ✓\033[0m")
    else:
        print("  \033[93m部分服务未就绪 ⚠\033[0m")
    print("-" * 60)
    
    # 显示访问地址
    print("\n  访问地址:")
    print(f"    用户界面: http://localhost:3020")
    print(f"    系统管理区: http://localhost:3020/#/admin")
    print(f"    API 服务: http://localhost:3010")
    print()
    
    return all_running

# ============ 进程管理辅助 ============

def _graceful_shutdown(pid, force=False):
    """优雅关闭进程"""
    if sys.platform == 'win32':
        # Windows: 发送关闭信号给进程树
        try:
            # 优先尝试通过 taskkill 优雅关闭
            CREATE_NO_WINDOW = 0x08000000
            result = subprocess.run(
                ['taskkill', '/PID', str(pid), '/T'],
                capture_output=True, timeout=5,
                creationflags=CREATE_NO_WINDOW
            )
            return result.returncode == 0
        except:
            pass
        
        return False
    else:
        # Unix: 发送 SIGTERM
        try:
            os.kill(pid, signal.SIGTERM)
            return True
        except:
            return False

def _kill_process(pid, force=False):
    """强制杀掉进程"""
    if sys.platform == 'win32':
        try:
            CREATE_NO_WINDOW = 0x08000000
            result = subprocess.run(
                ['taskkill', '/PID', str(pid), '/F', '/T'],
                capture_output=True, timeout=5,
                creationflags=CREATE_NO_WINDOW
            )
            return result.returncode == 0
        except:
            return False
    else:
        try:
            os.kill(pid, signal.SIGKILL if force else signal.SIGTERM)
            return True
        except:
            return False

def _free_port(port):
    """释放端口（通过 Windows netstat 查找占用进程）"""
    if sys.platform != 'win32':
        return False
    
    try:
        # 使用 netstat 查找占用端口的进程
        result = subprocess.run(
            ['netstat', '-ano', '-p', 'tcp'],
            capture_output=True, timeout=5
        )
        # 尝试多种编码解码
        try:
            stdout = result.stdout.decode('gbk')
        except:
            stdout = result.stdout.decode('utf-8', errors='replace')
        
        for line in stdout.splitlines():
            if f':{port}' in line and 'LISTENING' in line:
                # 获取 PID（最后一列）
                parts = line.strip().split()
                if len(parts) >= 5:
                    pid = int(parts[-1])
                    print(f"    发现占用端口 {port} 的进程 PID: {pid}")
                    
                    # 检查是否是本项目的进程
                    try:
                        proc_result = subprocess.run(
                            ['wmic', 'process', 'where', f'ProcessId={pid}', 'get', 'CommandLine'],
                            capture_output=True, timeout=3
                        )
                        try:
                            cmdline = proc_result.stdout.decode('gbk').lower()
                        except:
                            cmdline = proc_result.stdout.decode('utf-8', errors='replace').lower()
                        
                        # 只关闭与本项目相关的进程
                        project_path = str(PROJECT_ROOT).lower()
                        if project_path in cmdline or 'node' in cmdline or 'npm' in cmdline:
                            print(f"    关闭进程 {pid}...")
                            _kill_process(pid, force=True)
                            time.sleep(1)
                            return not is_port_in_use(port)
                        else:
                            print(f"    ⚠ 进程 {pid} 不属于本项目，跳过")
                            return False
                    except:
                        # 如果无法检查，仍然尝试关闭（因为端口冲突）
                        print(f"    尝试关闭进程 {pid}...")
                        _kill_process(pid, force=True)
                        time.sleep(1)
                        return not is_port_in_use(port)
    except Exception as e:
        print(f"    检查端口失败: {e}")
    
    return False

# ============ 日志 ============

def show_log(service_key, lines=50):
    """查看服务日志"""
    service = SERVICES[service_key]
    log_file = service['log_file']
    
    if not log_file.exists():
        print(f"  日志文件不存在: {log_file}")
        return
    
    print(f"\n  最近 {lines} 行日志 ({service['name']}):")
    print("  " + "-" * 50)
    
    try:
        content = log_file.read_text(encoding='utf-8', errors='replace')
        log_lines = content.strip().split('\n')
        for line in log_lines[-lines:]:
            print(f"  {line}")
    except Exception as e:
        print(f"  读取日志失败: {e}")

# ============ 帮助 ============

def show_help():
    """显示帮助信息"""
    print_header()
    print("使用方法:")
    print(f"  python {sys.argv[0]} start [service]      # 启动服务")
    print(f"  python {sys.argv[0]} stop [service]       # 关闭服务")
    print(f"  python {sys.argv[0]} restart [service]    # 重启服务")
    print(f"  python {sys.argv[0]} status               # 查看服务状态")
    print(f"  python {sys.argv[0]} logs [service]       # 查看服务日志")
    print(f"  python {sys.argv[0]} monitor              # 启动 Web 管理面板")
    print(f"  python {sys.argv[0]} help                 # 显示此帮助")
    print()
    print("可用服务:")
    for key, service in SERVICES.items():
        print(f"  {key:12s}  {service['name']:<20s}  端口: {service['port']}")
    print()
    print("管理面板:")
    print("  启动管理面板后可在浏览器访问所有服务状态和入口")
    print("  默认地址: http://localhost:3040")
    print()
    print("选项:")
    print("  start all    启动所有服务（默认）")
    print("  stop all     关闭所有服务")
    print("  stop -f      强制关闭所有服务")
    print()

# ============ 主入口 ============

def main():
    if len(sys.argv) < 2:
        show_help()
        sys.exit(0)
    
    action = sys.argv[1].lower()
    target = sys.argv[2].lower() if len(sys.argv) > 2 else 'all'
    force = '-f' in sys.argv or '--force' in sys.argv
    
    # 过滤掉选项参数
    args = [a for a in sys.argv[2:] if not a.startswith('-')]
    target = args[0] if args else 'all'
    force = '-f' in sys.argv or '--force' in sys.argv
    
    if action == 'help' or action == '--help' or action == '-h':
        show_help()
        return
    
    elif action == 'status':
        show_status()
        return
    
    elif action == 'logs':
        if target == 'all':
            print_header()
            for key in ['api', 'frontend']:
                show_log(key)
                print()
        elif target in SERVICES:
            show_log(target)
        else:
            print(f"  未知服务: {target}")
            show_help()
        return
    
    elif action == 'start':
        if target == 'all':
            start_all()
        elif target in SERVICES:
            print_header()
            start_service(target)
        else:
            print(f"  未知服务: {target}")
            show_help()
    
    elif action == 'stop':
        if target == 'all':
            stop_all(force=force)
        elif target in SERVICES:
            print_header()
            stop_service(target, force=force)
        else:
            print(f"  未知服务: {target}")
            show_help()
    
    elif action == 'restart':
        if target == 'all':
            restart_all()
        elif target in SERVICES:
            print_header()
            restart_service(target)
        else:
            print(f"  未知服务: {target}")
            show_help()
    
    elif action == 'monitor':
        # 启动 Web 管理面板
        import subprocess
        monitor_script = PROJECT_ROOT / 'service_monitor.py'
        if monitor_script.exists():
            print(f"  → 启动服务管理面板...")
            try:
                subprocess.Popen(
                    [sys.executable, str(monitor_script)],
                    cwd=str(PROJECT_ROOT),
                    creationflags=0x00000008 if sys.platform == 'win32' else 0
                )
                time.sleep(1)
                print(f"  ✓ 管理面板已启动: http://localhost:3040")
            except Exception as e:
                print(f"  ✗ 启动失败: {e}")
        else:
            print(f"  ✗ 管理面板脚本不存在: {monitor_script}")
    
    else:
        print(f"  未知命令: {action}")
        show_help()

if __name__ == '__main__':
    main()