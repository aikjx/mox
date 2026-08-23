#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
璇玑系统 - 企业级服务管理平台
====================================
统一管理整个项目的所有服务生命周期

核心功能：
  1. 配置驱动 - 服务配置支持 JSON 配置文件
  2. 进程管理 - 启动/停止/重启/状态监控
  3. 健康检查 - HTTP 健康检查
  4. 日志聚合 - 统一日志查看
  5. Web 面板 - 可视化管理界面
  6. 权限控制 - 管理员认证
  7. 会话管理 - Cookie 会话

使用方法：
  python platform_manager.py start [service]     # 启动服务
  python platform_manager.py stop [service]      # 停止服务
  python platform_manager.py restart [service]   # 重启服务
  python platform_manager.py status             # 查看状态
  python platform_manager.py logs [service]     # 查看日志
  python platform_manager.py dashboard          # 启动 Web 管理面板
"""

import os
import sys
import json
import socket
import time
import hashlib
import secrets
import subprocess
import threading
import logging
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path
from datetime import datetime
from urllib.parse import urlparse

# ============ 配置 ============
PROJECT_ROOT = Path(__file__).parent.resolve()
DATA_DIR = PROJECT_ROOT / '.runtime'
LOG_DIR = PROJECT_ROOT / '.logs'
CONFIG_FILE = PROJECT_ROOT / 'platform_config.json'

DATA_DIR.mkdir(exist_ok=True)
LOG_DIR.mkdir(exist_ok=True)

# 管理员配置
ADMIN_USERNAME = os.environ.get('PM_ADMIN_USER', 'admin')
ADMIN_PASSWORD = os.environ.get('PM_ADMIN_PASS', 'admin123')
SESSION_TIMEOUT = 30 * 60


def hash_password(password):
    return hashlib.sha256(password.encode()).hexdigest()


ADMIN_PASSWORD_HASH = hash_password(ADMIN_PASSWORD)

# ============ 默认配置 ============
DEFAULT_CONFIG = {
    "version": "2.0",
    "project_name": "璇玑系统",
    "dashboard_port": 3040,
    "admin": {"username": "admin", "password": "admin123"},
    "services": {
        "api": {
            "name": "API 后端服务",
            "description": "璇玑系统核心 API 服务",
            "port": 3010,
            "health_check": "/health",
            "cwd": "platform/backend-node",
            "command": "node src/api-server.js",
            "npm_deps": True,
            "is_admin_only": True,
            "auto_start": True,
            "restart_delay": 3,
            "wait_time": 5,
            "tags": ["API", "后端", "核心"]
        },
        "frontend": {
            "name": "用户前端界面",
            "description": "面向终端用户的操作界面",
            "port": 3020,
            "health_check": "/",
            "cwd": "frontend-ui",
            "command": "npm run dev",
            "npm_deps": True,
            "is_admin_only": False,
            "auto_start": True,
            "wait_time": 8,
            "tags": ["前端", "用户界面", "Vite"]
        },
        "admin": {
            "name": "企业管理界面",
            "description": "企业级后台管理系统",
            "port": 3030,
            "health_check": "/",
            "cwd": "frontend-admin-ui",
            "command": "npm run dev",
            "npm_deps": True,
            "is_admin_only": True,
            "auto_start": True,
            "wait_time": 8,
            "tags": ["前端", "管理后台", "企业级"]
        }
    }
}


class ConfigManager:
    """配置管理器"""
    
    def __init__(self):
        self.config = self._load_config()
    
    def _load_config(self):
        if CONFIG_FILE.exists():
            try:
                with open(CONFIG_FILE, 'r', encoding='utf-8') as f:
                    saved = json.load(f)
                return self._merge_config(DEFAULT_CONFIG, saved)
            except Exception as e:
                print(f"⚠ 配置加载失败: {e}，使用默认配置")
        self._save_config(DEFAULT_CONFIG)
        return DEFAULT_CONFIG.copy()
    
    def _merge_config(self, base, override):
        import copy
        result = copy.deepcopy(base)
        for key, value in override.items():
            if key in result and isinstance(result[key], dict) and isinstance(value, dict):
                result[key] = self._merge_config(result[key], value)
            else:
                result[key] = value
        return result
    
    def _save_config(self, config):
        try:
            with open(CONFIG_FILE, 'w', encoding='utf-8') as f:
                json.dump(config, f, ensure_ascii=False, indent=2)
        except Exception as e:
            print(f"⚠ 配置保存失败: {e}")
    
    def get_service(self, key):
        return self.config['services'].get(key)
    
    def get_all_services(self):
        return self.config['services']
    
    def get_dashboard_port(self):
        return self.config.get('dashboard_port', 3040)
    
    def update_service(self, key, updates):
        if key in self.config['services']:
            self.config['services'][key].update(updates)
            self._save_config(self.config)


class ServiceManager:
    """服务管理器"""
    
    def __init__(self, config):
        self.config = config
        self.services = config.get_all_services()
        self.running_processes = {}
        self.lock = threading.Lock()
        self.logger = self._setup_logger()
    
    def _setup_logger(self):
        logger = logging.getLogger('PlatformManager')
        logger.setLevel(logging.INFO)
        log_file = LOG_DIR / 'platform_manager.log'
        if not logger.handlers:
            fh = logging.FileHandler(str(log_file), encoding='utf-8')
            fh.setLevel(logging.INFO)
            formatter = logging.Formatter('%(asctime)s - %(levelname)s - %(message)s')
            fh.setFormatter(formatter)
            logger.addHandler(fh)
        return logger
    
    def start_service(self, service_key):
        """启动服务"""
        svc = self.services.get(service_key)
        if not svc:
            return {'success': False, 'message': f'服务 {service_key} 不存在'}
        
        with self.lock:
            if self._is_running(service_key):
                return {'success': True, 'message': f'{svc["name"]} 已在运行', 'already_running': True}
            
            port = svc['port']
            if self._port_in_use(port):
                self.logger.warning(f"端口 {port} 被占用，尝试释放")
                self._free_port(port)
            
            cwd = PROJECT_ROOT / svc['cwd']
            if svc.get('npm_deps') and not (cwd / 'node_modules').exists():
                self.logger.info(f"安装 {svc['name']} 依赖...")
                print(f"  ⚠ 检测到依赖未安装，正在安装...")
                result = subprocess.run(['npm', 'install'], cwd=str(cwd), shell=sys.platform == 'win32', timeout=180)
                if result.returncode != 0:
                    return {'success': False, 'message': '依赖安装失败'}
                print(f"  ✓ 依赖安装完成")
            
            log_file = LOG_DIR / f'{service_key}.log'
            log_handle = open(str(log_file), 'a', encoding='utf-8')
            
            try:
                use_shell = sys.platform == 'win32'
                cmd = svc['command']
                
                creation_flags = 0
                if sys.platform == 'win32':
                    DETACHED_PROCESS = 0x00000008
                    CREATE_NEW_PROCESS_GROUP = 0x00000200
                    creation_flags = DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
                
                proc = subprocess.Popen(
                    cmd, cwd=str(cwd), shell=use_shell,
                    stdout=log_handle, stderr=log_handle, stdin=subprocess.DEVNULL,
                    creationflags=creation_flags
                )
                
                self.running_processes[service_key] = {
                    'proc': proc,
                    'started_at': time.time(),
                    'service_config': svc,
                    'log_file': str(log_file)
                }
                
                self.logger.info(f"启动 {svc['name']} (PID: {proc.pid})")
                
                wait_time = svc.get('wait_time', 5)
                ready = self._wait_ready(service_key, wait_time)
                
                if ready:
                    return {'success': True, 'message': f'{svc["name"]} 启动成功', 'pid': proc.pid}
                elif self._is_alive(proc.pid):
                    return {'success': True, 'message': f'{svc["name"]} 启动中', 'pid': proc.pid, 'starting': True}
                else:
                    del self.running_processes[service_key]
                    return {'success': False, 'message': f'{svc["name"]} 启动失败'}
                    
            except Exception as e:
                self.logger.error(f"启动失败: {e}")
                return {'success': False, 'message': str(e)}
    
    def stop_service(self, service_key, force=False):
        """停止服务"""
        with self.lock:
            if service_key not in self.running_processes:
                return {'success': True, 'message': '服务未运行'}
            
            info = self.running_processes[service_key]
            pid = info['proc'].pid
            svc = info['service_config']
            
            self.logger.info(f"停止 {svc['name']} (PID: {pid})")
            self._kill_tree(pid, force)
            
            for _ in range(20 if not force else 6):
                if not self._is_alive(pid):
                    break
                time.sleep(0.5)
            
            if self._is_alive(pid):
                self._kill_tree(pid, True)
                time.sleep(1)
            
            del self.running_processes[service_key]
            return {'success': True, 'message': f'{svc["name"]} 已停止'}
    
    def restart_service(self, service_key):
        """重启服务"""
        stop_result = self.stop_service(service_key)
        if not stop_result['success']:
            return stop_result
        time.sleep(1)
        return self.start_service(service_key)
    
    def start_all(self):
        """启动所有服务"""
        results = {}
        for key, svc in self.services.items():
            if svc.get('auto_start', True):
                results[key] = self.start_service(key)
                time.sleep(1)
        return results
    
    def stop_all(self, force=False):
        """停止所有服务"""
        results = {}
        for key in reversed(list(self.services.keys())):
            results[key] = self.stop_service(key, force)
            time.sleep(0.5)
        return results
    
    def restart_all(self):
        """重启所有服务"""
        self.stop_all()
        time.sleep(2)
        return self.start_all()
    
    def get_status(self, service_key):
        """获取服务状态"""
        svc = self.services.get(service_key)
        if not svc:
            return None
        
        port = svc['port']
        status = {
            'key': service_key,
            'name': svc['name'],
            'port': port,
            'description': svc['description'],
            'tags': svc.get('tags', []),
            'is_admin_only': svc.get('is_admin_only', False),
            'url': f'http://localhost:{port}',
            'status': 'stopped',
        }
        
        if service_key in self.running_processes:
            info = self.running_processes[service_key]
            proc = info['proc']
            pid = proc.pid
            
            if self._is_alive(pid):
                health_path = svc.get('health_check', '/')
                if self._check_http(port, health_path):
                    status['status'] = 'running'
                    status['pid'] = pid
                    status['uptime'] = int(time.time() - info['started_at'])
                elif self._port_in_use(port):
                    status['status'] = 'starting'
                    status['pid'] = pid
                else:
                    status['status'] = 'error'
                    status['pid'] = pid
            else:
                del self.running_processes[service_key]
        
        return status
    
    def get_all_status(self):
        """获取所有服务状态"""
        return {key: self.get_status(key) for key in self.services}
    
    def get_logs(self, service_key, lines=100):
        """获取服务日志"""
        if service_key in self.running_processes:
            log_file = self.running_processes[service_key]['log_file']
            try:
                with open(log_file, 'r', encoding='utf-8', errors='replace') as f:
                    all_lines = f.readlines()
                return ''.join(all_lines[-lines:])
            except Exception as e:
                return f"读取日志失败: {e}"
        return "服务未运行"
    
    # ============ 内部方法 ============
    
    def _is_running(self, key):
        if key not in self.running_processes:
            return False
        return self._is_alive(self.running_processes[key]['proc'].pid)
    
    def _port_in_use(self, port):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.settimeout(1)
            return s.connect_ex(('127.0.0.1', port)) == 0
    
    def _is_alive(self, pid):
        try:
            if sys.platform == 'win32':
                result = subprocess.run(['tasklist', '/FI', f'PID eq {pid}'], capture_output=True, timeout=3)
                return str(pid) in result.stdout.decode('gbk', errors='replace')
            else:
                os.kill(pid, 0)
                return True
        except:
            return False
    
    def _check_http(self, port, path='/', timeout=2):
        import urllib.request
        url = f'http://localhost:{port}{path}'
        try:
            urllib.request.urlopen(url, timeout=timeout)
            return True
        except:
            return False
    
    def _wait_ready(self, key, wait_time):
        svc = self.services[key]
        port = svc['port']
        health = svc.get('health_check', '/')
        
        for _ in range(wait_time * 2):
            time.sleep(0.5)
            if self._check_http(port, health):
                return True
            if key in self.running_processes:
                if not self._is_alive(self.running_processes[key]['proc'].pid):
                    return False
        return self._port_in_use(port)
    
    def _kill_tree(self, pid, force=False):
        try:
            if sys.platform == 'win32':
                cmd = ['taskkill', '/PID', str(pid), '/T']
                if force:
                    cmd.insert(2, '/F')
                CREATE_NO_WINDOW = 0x08000000
                subprocess.run(cmd, capture_output=True, timeout=5, creationflags=CREATE_NO_WINDOW)
            else:
                import signal as sig
                os.kill(pid, sig.SIGKILL if force else sig.SIGTERM)
        except Exception as e:
            self.logger.warning(f"终止进程失败: {e}")
    
    def _free_port(self, port):
        if sys.platform != 'win32':
            return
        try:
            result = subprocess.run(['netstat', '-ano', '-p', 'tcp'], capture_output=True, timeout=5)
            try:
                output = result.stdout.decode('gbk')
            except:
                output = result.stdout.decode('utf-8', errors='replace')
            
            for line in output.splitlines():
                if f':{port}' in line and 'LISTENING' in line:
                    parts = line.strip().split()
                    if len(parts) >= 5:
                        pid = int(parts[-1])
                        self._kill_tree(pid, True)
                        time.sleep(1)
        except Exception as e:
            self.logger.warning(f"释放端口失败: {e}")


class AuthManager:
    """认证管理器"""
    
    def __init__(self):
        self.sessions = {}
    
    def validate_password(self, username, password):
        return username == ADMIN_USERNAME and hash_password(password) == ADMIN_PASSWORD_HASH
    
    def create_session(self, username):
        sid = secrets.token_hex(32)
        self.sessions[sid] = {'user': username, 'created_at': time.time()}
        return sid
    
    def validate_session(self, sid):
        if sid and sid in self.sessions:
            s = self.sessions[sid]
            if time.time() - s['created_at'] < SESSION_TIMEOUT:
                return s
        return None
    
    def destroy_session(self, sid):
        if sid in self.sessions:
            del self.sessions[sid]
    
    def get_or_create_guest(self, request):
        cookie = request.headers.get('Cookie', '')
        sid = None
        for p in cookie.split(';'):
            p = p.strip()
            if p.startswith('session_id='):
                sid = p.split('=', 1)[1]
                break
        
        session = self.validate_session(sid) if sid else None
        if session:
            return sid, session
        
        new_sid = secrets.token_hex(32)
        self.sessions[new_sid] = {'user': 'guest', 'created_at': time.time()}
        return new_sid, self.sessions[new_sid]


class Dashboard:
    """Web 管理面板"""
    
    def __init__(self, svc_mgr, config):
        self.svc_mgr = svc_mgr
        self.config = config
        self.auth = AuthManager()
        self.server = None
        self.port = config.get_dashboard_port()
    
    def start(self):
        if self._port_used(self.port):
            print(f"⚠ 端口 {self.port} 已被占用")
            return False
        
        svc_mgr = self.svc_mgr
        auth = self.auth
        port = self.port
        admin_user = ADMIN_USERNAME
        cfg = self.config
        
        class Handler(BaseHTTPRequestHandler):
            def _resp(self, code, ct, body, sid=None):
                self.send_response(code)
                if sid:
                    self.send_header('Set-Cookie', f'session_id={sid}; Path=/; HttpOnly; SameSite=Lax')
                self.send_header('Content-Type', ct)
                self.end_headers()
                data = body.encode('utf-8') if isinstance(body, str) else body
                self.wfile.write(data)
            
            def _get_sid_session(self):
                return auth.get_or_create_guest(self)
            
            def _is_admin(self, sid):
                s = auth.validate_session(sid)
                return s and s.get('user') == 'admin'
            
            def do_GET(self):
                parsed = urlparse(self.path)
                path = parsed.path
                sid, session = self._get_sid_session()
                
                routes = {
                    '/login': lambda: self._resp(200, 'text/html; charset=utf-8', LOGIN_HTML, sid),
                    '/': lambda: self._resp(200, 'text/html; charset=utf-8', DASHBOARD_HTML, sid),
                    '/api/status': lambda: self._api_status(sid),
                    '/api/session': lambda: self._api_session(sid),
                    '/api/config': lambda: self._api_config(sid),
                }
                
                handler = routes.get(path)
                if handler:
                    handler()
                else:
                    self._resp(404, 'text/plain', 'Not Found', sid)
            
            def do_POST(self):
                parsed = urlparse(self.path)
                path = parsed.path
                sid, session = self._get_sid_session()
                
                cl = int(self.headers.get('Content-Length', 0))
                body = self.rfile.read(cl).decode('utf-8') if cl > 0 else '{}'
                try:
                    data = json.loads(body)
                except:
                    data = {}
                
                routes = {
                    '/api/login': lambda: self._api_login(data),
                    '/api/logout': lambda: self._api_logout(sid),
                    '/api/start': lambda: self._api_start(data, sid),
                    '/api/stop': lambda: self._api_stop(data, sid),
                    '/api/restart': lambda: self._api_restart(data, sid),
                    '/api/start_all': lambda: self._api_start_all(sid),
                    '/api/stop_all': lambda: self._api_stop_all(data, sid),
                    '/api/restart_all': lambda: self._api_restart_all(sid),
                    '/api/logs': lambda: self._api_logs(data, sid),
                }
                
                handler = routes.get(path)
                if handler:
                    handler()
                else:
                    self._resp(404, 'text/plain', 'Not Found', sid)
            
            # API 方法
            def _api_status(self, sid):
                status = svc_mgr.get_all_status()
                is_admin = self._is_admin(sid)
                filtered = []
                for k, s in status.items():
                    if s['is_admin_only'] and not is_admin:
                        s['description'] = '🔒 管理员权限才能查看详细信息'
                        s['tags'] = ['🔒 受限访问']
                        s['requires_auth'] = True
                    else:
                        s['requires_auth'] = False
                    filtered.append(s)
                self._resp(200, 'application/json; charset=utf-8',
                           json.dumps(filtered, ensure_ascii=False), sid)
            
            def _api_session(self, sid):
                is_adm = self._is_admin(sid)
                self._resp(200, 'application/json; charset=utf-8',
                           json.dumps({'user': 'admin' if is_adm else 'guest', 
                                      'is_admin': is_adm, 'username': admin_user}, ensure_ascii=False), sid)
            
            def _api_config(self, sid):
                if not self._is_admin(sid):
                    self._resp(403, 'application/json; charset=utf-8',
                               json.dumps({'error': '需要管理员权限'}), sid)
                    return
                self._resp(200, 'application/json; charset=utf-8',
                           json.dumps(cfg.config, ensure_ascii=False, indent=2), sid)
            
            def _api_login(self, data):
                username = data.get('username', '')
                password = data.get('password', '')
                if auth.validate_password(username, password):
                    new_sid = auth.create_session(username)
                    self._resp(200, 'application/json; charset=utf-8',
                               json.dumps({'success': True, 'user': username}), new_sid)
                else:
                    self._resp(401, 'application/json; charset=utf-8',
                               json.dumps({'success': False, 'message': '用户名或密码错误'}), sid)
            
            def _api_logout(self, sid):
                auth.destroy_session(sid)
                new_sid = auth.create_session('guest')
                self._resp(200, 'application/json; charset=utf-8',
                           json.dumps({'success': True}), new_sid)
            
            def _api_start(self, data, sid):
                if not self._is_admin(sid):
                    self._resp(403, 'application/json; charset=utf-8',
                               json.dumps({'error': '需要管理员权限'}), sid)
                    return
                result = svc_mgr.start_service(data.get('service'))
                self._resp(200, 'application/json; charset=utf-8',
                           json.dumps(result, ensure_ascii=False), sid)
            
            def _api_stop(self, data, sid):
                if not self._is_admin(sid):
                    self._resp(403, 'application/json; charset=utf-8',
                               json.dumps({'error': '需要管理员权限'}), sid)
                    return
                result = svc_mgr.stop_service(data.get('service'), data.get('force', False))
                self._resp(200, 'application/json; charset=utf-8',
                           json.dumps(result, ensure_ascii=False), sid)
            
            def _api_restart(self, data, sid):
                if not self._is_admin(sid):
                    self._resp(403, 'application/json; charset=utf-8',
                               json.dumps({'error': '需要管理员权限'}), sid)
                    return
                result = svc_mgr.restart_service(data.get('service'))
                self._resp(200, 'application/json; charset=utf-8',
                           json.dumps(result, ensure_ascii=False), sid)
            
            def _api_start_all(self, sid):
                if not self._is_admin(sid):
                    self._resp(403, 'application/json; charset=utf-8',
                               json.dumps({'error': '需要管理员权限'}), sid)
                    return
                results = svc_mgr.start_all()
                self._resp(200, 'application/json; charset=utf-8',
                           json.dumps({'success': True, 'results': results}, ensure_ascii=False), sid)
            
            def _api_stop_all(self, data, sid):
                if not self._is_admin(sid):
                    self._resp(403, 'application/json; charset=utf-8',
                               json.dumps({'error': '需要管理员权限'}), sid)
                    return
                results = svc_mgr.stop_all(data.get('force', False))
                self._resp(200, 'application/json; charset=utf-8',
                           json.dumps({'success': True, 'results': results}, ensure_ascii=False), sid)
            
            def _api_restart_all(self, sid):
                if not self._is_admin(sid):
                    self._resp(403, 'application/json; charset=utf-8',
                               json.dumps({'error': '需要管理员权限'}), sid)
                    return
                results = svc_mgr.restart_all()
                self._resp(200, 'application/json; charset=utf-8',
                           json.dumps({'success': True, 'results': results}, ensure_ascii=False), sid)
            
            def _api_logs(self, data, sid):
                if not self._is_admin(sid):
                    self._resp(403, 'application/json; charset=utf-8',
                               json.dumps({'error': '需要管理员权限'}), sid)
                    return
                logs = svc_mgr.get_logs(data.get('service'), data.get('lines', 100))
                self._resp(200, 'application/json; charset=utf-8',
                           json.dumps({'logs': logs}, ensure_ascii=False), sid)
            
            def log_message(self, format, *args):
                pass
        
        self.server = HTTPServer(('0.0.0.0', self.port), Handler)
        
        # 清理线程
        def cleanup():
            while True:
                time.sleep(60)
                self.auth.sessions = {
                    k: v for k, v in self.auth.sessions.items()
                    if time.time() - v['created_at'] < SESSION_TIMEOUT
                }
        threading.Thread(target=cleanup, daemon=True).start()
        return True
    
    def _port_used(self, port):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.settimeout(1)
            return s.connect_ex(('127.0.0.1', port)) == 0
    
    def serve(self):
        self.server.serve_forever()
    
    def shutdown(self):
        if self.server:
            self.server.shutdown()


# ============ HTML 模板 ============

LOGIN_HTML = '''<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>登录 - 璇玑系统管理平台</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI','PingFang SC','Microsoft YaHei',sans-serif;background:linear-gradient(135deg,#1a1a2e,#16213e,#0f3460);min-height:100vh;display:flex;align-items:center;justify-content:center;color:#ecf0f1}
.box{background:rgba(255,255,255,0.05);border:1px solid rgba(255,255,255,0.1);border-radius:24px;padding:48px 40px;width:100%;max-width:420px;backdrop-filter:blur(20px);box-shadow:0 25px 50px rgba(0,0,0,0.4)}
.brand{text-align:center;margin-bottom:40px}
.icon{font-size:56px;margin-bottom:12px}
.title{font-size:24px;font-weight:700;background:linear-gradient(135deg,#667eea,#764ba2);-webkit-background-clip:text;-webkit-text-fill-color:transparent}
.sub{font-size:13px;color:#95a5a6;margin-top:6px}
.group{margin-bottom:20px}
.label{display:block;font-size:13px;color:#bdc3c7;margin-bottom:8px}
.input{width:100%;padding:12px 16px;background:rgba(255,255,255,0.05);border:1px solid rgba(255,255,255,0.1);border-radius:10px;color:#ecf0f1;font-size:14px;outline:none}
.input:focus{border-color:#667eea;box-shadow:0 0 0 3px rgba(102,126,234,0.2)}
.btn{width:100%;padding:12px;background:linear-gradient(135deg,#667eea,#764ba2);border:none;border-radius:10px;color:white;font-size:15px;font-weight:600;cursor:pointer;margin-top:8px;transition:all .3s}
.btn:hover{transform:translateY(-2px);box-shadow:0 15px 35px rgba(102,126,234,0.4)}
.err{background:rgba(231,76,60,0.2);color:#e74c3c;padding:10px 14px;border-radius:8px;font-size:12px;margin-bottom:16px;display:none}
.err.show{display:block}
.sec{text-align:center;font-size:11px;color:#7f8c8d;margin-top:24px}
</style></head><body>
<div class="box">
<div class="brand"><div class="icon">🌌</div><div class="title">企业级服务管理平台</div><div class="sub">璇玑系统 Platform Manager v2.0</div></div>
<div class="err" id="err">用户名或密码错误</div>
<form onsubmit="login(event)">
<div class="group"><label class="label">管理员用户名</label><input class="input" type="text" id="u" placeholder="admin" required></div>
<div class="group"><label class="label">管理员密码</label><input class="input" type="password" id="p" placeholder="请输入密码" required></div>
<button type="submit" class="btn">🔐 登录管理平台</button>
</form>
<div class="sec">本平台需要管理员权限 · 会话有效期 30 分钟</div>
</div>
<script>
async function login(e){
e.preventDefault();
try{
const r=await fetch('/api/login',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({username:document.getElementById('u').value,password:document.getElementById('p').value})});
if(r.ok){const d=await r.json();if(d.success){location.href='/'}else{showErr(d.message)}}else{showErr('登录失败')}
}catch(e){showErr('网络错误')}
}
function showErr(m){const el=document.getElementById('err');el.textContent=m;el.classList.add('show');setTimeout(()=>el.classList.remove('show'),3000)}
document.getElementById('u').focus();
</script></body></html>'''

DASHBOARD_HTML = '''<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>璇玑系统 - 企业级服务管理平台</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI','PingFang SC','Microsoft YaHei',sans-serif;background:linear-gradient(135deg,#1a1a2e,#16213e,#0f3460);min-height:100vh;color:#ecf0f1}
.c{max-width:1400px;margin:0 auto;padding:28px 24px}
.hd{display:flex;justify-content:space-between;align-items:center;margin-bottom:24px}
.brand{display:flex;align-items:center;gap:12px}
.bi{font-size:36px}
.bt{font-size:20px;font-weight:700;background:linear-gradient(135deg,#667eea,#764ba2);-webkit-background-clip:text;-webkit-text-fill-color:transparent}
.bs{font-size:12px;color:#95a5a6}
.ub{display:flex;align-items:center;gap:16px;background:rgba(255,255,255,0.05);border:1px solid rgba(255,255,255,0.1);border-radius:12px;padding:10px 16px}
.ui{display:flex;align-items:center;gap:10px}
.ua{font-size:22px}
.un{font-size:13px;font-weight:600}
.ur{font-size:11px;color:#95a5a6}
.ra{color:#9b59b6}
.pb{background:linear-gradient(135deg,rgba(155,89,182,0.12),rgba(52,152,219,0.12));border:1px solid rgba(155,89,182,0.3);border-radius:12px;padding:14px 18px;display:flex;align-items:center;gap:12px;margin-bottom:24px;font-size:13px}
.st{display:grid;grid-template-columns:repeat(4,1fr);gap:16px;margin-bottom:24px}
.sc{background:rgba(255,255,255,0.05);border:1px solid rgba(255,255,255,0.1);border-radius:14px;padding:20px;text-align:center}
.si{font-size:28px;margin-bottom:6px}
.sv{font-size:32px;font-weight:700}
.sv.r{color:#2ecc71}
.sv.s{color:#e74c3c}
.sv.t{color:#3498db}
.sv.a{color:#9b59b6}
.sl{font-size:12px;color:#95a5a6;margin-top:4px}

/* Operations Bar */
.op-bar{background:rgba(255,255,255,0.03);border:1px solid rgba(255,255,255,0.08);border-radius:14px;padding:16px 20px;margin-bottom:24px;display:flex;align-items:center;gap:16px;flex-wrap:wrap}
.op-label{font-size:13px;font-weight:600;color:#bdc3c7;margin-right:4px}
.op-group{display:flex;gap:8px;flex-wrap:wrap;align-items:center}
.op-divider{width:1px;height:28px;background:rgba(255,255,255,0.1)}

.btn{padding:10px 18px;border:none;border-radius:8px;font-size:13px;font-weight:500;cursor:pointer;display:inline-flex;align-items:center;gap:6px;transition:all .2s;white-space:nowrap}
.btn-p{background:linear-gradient(135deg,#667eea,#764ba2);color:white}
.btn-s{background:linear-gradient(135deg,#11998e,#38ef7d);color:white}
.btn-d{background:linear-gradient(135deg,#eb3349,#f45c43);color:white}
.btn-w{background:linear-gradient(135deg,#f7971e,#ffd200);color:#333}
.btn-sec{background:rgba(255,255,255,0.1);color:#ecf0f1;border:1px solid rgba(255,255,255,0.2)}
.btn:hover{transform:translateY(-1px);filter:brightness(1.1)}
.btn:disabled{opacity:0.45;cursor:not-allowed;transform:none}
.btn .spinner{width:14px;height:14px;border:2px solid rgba(255,255,255,0.3);border-top-color:white;border-radius:50%;animation:spin .8s linear infinite}
@keyframes spin{to{transform:rotate(360deg)}}

/* Progress Panel */
.progress-panel{display:none;background:rgba(30,30,46,0.95);border:1px solid rgba(102,126,234,0.3);border-radius:14px;padding:18px 20px;margin-bottom:24px}
.progress-panel.show{display:block;animation:fadeIn .3s}
@keyframes fadeIn{from{opacity:0;transform:translateY(-10px)}to{opacity:1;transform:translateY(0)}}
.progress-header{display:flex;justify-content:space-between;align-items:center;margin-bottom:14px}
.progress-title{font-size:14px;font-weight:600;display:flex;align-items:center;gap:8px}
.progress-bar{height:6px;background:rgba(255,255,255,0.08);border-radius:3px;overflow:hidden;margin-bottom:14px}
.progress-fill{height:100%;background:linear-gradient(90deg,#667eea,#764ba2);border-radius:3px;transition:width .4s ease}
.progress-fill.done{background:linear-gradient(90deg,#11998e,#38ef7d)}
.progress-results{display:grid;grid-template-columns:repeat(auto-fill,minmax(220px,1fr));gap:8px}
.pr-item{display:flex;align-items:center;gap:8px;padding:8px 12px;background:rgba(255,255,255,0.03);border-radius:8px;font-size:12px}
.pr-icon{font-size:14px}
.pr-ok{color:#2ecc71}
.pr-fail{color:#e74c3c}
.pr-wait{color:#f1c40f}

/* Services Grid */
.sg{display:grid;grid-template-columns:repeat(auto-fill,minmax(360px,1fr));gap:18px}
.svc{background:rgba(255,255,255,0.05);border:1px solid rgba(255,255,255,0.1);border-radius:16px;padding:22px;position:relative;transition:all .3s}
.svc::before{content:'';position:absolute;top:0;left:0;right:0;height:3px;background:var(--c,#3498db);border-radius:16px 16px 0 0}
.svc:hover{border-color:rgba(255,255,255,0.2);transform:translateY(-2px)}
.svc.r{border-color:rgba(241,196,15,0.3)}
.svc.r::before{background:linear-gradient(90deg,#f39c12,#e67e22)}
.lock{position:absolute;top:12px;right:12px;background:rgba(241,196,15,0.2);color:#f1c40f;padding:3px 8px;border-radius:10px;font-size:10px;font-weight:500}
.sh{display:flex;justify-content:space-between;align-items:flex-start;margin-bottom:10px}
.si2{font-size:30px}
.ss{display:flex;align-items:center;gap:6px;padding:4px 10px;border-radius:12px;font-size:11px;font-weight:500}
.sd{width:6px;height:6px;border-radius:50%}
.sr{background:rgba(46,213,115,0.2);color:#2ed573}
.sr .sd{background:#2ed573;animation:pulse 1.5s infinite}
.ss2{background:rgba(231,76,60,0.2);color:#e74c3c}
.ss2 .sd{background:#e74c3c}
.sst{background:rgba(241,196,15,0.2);color:#f1c40f}
.sst .sd{background:#f1c40f;animation:pulse 1.5s infinite}
@keyframes pulse{0%,100%{opacity:1}50%{opacity:0.4}}
.sn{font-size:15px;font-weight:600;margin-bottom:5px}
.sdesc{font-size:12px;color:#95a5a6;line-height:1.5;margin-bottom:10px}
.sdesc.l{color:#f1c40f}
.sm{display:flex;gap:14px;font-size:11px;color:#7f8c8d;margin-bottom:10px;flex-wrap:wrap}
.sm span{display:flex;align-items:center;gap:4px}
.stags{display:flex;gap:5px;margin-bottom:12px;flex-wrap:wrap}
.tag{padding:2px 8px;background:rgba(255,255,255,0.08);border-radius:8px;font-size:10px;color:#bdc3c7}
.tag.l{background:rgba(241,196,15,0.2);color:#f1c40f}

/* Service Action Buttons */
.sa{display:grid;grid-template-columns:repeat(auto-fit,minmax(80px,1fr));gap:6px}
.ab{padding:7px 10px;border:none;border-radius:8px;font-size:11px;font-weight:500;cursor:pointer;transition:all .2s;display:flex;align-items:center;justify-content:center;gap:3px;text-decoration:none;text-align:center}
.as{background:linear-gradient(135deg,#11998e,#38ef7d);color:white}
.ap{background:linear-gradient(135deg,#eb3349,#f45c43);color:white}
.ar{background:linear-gradient(135deg,#f7971e,#ffd200);color:#333}
.ao{background:var(--c,#3498db);color:white}
.al{background:rgba(102,126,234,0.25);color:#8fa4f0}
.ab:hover{filter:brightness(1.1);transform:translateY(-1px)}
.ab:disabled{opacity:0.35;cursor:not-allowed;transform:none}
.ab .spinner{width:10px;height:10px;border:2px solid rgba(255,255,255,0.3);border-top-color:white;border-radius:50%;animation:spin .8s linear infinite}
.ab.danger{background:linear-gradient(135deg,#c0392b,#e74c3c)}

/* Modal */
.lm{display:none;position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(0,0,0,0.8);z-index:1000;align-items:center;justify-content:center}
.lm.show{display:flex}
.lc{background:#1a1a2e;border:1px solid rgba(255,255,255,0.1);border-radius:16px;padding:24px;width:90%;max-width:800px;max-height:80vh;display:flex;flex-direction:column}
.lh{display:flex;justify-content:space-between;align-items:center;margin-bottom:16px}
.lt{font-size:16px;font-weight:600}
.lx{background:none;border:none;color:#95a5a6;font-size:20px;cursor:pointer}
.lb{flex:1;overflow:auto;background:#0d0d1a;border-radius:8px;padding:16px;font-family:'Consolas',monospace;font-size:12px;line-height:1.6}

/* Confirm Dialog */
.cd{display:none;position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(0,0,0,0.85);z-index:1100;align-items:center;justify-content:center}
.cd.show{display:flex}
.cd-box{background:#1a1a2e;border:1px solid rgba(255,255,255,0.15);border-radius:16px;padding:28px;max-width:420px;width:90%}
.cd-icon{font-size:40px;text-align:center;margin-bottom:12px}
.cd-title{font-size:16px;font-weight:600;text-align:center;margin-bottom:8px}
.cd-msg{font-size:13px;color:#bdc3c7;text-align:center;margin-bottom:20px;line-height:1.6}
.cd-actions{display:flex;gap:10px;justify-content:center}
.cd-cancel{padding:10px 20px;background:rgba(255,255,255,0.1);border:1px solid rgba(255,255,255,0.2);border-radius:8px;color:#ecf0f1;font-size:13px;cursor:pointer}
.cd-confirm{padding:10px 20px;background:linear-gradient(135deg,#eb3349,#f45c43);border:none;border-radius:8px;color:white;font-size:13px;cursor:pointer}

/* Toast */
.tst{position:fixed;bottom:24px;right:24px;background:rgba(30,30,46,0.95);border:1px solid rgba(255,255,255,0.1);border-radius:10px;padding:12px 20px;font-size:13px;z-index:2000;display:none}
.tst.show{display:block;animation:slideIn .3s}
.tst.s{border-color:rgba(46,213,115,0.5);color:#2ed573}
.tst.e{border-color:rgba(231,76,60,0.5);color:#e74c3c}
.tst.i{border-color:rgba(102,126,234,0.5);color:#667eea}
@keyframes slideIn{from{transform:translateX(100%)}to{transform:translateX(0)}}
.ft{text-align:center;padding:24px;color:#7f8c8d;font-size:11px}

@media(max-width:768px){
.op-bar{flex-direction:column;align-items:stretch}
.op-divider{display:none}
.st{grid-template-columns:repeat(2,1fr)}
}
</style></head><body>
<div class="c">
<div class="hd">
<div class="brand"><span class="bi">🌌</span><div><div class="bt">璇玑系统管理平台</div><div class="bs">Enterprise Service Manager v2.0</div></div></div>
<div id="ub" class="ub"></div>
</div>

<div id="pb" class="pb" style="display:none"></div>

<div id="st" class="st"></div>

<!-- Operations Bar -->
<div class="op-bar" id="op-bar">
<span class="op-label">⚡ 操作面板</span>
<div class="op-group">
<button class="btn btn-s" onclick="confirmBatch('start_all')" id="btn-start"><span>▶</span> 启动所有</button>
<button class="btn btn-w" onclick="confirmBatch('restart_all')" id="btn-restart"><span>🔄</span> 重启所有</button>
<button class="btn btn-d" onclick="confirmBatch('stop_all')" id="btn-stop"><span>⏹</span> 停止所有</button>
</div>
<div class="op-divider"></div>
<div class="op-group">
<button class="btn btn-sec" onclick="refresh()"><span>🔄</span> 刷新状态</button>
<button class="btn btn-sec" onclick="showConfig()"><span>⚙️</span> 配置管理</button>
</div>
</div>

<!-- Progress Panel -->
<div class="progress-panel" id="progress">
<div class="progress-header">
<span class="progress-title" id="prog-title">⚡ 正在执行操作...</span>
<span style="font-size:12px;color:#7f8c8d" id="prog-count"></span>
</div>
<div class="progress-bar"><div class="progress-fill" id="prog-fill" style="width:0%"></div></div>
<div class="progress-results" id="prog-results"></div>
</div>

<!-- Services Grid -->
<div id="sg" class="sg"></div>

<!-- Logs Modal -->
<div id="lm" class="lm"><div class="lc"><div class="lh"><span id="lt" class="lt">服务日志</span><button class="lx" onclick="closeLogs()">✕</button></div><div id="lb" class="lb">加载中...</div></div></div>

<!-- Confirm Dialog -->
<div id="cd" class="cd"><div class="cd-box">
<div class="cd-icon" id="cd-icon">⚠️</div>
<div class="cd-title" id="cd-title">确认操作</div>
<div class="cd-msg" id="cd-msg">此操作将影响所有服务，是否继续？</div>
<div class="cd-actions">
<button class="cd-cancel" onclick="closeConfirm()">取消</button>
<button class="cd-confirm" id="cd-confirm">确认执行</button>
</div>
</div></div>

<!-- Toast -->
<div id="tst" class="tst"></div>

<div class="ft"><p>璇玑系统 © 2026 | 企业级服务管理平台</p></div>
</div>

<script>
let session=null,services=[],timer=null;
const COLORS={api:'#3498db',frontend:'#2ecc71',admin:'#9b59b6'};
const ICONS={api:'🔧',frontend:'🎨',admin:'⚙️'};
const ADMIN_OPS=['start','stop','restart','logs'];

// ============ Init ============
async function init(){await loadSession();await refresh();startAuto()}
async function loadSession(){try{const r=await fetch('/api/session');session=await r.json()}catch(e){session={user:'guest',is_admin:false}}renderUser();updateOpBar()}
function renderUser(){const ub=document.getElementById('ub');if(session.is_admin){ub.innerHTML='<div class="ui"><span class="ua">🛡️</span><div><div class="un">'+session.username+'管理员</div><div class="ur ra">● 系统管理员</div></div></div><button class="btn btn-sec" onclick="logout()" style="padding:6px 12px;font-size:12px">退出</button>';document.getElementById('pb').style.display='none'}else{ub.innerHTML='<div class="ui"><span class="ua">👤</span><div><div class="un">访客用户</div><div class="ur">普通用户 · 部分功能受限</div></div></div><button class="btn btn-p" onclick="location.href=\'/login\'" style="padding:6px 16px;font-size:12px">🔐 管理员登录</button>';const pb=document.getElementById('pb');pb.style.display='flex';pb.innerHTML='🔒 <span>您当前以 <strong>普通用户</strong> 身份访问，启动/停止服务等操作需要管理员权限。点击右上角登录获取完整权限。</span>'}}
function updateOpBar(){const bar=document.getElementById('op-bar');if(!session.is_admin){bar.querySelectorAll('.btn').forEach(b=>{if(b.textContent.includes('启动')||b.textContent.includes('停止')||b.textContent.includes('重启')){b.disabled=true}})}}

// ============ Refresh ============
async function refresh(){try{const r=await fetch('/api/status');services=await r.json();renderStats();renderServices()}catch(e){console.error(e)}}
function renderStats(){const run=services.filter(s=>s.status==='running').length,sto=services.filter(s=>s.status==='stopped').length,res=services.filter(s=>s.is_admin_only).length;document.getElementById('st').innerHTML='<div class="sc"><div class="si">📊</div><div class="sv t">'+services.length+'</div><div class="sl">服务总数</div></div><div class="sc"><div class="si">✅</div><div class="sv r">'+run+'</div><div class="sl">运行中</div></div><div class="sc"><div class="si">⏹</div><div class="sv s">'+sto+'</div><div class="sl">已停止</div></div><div class="sc"><div class="si">🔒</div><div class="sv a">'+res+'</div><div class="sl">需管理员权限</div></div>'}
function renderServices(){const g=document.getElementById('sg');g.innerHTML='';services.forEach(s=>g.appendChild(renderCard(s)))}

function renderCard(s){
const adm=session.is_admin;
const r=s.status==='running';
const locked=s.requires_auth&&!adm;
const sc='status-'+s.status;
const stext=r?'运行中':s.status==='starting'?'启动中':'已停止';

const c=document.createElement('div');
c.className='svc'+(locked?' r':'');
c.style.setProperty('--c',COLORS[s.key]||'#3498db');

const tags=(s.tags||[]).map(t=>'<span class="tag'+(t.includes('受限')?' l':'')+'">'+t+'</span>').join('');

let actions='';
if(locked){
actions='<a href="/login" class="ab as">🔐 登录访问</a>';
}else if(adm){
if(r){
actions='<button class="ab ar" data-action="restart" data-svc="'+s.key+'">🔄 重启</button>';
actions+='<button class="ab ap" data-action="stop" data-svc="'+s.key+'">⏹ 停止</button>';
}else{
actions='<button class="ab as" data-action="start" data-svc="'+s.key+'">▶ 启动</button>';
actions+='<button class="ab ar" data-action="restart" data-svc="'+s.key+'">🔄 重启</button>';
}
actions+='<button class="ab al" data-action="logs" data-svc="'+s.key+'" '+(r?'':'disabled')+'>📋 日志</button>';
if(r){actions+='<a href="'+s.url+'" target="_blank" class="ab ao">🚀 访问</a>';}
}else{
if(r){actions='<a href="'+s.url+'" target="_blank" class="ab ao">🚀 访问</a>';}
else{actions='<div class="ab" style="background:rgba(255,255,255,0.05);color:#7f8c8d">⏸ 服务未运行</div>';}
}

c.innerHTML=
(locked?'<div class="lock">🔒 需管理员权限</div>':'')+
'<div class="sh"><span class="si2">'+(ICONS[s.key]||'📦')+'</span><div class="ss '+sc+'"><span class="sd"></span>'+stext+'</div></div>'+
'<div class="sn">'+s.name+'</div>'+
'<div class="sdesc '+(locked?'l':'')+'">'+s.description+'</div>'+
'<div class="sm">'+
'<span>📡 端口 :'+s.port+'</span>'+
(s.pid?'<span>🆔 PID '+s.pid+'</span>':'')+
(s.uptime?'<span>⏱ 运行 '+Math.floor(s.uptime/60)+'分</span>':'')+
'</div>'+
'<div class="stags">'+tags+'</div>'+
'<div class="sa">'+actions+'</div>';
return c;
}

// Event delegation for service actions
document.addEventListener('click',function(e){
const btn=e.target.closest('[data-action]');
if(!btn)return;
const action=btn.dataset.action;
const svc=btn.dataset.svc;
if(!action||!svc)return;

if(action==='logs'){showLogs(svc);return}
act(action,svc,btn);
});

// ============ Single Service Action ============
async function act(action,svc,btn){
if(!session.is_admin){toast('需要管理员权限','error');return}
const map={start:['/api/start','启动'],stop:['/api/stop','停止'],restart:['/api/restart','重启']};
const[endpoint,label]=map[action];
try{
if(btn){btn.disabled=true;btn.innerHTML='<span class="spinner"></span> '+label+'中...';}
toast('正在'+label+' '+svc+'...','info');
const r=await fetch(endpoint,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({service:svc})});
const d=await r.json();
if(d.success||(action==='stop'&&d.message&&d.message.includes('未运行'))){
toast(d.message||label+'完成','success');
}else{
toast(d.message||'操作失败','error');
}
refresh();
}catch(e){
toast('请求失败','error');
}finally{
if(btn){btn.disabled=false;btn.innerHTML=btn.innerHTML.replace(/<span class="spinner"><\/span> /,'');}
}
}

// ============ Batch Operations ============
async function confirmBatch(action){
const configs={
start_all:{icon:'🚀',title:'启动所有服务',msg:'将启动所有已配置的服务，可能需要等待几秒时间。',confirm:'开始启动'},
stop_all:{icon:'⏹',title:'停止所有服务',msg:'⚠️ 这将停止所有正在运行的服务，正在进行的请求可能会被中断。',confirm:'确认停止',danger:true},
restart_all:{icon:'🔄',title:'重启所有服务',msg:'将依次停止并重新启动所有服务，会有短暂的服务不可用。',confirm:'确认重启',danger:true}
};
const cfg=configs[action];
const cd=document.getElementById('cd');
document.getElementById('cd-icon').textContent=cfg.icon;
document.getElementById('cd-title').textContent=cfg.title;
document.getElementById('cd-msg').textContent=cfg.msg;
const confirmBtn=document.getElementById('cd-confirm');
confirmBtn.textContent=cfg.confirm;
confirmBtn.className='cd-confirm'+(cfg.danger?'':'');
confirmBtn.style.background=cfg.danger?'linear-gradient(135deg,#eb3349,#f45c43)':'linear-gradient(135deg,#667eea,#764ba2)';
confirmBtn.onclick=()=>{closeConfirm();doBatch(action)};
cd.classList.add('show');
}

function closeConfirm(){document.getElementById('cd').classList.remove('show')}

async function doBatch(action){
if(!session.is_admin){toast('需要管理员权限','error');return}
const map={start_all:['/api/start_all','启动所有服务'],stop_all:['/api/stop_all','停止所有服务'],restart_all:['/api/restart_all','重启所有服务']};
const[endpoint,label]=map[action];

// Show progress panel
const prog=document.getElementById('progress');
prog.classList.add('show');
document.getElementById('prog-title').textContent='⚡ '+label+'中...';
document.getElementById('prog-fill').style.width='10%';
document.getElementById('prog-fill').classList.remove('done');
document.getElementById('prog-results').innerHTML='';

try{
const r=await fetch(endpoint,{method:'POST',headers:{'Content-Type':'application/json'}});
const d=await r.json();
const results=d.results||{};
const total=Object.keys(results).length;
let done=0,success=0,fail=0;

document.getElementById('prog-title').textContent='✅ '+label+'完成';
document.getElementById('prog-fill').classList.add('done');
document.getElementById('prog-fill').style.width='100%';

const resultsDiv=document.getElementById('prog-results');
resultsDiv.innerHTML='';
for(const[key,res] of Object.entries(results)){
const item=document.createElement('div');
item.className='pr-item '+(res.success?'pr-ok':'pr-fail');
item.innerHTML='<span class="pr-icon">'+(res.success?'✅':'❌')+'</span><span>'+key+': '+(res.message||'')+'</span>';
resultsDiv.appendChild(item);
if(res.success)success++;else fail++;
}

document.getElementById('prog-count').textContent=success+' 成功 / '+fail+' 失败';
toast(label+'完成: '+success+'成功, '+fail+'失败',fail>0?'error':'success');

setTimeout(()=>prog.classList.remove('show'),6000);
refresh();
}catch(e){
toast('请求失败','error');
prog.classList.remove('show');
}
}

// ============ Config ============
async function showConfig(){
if(!session.is_admin){toast('需要管理员权限','error');return}
try{
const r=await fetch('/api/config');
const d=await r.json();
alert('服务配置:\\n\\n'+JSON.stringify(d,null,2));
}catch(e){toast('加载配置失败','error')}
}

// ============ Other ============
async function logout(){try{await fetch('/api/logout',{method:'POST',headers:{'Content-Type':'application/json'}});session={user:'guest',is_admin:false};renderUser();refresh()}catch(e){}}
async function showLogs(svc){try{document.getElementById('lt').textContent=svc+' 日志';document.getElementById('lm').classList.add('show');const r=await fetch('/api/logs',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({service:svc,lines:200})});const d=await r.json();document.getElementById('lb').textContent=d.logs||'暂无日志'}catch(e){document.getElementById('lb').textContent='加载日志失败'}}
function closeLogs(){document.getElementById('lm').classList.remove('show')}
function toast(msg,type){const t=document.getElementById('tst');t.textContent=msg;t.className='tst show '+(type||'');setTimeout(()=>t.classList.remove('show'),3000)}
function startAuto(){timer=setInterval(refresh,5000)}
init();
</script></body></html>'''


# ============ CLI 入口 ============

def print_header():
    print("\n" + "=" * 64)
    print("  🌌 璇玑系统 - 企业级服务管理平台 v2.0")
    print("=" * 64 + "\n")


def print_service_table(status_dict):
    print(f"{'服务名称':<20} {'状态':<10} {'端口':<8} {'PID':<8} {'运行时长':<12} {'权限':<10}")
    print("-" * 72)
    for key, s in status_dict.items():
        status_icon = {'running': '🟢 运行中', 'stopped': '🔴 已停止', 'starting': '🟡 启动中'}.get(s['status'], '⚪ 未知')
        pid = str(s.get('pid', '-'))
        uptime = f"{int(s.get('uptime', 0) / 60)}分钟" if s.get('uptime') else '-'
        perm = '🔒 管理员' if s.get('is_admin_only') else '👁️ 公开'
        print(f"{s['name']:<20} {status_icon:<12} {s['port']:<8} {pid:<8} {uptime:<12} {perm}")


def main():
    config_mgr = ConfigManager()
    svc_mgr = ServiceManager(config_mgr)
    
    if len(sys.argv) < 2:
        print_header()
        print("使用方法:")
        print("  python platform_manager.py start [service]     # 启动服务")
        print("  python platform_manager.py stop [service]      # 停止服务")
        print("  python platform_manager.py restart [service]   # 重启服务")
        print("  python platform_manager.py status             # 查看状态")
        print("  python platform_manager.py logs [service]     # 查看日志")
        print("  python platform_manager.py dashboard          # 启动管理面板")
        print("  python platform_manager.py cleanup            # 清理所有服务")
        print("\n服务列表:")
        for key, svc in config_mgr.get_all_services().items():
            print(f"  {key:<12} {svc['name']} (端口: {svc['port']})")
        return
    
    action = sys.argv[1].lower()
    target = sys.argv[2] if len(sys.argv) > 2 else None
    
    if action == 'dashboard':
        port = config_mgr.get_dashboard_port()
        dashboard = Dashboard(svc_mgr, config_mgr)
        
        print(f"\n🌐 启动 Web 管理面板...")
        if dashboard.start():
            print(f"  ✅ 管理面板已启动: http://localhost:{port}")
            print(f"  👤 管理员账户: {ADMIN_USERNAME} / {ADMIN_PASSWORD}")
            print(f"  💡 浏览器访问以上地址进行管理\n")
            try:
                import webbrowser
                webbrowser.open(f'http://localhost:{port}')
            except:
                pass
            dashboard.serve()
        else:
            print(f"  ❌ 启动失败，请检查端口 {port} 是否被占用")
    
    elif action == 'start':
        print_header()
        if target:
            result = svc_mgr.start_service(target)
            print(f"  {result['message']}")
            if result.get('success') and result.get('pid'):
                print(f"  PID: {result['pid']}")
        else:
            results = svc_mgr.start_all()
            for key, r in results.items():
                icon = '✅' if r['success'] else '❌'
                print(f"  {icon} {r['message']}")
    
    elif action == 'stop':
        print_header()
        if target:
            result = svc_mgr.stop_service(target)
            print(f"  {result['message']}")
        else:
            results = svc_mgr.stop_all()
            for key, r in results.items():
                print(f"  ✅ {r['message']}")
    
    elif action == 'restart':
        print_header()
        if target:
            result = svc_mgr.restart_service(target)
            icon = '✅' if result['success'] else '❌'
            print(f"  {icon} {result['message']}")
        else:
            results = svc_mgr.restart_all()
            for key, r in results.items():
                icon = '✅' if r['success'] else '❌'
                print(f"  {icon} {r['message']}")
    
    elif action == 'status':
        print_header()
        status = svc_mgr.get_all_status()
        print_service_table(status)
    
    elif action == 'logs':
        print_header()
        if target:
            logs = svc_mgr.get_logs(target, 50)
            print(f"📋 {target} 最近日志:\n")
            print(logs)
        else:
            for key in config_mgr.get_all_services():
                print(f"\n{'='*50}")
                print(f"  📋 {key} 日志")
                print('='*50)
                print(svc_mgr.get_logs(key, 20))
    
    elif action == 'cleanup':
        print_header()
        print("  正在清理所有服务...")
        svc_mgr.stop_all(force=True)
        print("  ✅ 已清理所有服务")
    
    else:
        print(f"❌ 未知操作: {action}")
        print("可用操作: start, stop, restart, status, logs, dashboard, cleanup")


if __name__ == '__main__':
    main()