#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
璇玑系统 - 服务管理面板 (带权限控制)
============================================
一个 Web 服务，用于展示所有服务状态和访问入口
- 管理员需要登录后才能查看完整信息
- 普通用户只能查看公开信息
启动后访问: http://localhost:9999

默认管理员账户: admin / admin123
可通过环境变量 MONITOR_ADMIN_USER 和 MONITOR_ADMIN_PASS 修改
"""

import os
import sys
import json
import socket
import hashlib
import secrets
import time
import webbrowser
import threading
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path
from datetime import datetime, timedelta
from urllib.parse import parse_qs, urlparse

# ============ 配置 ============
PROJECT_ROOT = Path(__file__).parent.resolve()
PID_DIR = PROJECT_ROOT / '.runtime'
MONITOR_PORT = 9999

# 管理员配置（可通过环境变量覆盖）
ADMIN_USERNAME = os.environ.get('MONITOR_ADMIN_USER', 'admin')
ADMIN_PASSWORD_HASH = os.environ.get(
    'MONITOR_ADMIN_PASS_HASH',
    hashlib.sha256('admin123'.encode()).hexdigest()
)

# 会话配置
SESSION_TIMEOUT = 30 * 60  # 30分钟过期
sessions = {}  # {session_id: {'user': 'admin', 'created_at': timestamp}}

# 服务配置（与 service_manager.py 保持一致）
SERVICES = {
    'api': {
        'name': 'API 后端服务',
        'description': '璇玑系统核心 API 服务，提供图谱分析、AI 对话、专家联盟等能力',
        'port': 3002,
        'path': '/health',
        'icon': '🔧',
        'color': '#3498db',
        'tags': ['API', '后端', '核心'],
        'is_admin_only': True,  # 管理员才能查看详细信息
    },
    'frontend': {
        'name': '用户前端界面',
        'description': '面向终端用户的操作界面，包含图谱可视化、AI 对话、专家咨询等功能',
        'port': 5173,
        'path': '/',
        'icon': '🎨',
        'color': '#2ecc71',
        'tags': ['前端', '用户界面', 'Vite'],
        'is_admin_only': False,
    },
    'frontend2': {
        'name': '用户前端 (5174)',
        'description': '备用用户前端实例，运行于端口 5174，功能与 5173 保持一致',
        'port': 5174,
        'path': '/',
        'icon': '🎨',
        'color': '#1abc9c',
        'tags': ['前端', '备用实例', 'Vite'],
        'is_admin_only': False,
    },
    'admin': {
        'name': '企业管理界面',
        'description': '企业级后台管理系统，用于用户管理、权限配置、知识库管理等',
        'port': 5175,
        'path': '/',
        'icon': '⚙️',
        'color': '#9b59b6',
        'tags': ['前端', '管理后台', '企业级'],
        'is_admin_only': True,  # 管理员才能查看详细信息
    },
}

# ============ 权限工具函数 ============

def generate_session_id():
    """生成安全的会话 ID"""
    return secrets.token_hex(32)

def validate_password(username, password):
    """验证管理员密码"""
    if username != ADMIN_USERNAME:
        return False
    password_hash = hashlib.sha256(password.encode()).hexdigest()
    return password_hash == ADMIN_PASSWORD_HASH

def get_or_create_session(request):
    """获取或创建会话"""
    # 从 Cookie 获取现有会话
    cookie_header = request.headers.get('Cookie', '')
    session_id = None
    
    for part in cookie_header.split(';'):
        part = part.strip()
        if part.startswith('session_id='):
            session_id = part.split('=', 1)[1]
            break
    
    # 检查会话是否有效
    if session_id and session_id in sessions:
        session = sessions[session_id]
        if time.time() - session['created_at'] < SESSION_TIMEOUT:
            return session_id, session
    
    # 创建新会话
    new_session_id = generate_session_id()
    sessions[new_session_id] = {
        'user': 'guest',
        'created_at': time.time()
    }
    return new_session_id, sessions[new_session_id]

def is_admin(session):
    """检查是否为管理员"""
    return session.get('user') == 'admin'

def cleanup_expired_sessions():
    """清理过期会话"""
    current_time = time.time()
    expired = [sid for sid, s in sessions.items() 
               if current_time - s['created_at'] > SESSION_TIMEOUT]
    for sid in expired:
        del sessions[sid]

# ============ 服务状态检测 ============

def is_port_in_use(port):
    """检查端口是否被占用"""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.settimeout(1)
        result = sock.connect_ex(('127.0.0.1', port))
        return result == 0

def check_http_service(port, path='/', timeout=2):
    """检查 HTTP 服务是否可访问"""
    import urllib.request
    url = f'http://localhost:{port}{path}'
    try:
        req = urllib.request.Request(url, method='HEAD')
        urllib.request.urlopen(req, timeout=timeout)
        return True
    except:
        try:
            urllib.request.urlopen(url, timeout=timeout)
            return True
        except:
            return False

def get_service_status(service_key):
    """获取服务状态"""
    service = SERVICES[service_key]
    port = service['port']
    
    if not is_port_in_use(port):
        return 'stopped'
    
    if check_http_service(port, service['path']):
        return 'running'
    
    if check_http_service(port, '/'):
        return 'running'
    
    return 'starting'

def get_all_service_status(is_admin_user=False):
    """获取所有服务状态（根据权限过滤）"""
    status = {}
    for key, service in SERVICES.items():
        service_status = {
            'key': key,
            'name': service['name'],
            'port': service['port'],
            'icon': service['icon'],
            'color': service['color'],
            'status': get_service_status(key),
            'url': f"http://localhost:{service['port']}",
            'is_admin_only': service['is_admin_only'],
        }
        
        # 非管理员用户：隐藏敏感服务的详细描述和标签
        if service['is_admin_only'] and not is_admin_user:
            service_status['description'] = '🔒 管理员权限才能查看详细信息'
            service_status['tags'] = ['🔒 受限访问']
            service_status['requires_auth'] = True
        else:
            service_status['description'] = service['description']
            service_status['tags'] = service['tags']
            service_status['requires_auth'] = False
        
        status[key] = service_status
    return status

# ============ HTML 模板 ============

LOGIN_HTML = '''<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>璇玑系统 - 管理员登录</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC', 'Microsoft YaHei', sans-serif;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            color: #ecf0f1;
        }
        .login-container {
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 24px;
            padding: 48px 40px;
            width: 100%;
            max-width: 400px;
            backdrop-filter: blur(20px);
            box-shadow: 0 25px 50px rgba(0, 0, 0, 0.4);
        }
        .logo-section {
            text-align: center;
            margin-bottom: 40px;
        }
        .logo-icon {
            font-size: 64px;
            margin-bottom: 16px;
        }
        .logo-title {
            font-size: 28px;
            font-weight: 700;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
            margin-bottom: 8px;
        }
        .logo-subtitle {
            font-size: 13px;
            color: #95a5a6;
        }
        .form-group {
            margin-bottom: 24px;
        }
        .form-label {
            display: block;
            font-size: 13px;
            color: #bdc3c7;
            margin-bottom: 8px;
        }
        .form-input {
            width: 100%;
            padding: 14px 16px;
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 10px;
            color: #ecf0f1;
            font-size: 15px;
            transition: all 0.3s ease;
            outline: none;
        }
        .form-input:focus {
            border-color: #667eea;
            box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.2);
        }
        .form-input::placeholder {
            color: #7f8c8d;
        }
        .btn-login {
            width: 100%;
            padding: 14px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            border: none;
            border-radius: 10px;
            color: white;
            font-size: 16px;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.3s ease;
            margin-top: 10px;
        }
        .btn-login:hover {
            transform: translateY(-2px);
            box-shadow: 0 15px 35px rgba(102, 126, 234, 0.4);
        }
        .error-message {
            background: rgba(231, 76, 60, 0.2);
            color: #e74c3c;
            padding: 12px 16px;
            border-radius: 8px;
            font-size: 13px;
            margin-bottom: 20px;
            display: none;
        }
        .error-message.show {
            display: block;
        }
        .security-notice {
            text-align: center;
            font-size: 11px;
            color: #7f8c8d;
            margin-top: 30px;
            line-height: 1.6;
        }
    </style>
</head>
<body>
    <div class="login-container">
        <div class="logo-section">
            <div class="logo-icon">🌐</div>
            <h1 class="logo-title">管理员登录</h1>
            <p class="logo-subtitle">璇玑系统服务管理面板</p>
        </div>
        
        <div class="error-message" id="error-message">用户名或密码错误</div>
        
        <form id="login-form" onsubmit="handleLogin(event)">
            <div class="form-group">
                <label class="form-label">用户名</label>
                <input type="text" class="form-input" id="username" placeholder="请输入管理员用户名" required>
            </div>
            <div class="form-group">
                <label class="form-label">密码</label>
                <input type="password" class="form-input" id="password" placeholder="请输入密码" required>
            </div>
            <button type="submit" class="btn-login">🔐 登录</button>
        </form>
        
        <div class="security-notice">
            本面板需要管理员权限访问<br>
            登录状态将保持 30 分钟
        </div>
    </div>
    
    <script>
        async function handleLogin(event) {
            event.preventDefault();
            const username = document.getElementById('username').value;
            const password = document.getElementById('password').value;
            
            try {
                const response = await fetch('/api/login', {
                    method: 'POST',
                    headers: {'Content-Type': 'application/json'},
                    body: JSON.stringify({username, password})
                });
                
                if (response.ok) {
                    const data = await response.json();
                    if (data.success) {
                        window.location.href = '/';
                    } else {
                        showError(data.message || '登录失败');
                    }
                } else {
                    showError('登录失败，请重试');
                }
            } catch (e) {
                showError('网络错误，请重试');
            }
        }
        
        function showError(message) {
            const errorDiv = document.getElementById('error-message');
            errorDiv.textContent = message;
            errorDiv.classList.add('show');
            setTimeout(() => errorDiv.classList.remove('show'), 3000);
        }
        
        // 自动聚焦用户名
        document.getElementById('username').focus();
    </script>
</body>
</html>'''

MAIN_HTML = '''<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>璇玑系统 - 服务管理面板</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC', 'Microsoft YaHei', sans-serif;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
            min-height: 100vh;
            color: #ecf0f1;
        }
        .container { max-width: 1200px; margin: 0 auto; padding: 40px 20px; }
        .header { text-align: center; margin-bottom: 40px; position: relative; }
        .logo { font-size: 48px; margin-bottom: 10px; }
        .title {
            font-size: 32px; font-weight: 700;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            -webkit-background-clip: text; -webkit-text-fill-color: transparent;
            background-clip: text; margin-bottom: 8px;
        }
        .subtitle { font-size: 14px; color: #95a5a6; margin-bottom: 20px; }
        
        /* 用户信息条 */
        .user-bar {
            display: flex; justify-content: space-between; align-items: center;
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 12px;
            padding: 14px 20px;
            margin-bottom: 30px;
        }
        .user-info { display: flex; align-items: center; gap: 12px; }
        .user-avatar { font-size: 28px; }
        .user-details .user-name { font-weight: 600; font-size: 15px; }
        .user-details .user-role { font-size: 12px; color: #95a5a6; }
        .role-badge {
            display: inline-block; padding: 3px 10px; border-radius: 12px;
            font-size: 11px; font-weight: 500; margin-left: 8px;
        }
        .role-admin { background: rgba(155, 89, 182, 0.2); color: #9b59b6; }
        .role-guest { background: rgba(149, 165, 166, 0.2); color: #95a5a6; }
        .btn-logout {
            padding: 8px 16px; background: rgba(231, 76, 60, 0.2);
            border: 1px solid rgba(231, 76, 60, 0.3);
            border-radius: 8px; color: #e74c3c;
            font-size: 13px; cursor: pointer; transition: all 0.3s ease;
        }
        .btn-logout:hover { background: rgba(231, 76, 60, 0.3); }
        
        .stats { display: flex; justify-content: center; gap: 30px; margin-bottom: 40px; }
        .stat-card {
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 12px; padding: 20px 30px;
            text-align: center; backdrop-filter: blur(10px);
        }
        .stat-value { font-size: 36px; font-weight: 700; margin-bottom: 5px; }
        .stat-value.running { color: #2ecc71; }
        .stat-value.stopped { color: #e74c3c; }
        .stat-value.total { color: #3498db; }
        .stat-label { font-size: 12px; color: #95a5a6; }
        
        .actions { display: flex; justify-content: center; gap: 15px; margin-bottom: 40px; flex-wrap: wrap; }
        .btn {
            padding: 12px 24px; border: none; border-radius: 8px;
            font-size: 14px; font-weight: 500; cursor: pointer;
            transition: all 0.3s ease; display: flex; align-items: center; gap: 8px;
        }
        .btn-primary {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white;
        }
        .btn-primary:hover { transform: translateY(-2px); box-shadow: 0 10px 30px rgba(102, 126, 234, 0.3); }
        .btn-secondary {
            background: rgba(255, 255, 255, 0.1); color: #ecf0f1;
            border: 1px solid rgba(255, 255, 255, 0.2);
        }
        .btn-secondary:hover { background: rgba(255, 255, 255, 0.2); }
        
        .services-grid {
            display: grid; grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
            gap: 24px; margin-bottom: 40px;
        }
        .service-card {
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 16px; padding: 28px;
            transition: all 0.3s ease; position: relative; overflow: hidden;
        }
        .service-card::before {
            content: ''; position: absolute; top: 0; left: 0; right: 0;
            height: 4px; background: var(--service-color, #3498db); opacity: 0.8;
        }
        .service-card:hover { transform: translateY(-4px); box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3); }
        .service-card.restricted {
            border: 1px solid rgba(241, 196, 15, 0.3);
        }
        .service-card.restricted::before {
            background: linear-gradient(90deg, #f39c12, #e67e22);
        }
        .service-card.restricted .lock-overlay {
            position: absolute; top: 20px; right: 20px;
            background: rgba(241, 196, 15, 0.2);
            color: #f1c40f; padding: 4px 10px; border-radius: 12px;
            font-size: 11px; font-weight: 500;
        }
        .service-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 16px; }
        .service-icon { font-size: 36px; }
        .service-status {
            display: flex; align-items: center; gap: 8px;
            padding: 6px 14px; border-radius: 20px;
            font-size: 12px; font-weight: 500;
        }
        .status-indicator { width: 8px; height: 8px; border-radius: 50%; animation: pulse 2s infinite; }
        @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.5; } }
        .status-running { background: rgba(46, 213, 115, 0.2); color: #2ed573; }
        .status-running .status-indicator { background: #2ed573; }
        .status-stopped { background: rgba(231, 76, 60, 0.2); color: #e74c3c; }
        .status-stopped .status-indicator { background: #e74c3c; animation: none; }
        .status-starting { background: rgba(241, 196, 15, 0.2); color: #f1c40f; }
        .status-starting .status-indicator { background: #f1c40f; }
        .service-name { font-size: 20px; font-weight: 600; margin-bottom: 8px; }
        .service-desc { font-size: 13px; color: #95a5a6; line-height: 1.6; margin-bottom: 20px; }
        .restricted-desc { color: #f1c40f; }
        .service-info { display: flex; gap: 20px; margin-bottom: 20px; font-size: 13px; }
        .info-item { display: flex; align-items: center; gap: 6px; color: #bdc3c7; }
        .info-label { color: #7f8c8d; }
        .tags { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 20px; }
        .tag {
            padding: 4px 12px;
            background: rgba(255, 255, 255, 0.1);
            border-radius: 12px; font-size: 11px; color: #bdc3c7;
        }
        .tag.restricted-tag { background: rgba(241, 196, 15, 0.2); color: #f1c40f; }
        .service-actions { display: flex; gap: 12px; }
        .access-btn {
            flex: 1; padding: 10px 16px;
            background: var(--service-color, #3498db); color: white;
            border: none; border-radius: 8px; font-size: 13px; font-weight: 500;
            cursor: pointer; transition: all 0.3s ease;
            display: flex; align-items: center; justify-content: center; gap: 6px;
            text-decoration: none;
        }
        .access-btn:hover { transform: translateY(-2px); box-shadow: 0 8px 20px rgba(0, 0, 0, 0.3); }
        .access-btn:disabled { opacity: 0.5; cursor: not-allowed; transform: none; }
        .access-btn.lock-btn {
            background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
        }
        .access-btn.login-btn {
            background: linear-gradient(135deg, #fa709a 0%, #fee140 100%);
        }
        
        .footer {
            text-align: center; color: #7f8c8d; font-size: 12px;
            padding: 20px; border-top: 1px solid rgba(255, 255, 255, 0.1);
        }
        .refresh-time { display: inline-block; margin-left: 15px; color: #7f8c8d; }
        .auto-refresh {
            position: fixed; bottom: 20px; right: 20px;
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 30px; padding: 10px 20px;
            display: flex; align-items: center; gap: 10px;
            font-size: 12px; cursor: pointer; transition: all 0.3s ease;
        }
        .auto-refresh:hover { background: rgba(255, 255, 255, 0.1); }
        .auto-refresh input { accent-color: #667eea; }
        .permission-banner {
            background: linear-gradient(135deg, rgba(155, 89, 182, 0.1) 0%, rgba(52, 152, 219, 0.1) 100%);
            border: 1px solid rgba(155, 89, 182, 0.3);
            border-radius: 12px;
            padding: 16px 20px;
            margin-bottom: 30px;
            display: flex; align-items: center; gap: 12px;
        }
        .permission-banner .banner-icon { font-size: 24px; }
        .permission-banner .banner-text { flex: 1; font-size: 13px; color: #d5dbdb; }
        .permission-banner .banner-text strong { color: #9b59b6; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <div class="logo">🌌</div>
            <h1 class="title">璇玑系统 - 服务管理面板</h1>
            <p class="subtitle">实时监控所有服务状态，一键访问应用入口</p>
        </div>
        
        <!-- 用户信息条 -->
        <div class="user-bar">
            <div class="user-info">
                <span class="user-avatar" id="user-avatar">👤</span>
                <div class="user-details">
                    <span class="user-name" id="user-name">访客</span>
                    <span class="user-role" id="user-role">
                        <span class="role-badge role-guest" id="role-badge">普通用户</span>
                    </span>
                </div>
            </div>
            <button class="btn-logout" id="btn-auth" onclick="handleAuth()">
                🔐 管理员登录
            </button>
        </div>
        
        <!-- 权限提示条 -->
        <div class="permission-banner" id="permission-banner" style="display: none;">
            <span class="banner-icon">🔒</span>
            <div class="banner-text">
                您当前以 <strong>普通用户</strong> 身份查看，部分服务信息需要管理员权限才能查看完整详情。
                <br>点击右上角「管理员登录」获得完整访问权限。
            </div>
        </div>
        
        <div class="stats">
            <div class="stat-card">
                <div class="stat-value total" id="stat-total">0</div>
                <div class="stat-label">总服务数</div>
            </div>
            <div class="stat-card">
                <div class="stat-value running" id="stat-running">0</div>
                <div class="stat-label">运行中</div>
            </div>
            <div class="stat-card">
                <div class="stat-value stopped" id="stat-stopped">0</div>
                <div class="stat-label">已停止</div>
            </div>
        </div>
        
        <div class="actions">
            <button class="btn btn-primary" onclick="refreshStatus()">🔄 刷新状态</button>
            <button class="btn btn-secondary" onclick="openAllServices()">📑 打开所有服务</button>
        </div>
        
        <div class="services-grid" id="services-grid"></div>
        
        <div class="footer">
            <p>璇玑系统 © 2026 | 服务管理面板 v1.0</p>
            <p class="refresh-time" id="refresh-time">上次刷新: --</p>
        </div>
    </div>
    
    <div class="auto-refresh" onclick="toggleAutoRefresh()">
        <input type="checkbox" id="auto-refresh-checkbox" checked>
        <label for="auto-refresh-checkbox">自动刷新 (10秒)</label>
    </div>
    
    <script>
        let autoRefreshInterval = null;
        
        async function init() {
            await checkAuth();
            refreshStatus();
            startAutoRefresh();
        }
        
        async function checkAuth() {
            try {
                const response = await fetch('/api/session');
                const data = await response.json();
                updateUserUI(data.user || 'guest');
            } catch (e) {
                updateUserUI('guest');
            }
        }
        
        function updateUserUI(userType) {
            const avatar = document.getElementById('user-avatar');
            const name = document.getElementById('user-name');
            const roleBadge = document.getElementById('role-badge');
            const roleBadgeEl = document.getElementById('role-badge');
            const authBtn = document.getElementById('btn-auth');
            const permissionBanner = document.getElementById('permission-banner');
            
            if (userType === 'admin') {
                avatar.textContent = '🛡️';
                name.textContent = '管理员';
                roleBadge.textContent = '系统管理员';
                roleBadgeEl.className = 'role-badge role-admin';
                authBtn.textContent = '📤 退出登录';
                authBtn.classList.remove('btn-logout');
                authBtn.style.background = 'rgba(46, 204, 113, 0.2)';
                authBtn.style.borderColor = 'rgba(46, 204, 113, 0.3)';
                authBtn.style.color = '#2ecc71';
                permissionBanner.style.display = 'none';
            } else {
                avatar.textContent = '👤';
                name.textContent = '访客用户';
                roleBadge.textContent = '普通用户';
                roleBadgeEl.className = 'role-badge role-guest';
                authBtn.textContent = '🔐 管理员登录';
                authBtn.classList.add('btn-logout');
                authBtn.style.background = '';
                authBtn.style.borderColor = '';
                authBtn.style.color = '';
                permissionBanner.style.display = 'flex';
            }
        }
        
        async function handleAuth() {
            try {
                const response = await fetch('/api/session');
                const data = await response.json();
                
                if (data.user === 'admin') {
                    // 退出登录
                    if (confirm('确定要退出管理员登录吗？')) {
                        await fetch('/api/logout', {method: 'POST'});
                        updateUserUI('guest');
                        refreshStatus();
                    }
                } else {
                    // 跳转到登录页
                    window.location.href = '/login';
                }
            } catch (e) {
                alert('操作失败，请重试');
            }
        }
        
        async function refreshStatus() {
            try {
                const response = await fetch('/api/status');
                const data = await response.json();
                const sessionResp = await fetch('/api/session');
                const sessionData = await sessionResp.json();
                renderServices(data, sessionData.user === 'admin');
                updateTimestamp();
            } catch (e) {
                console.error('刷新失败:', e);
            }
        }
        
        function renderServices(services, isAdmin) {
            const grid = document.getElementById('services-grid');
            grid.innerHTML = '';
            
            let runningCount = 0;
            let stoppedCount = 0;
            
            services.forEach(service => {
                if (service.status === 'running') runningCount++;
                else stoppedCount++;
                grid.appendChild(createServiceCard(service, isAdmin));
            });
            
            document.getElementById('stat-total').textContent = services.length;
            document.getElementById('stat-running').textContent = runningCount;
            document.getElementById('stat-stopped').textContent = stoppedCount;
        }
        
        function createServiceCard(service, isAdmin) {
            const card = document.createElement('div');
            const isRestricted = service.requires_auth && !isAdmin;
            card.className = `service-card ${service.status} ${isRestricted ? 'restricted' : ''}`;
            card.style.setProperty('--service-color', service.color);
            
            const statusClass = service.status === 'running' ? 'status-running' :
                               service.status === 'starting' ? 'status-starting' : 'status-stopped';
            const statusText = service.status === 'running' ? '运行中' :
                              service.status === 'starting' ? '启动中' : '已停止';
            
            const tagsHTML = service.tags.map(tag => 
                `<span class="tag ${tag === '🔒 受限访问' ? 'restricted-tag' : ''}">${tag}</span>`
            ).join('');
            
            card.innerHTML = `
                ${isRestricted ? '<div class="lock-overlay">🔒 需管理员权限</div>' : ''}
                <div class="service-header">
                    <div class="service-icon">${service.icon}</div>
                    <div class="service-status ${statusClass}">
                        <span class="status-indicator"></span>${statusText}
                    </div>
                </div>
                <h3 class="service-name">${service.name}</h3>
                <p class="service-desc ${isRestricted ? 'restricted-desc' : ''}">${service.description}</p>
                <div class="service-info">
                    <div class="info-item"><span class="info-label">端口:</span><span>:${service.port}</span></div>
                    <div class="info-item"><span class="info-label">URL:</span><span>${service.url}</span></div>
                </div>
                <div class="tags">${tagsHTML}</div>
                <div class="service-actions">
                    ${isRestricted ? 
                        '<a href="/login" class="access-btn login-btn">🔐 登录查看</a>' :
                        `<a href="${service.url}" target="_blank" class="access-btn" ${service.status !== 'running' ? 'disabled' : ''}>${service.status === 'running' ? '🚀 访问服务' : '⏸ 服务未运行'}</a>`
                    }
                </div>
            `;
            return card;
        }
        
        function openAllServices() {
            const cards = document.querySelectorAll('.service-card');
            cards.forEach((card, index) => {
                setTimeout(() => {
                    const btn = card.querySelector('.access-btn');
                    if (btn && !btn.hasAttribute('disabled') && btn.href && btn.href.startsWith('http')) {
                        window.open(btn.href, '_blank');
                    }
                }, index * 300);
            });
        }
        
        function updateTimestamp() {
            const now = new Date();
            document.getElementById('refresh-time').textContent = `上次刷新: ${now.toLocaleTimeString('zh-CN')}`;
        }
        
        function toggleAutoRefresh() {
            const checkbox = document.getElementById('auto-refresh-checkbox');
            checkbox.checked = !checkbox.checked;
            if (checkbox.checked) startAutoRefresh();
            else stopAutoRefresh();
        }
        
        function startAutoRefresh() {
            if (autoRefreshInterval) clearInterval(autoRefreshInterval);
            autoRefreshInterval = setInterval(refreshStatus, 10000);
        }
        
        function stopAutoRefresh() {
            if (autoRefreshInterval) { clearInterval(autoRefreshInterval); autoRefreshInterval = null; }
        }
        
        // 启动
        init();
    </script>
</body>
</html>'''

# ============ HTTP 处理器 ============

class MonitorHandler(BaseHTTPRequestHandler):
    """服务监控 HTTP 处理器（带权限控制）"""
    
    def _get_session(self):
        """获取会话（不设置Cookie）"""
        return get_or_create_session(self)
    
    def _send_response_with_cookie(self, status_code, content_type, body_bytes, session_id=None):
        """发送响应（包含Cookie设置）"""
        self.send_response(status_code)
        if session_id:
            cookie_value = f'session_id={session_id}; Path=/; HttpOnly; SameSite=Lax'
            self.send_header('Set-Cookie', cookie_value)
        self.send_header('Content-Type', content_type)
        self.end_headers()
        self.wfile.write(body_bytes)
    
    def do_GET(self):
        parsed = urlparse(self.path)
        path = parsed.path
        session_id, session = self._get_session()
        
        # 路由
        if path == '/login':
            self._serve_login_page(session_id)
        elif path == '/' or path == '/index.html':
            self._serve_main_page(session, session_id)
        elif path == '/api/status':
            self._serve_status(session, session_id)
        elif path == '/api/session':
            self._serve_session(session, session_id)
        else:
            self._send_response_with_cookie(404, 'text/plain; charset=utf-8', b'Not Found', session_id)
    
    def do_POST(self):
        parsed = urlparse(self.path)
        path = parsed.path
        session_id, session = self._get_session()
        
        content_length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(content_length).decode('utf-8') if content_length > 0 else '{}'
        
        try:
            data = json.loads(body)
        except:
            data = {}
        
        if path == '/api/login':
            self._handle_login(data, session_id, session)
        elif path == '/api/logout':
            self._handle_logout(session_id)
        else:
            self._send_response_with_cookie(404, 'text/plain; charset=utf-8', b'Not Found', session_id)
    
    def _serve_login_page(self, session_id):
        """提供登录页面"""
        self._send_response_with_cookie(200, 'text/html; charset=utf-8', LOGIN_HTML.encode('utf-8'), session_id)
    
    def _serve_main_page(self, session, session_id):
        """提供主页面"""
        self._send_response_with_cookie(200, 'text/html; charset=utf-8', MAIN_HTML.encode('utf-8'), session_id)
    
    def _serve_status(self, session, session_id):
        """提供服务状态 API"""
        is_admin_user = is_admin(session)
        status = get_all_service_status(is_admin_user)
        data = list(status.values())
        body = json.dumps(data, ensure_ascii=False).encode('utf-8')
        
        self.send_response(200)
        cookie_value = f'session_id={session_id}; Path=/; HttpOnly; SameSite=Lax'
        self.send_header('Set-Cookie', cookie_value)
        self.send_header('Content-Type', 'application/json; charset=utf-8')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.end_headers()
        self.wfile.write(body)
    
    def _serve_session(self, session, session_id):
        """提供会话信息 API"""
        body = json.dumps({
            'user': session.get('user', 'guest'),
            'is_admin': is_admin(session),
        }).encode('utf-8')
        self._send_response_with_cookie(200, 'application/json; charset=utf-8', body, session_id)
    
    def _handle_login(self, data, session_id, session):
        """处理登录请求"""
        username = data.get('username', '')
        password = data.get('password', '')
        
        if validate_password(username, password):
            # 登录成功，更新会话
            session['user'] = 'admin'
            session['created_at'] = time.time()
            
            body = json.dumps({
                'success': True,
                'user': 'admin',
                'message': '登录成功'
            }).encode('utf-8')
            self._send_response_with_cookie(200, 'application/json; charset=utf-8', body, session_id)
        else:
            body = json.dumps({
                'success': False,
                'message': '用户名或密码错误'
            }).encode('utf-8')
            self._send_response_with_cookie(401, 'application/json; charset=utf-8', body, session_id)
    
    def _handle_logout(self, session_id):
        """处理登出请求"""
        if session_id in sessions:
            sessions[session_id]['user'] = 'guest'
            sessions[session_id]['created_at'] = time.time()
        
        body = json.dumps({
            'success': True,
            'message': '已退出登录'
        }).encode('utf-8')
        self._send_response_with_cookie(200, 'application/json; charset=utf-8', body, session_id)
    
    def log_message(self, format, *args):
        pass

# ============ 主入口 ============

def main():
    """启动服务监控面板"""
    import argparse
    
    parser = argparse.ArgumentParser(description='璇玑系统 - 服务管理面板 (带权限控制)')
    parser.add_argument('--port', type=int, default=MONITOR_PORT, help=f'监控面板端口 (默认: {MONITOR_PORT})')
    parser.add_argument('--no-browser', action='store_true', help='启动时不自动打开浏览器')
    parser.add_argument('--admin-user', type=str, default=None, help='管理员用户名')
    parser.add_argument('--admin-pass', type=str, default=None, help='管理员密码')
    args = parser.parse_args()
    
    global ADMIN_USERNAME, ADMIN_PASSWORD_HASH
    
    # 命令行参数优先
    if args.admin_user:
        ADMIN_USERNAME = args.admin_user
    if args.admin_pass:
        ADMIN_PASSWORD_HASH = hashlib.sha256(args.admin_pass.encode()).hexdigest()
    
    port = args.port
    
    # 检查端口
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.settimeout(1)
        if sock.connect_ex(('127.0.0.1', port)) == 0:
            print(f"⚠ 端口 {port} 已被占用，请使用 --port 参数指定其他端口")
            sys.exit(1)
    
    # 启动过期会话清理线程
    def cleanup_thread():
        while True:
            time.sleep(60)
            cleanup_expired_sessions()
    
    cleanup_daemon = threading.Thread(target=cleanup_thread, daemon=True)
    cleanup_daemon.start()
    
    # 创建服务器
    server = HTTPServer(('0.0.0.0', port), MonitorHandler)
    
    monitor_url = f"http://localhost:{port}"
    
    print("\n" + "=" * 60)
    print("  璇玑系统 - 服务管理面板 (带权限控制)")
    print("=" * 60)
    print(f"\n  🌐 管理面板地址: {monitor_url}")
    print(f"\n  🔐 管理员账户:")
    print(f"     用户名: {ADMIN_USERNAME}")
    print(f"     密码:   {'(已设置)' if ADMIN_PASSWORD_HASH != hashlib.sha256('admin123'.encode()).hexdigest() else 'admin123 (默认，请尽快修改)'}")
    print(f"\n  服务权限说明:")
    for key, service in SERVICES.items():
        perm = "🔒 管理员权限" if service['is_admin_only'] else "👁️ 公开访问"
        print(f"    {service['name']:<20s} {perm}")
    print(f"\n  访问方式:")
    print(f"    1. 浏览器打开: {monitor_url}")
    print(f"    2. 管理员登录后可查看所有服务详情")
    print(f"    3. 使用环境变量修改密码: MONITOR_ADMIN_USER, MONITOR_ADMIN_PASS")
    print("\n  按 Ctrl+C 停止面板\n" + "=" * 60 + "\n")
    
    # 自动打开浏览器
    if not args.no_browser:
        try:
            webbrowser.open(monitor_url)
        except:
            pass
    
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n\n  👋 服务管理面板已停止")
        server.shutdown()

if __name__ == '__main__':
    main()