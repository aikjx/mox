#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
璇玑系统 - 服务管理面板
==============================
一个 Web 服务，用于展示所有服务状态和访问入口
启动后访问: http://localhost:9999
"""

import os
import sys
import json
import socket
import threading
import time
import webbrowser
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path
from datetime import datetime

# ============ 配置 ============
PROJECT_ROOT = Path(__file__).parent.resolve()
PID_DIR = PROJECT_ROOT / '.runtime'
MONITOR_PORT = 9999

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
    },
    'frontend': {
        'name': '用户前端界面',
        'description': '面向终端用户的操作界面，包含图谱可视化、AI 对话、专家咨询等功能',
        'port': 5173,
        'path': '/',
        'icon': '🎨',
        'color': '#2ecc71',
        'tags': ['前端', '用户界面', 'Vite'],
    },
    'admin': {
        'name': '企业管理界面',
        'description': '企业级后台管理系统，用于用户管理、权限配置、知识库管理等',
        'port': 5175,
        'path': '/',
        'icon': '⚙️',
        'color': '#9b59b6',
        'tags': ['前端', '管理后台', '企业级'],
    },
}

# ============ 工具函数 ============

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
    
    # 端口在监听，检查 HTTP 可用性
    if check_http_service(port, service['path']):
        return 'running'
    
    # 尝试根路径
    if check_http_service(port, '/'):
        return 'running'
    
    return 'starting'

def get_all_service_status():
    """获取所有服务状态"""
    status = {}
    for key, service in SERVICES.items():
        status[key] = {
            'key': key,
            'name': service['name'],
            'description': service['description'],
            'port': service['port'],
            'icon': service['icon'],
            'color': service['color'],
            'tags': service['tags'],
            'status': get_service_status(key),
            'url': f"http://localhost:{service['port']}",
        }
    return status

# ============ HTML 模板 ============

HTML_TEMPLATE = '''<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>璇玑系统 - 服务管理面板</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC', 'Microsoft YaHei', sans-serif;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
            min-height: 100vh;
            color: #ecf0f1;
        }
        
        .container {
            max-width: 1200px;
            margin: 0 auto;
            padding: 40px 20px;
        }
        
        .header {
            text-align: center;
            margin-bottom: 50px;
        }
        
        .logo {
            font-size: 48px;
            margin-bottom: 10px;
        }
        
        .title {
            font-size: 32px;
            font-weight: 700;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
            margin-bottom: 8px;
        }
        
        .subtitle {
            font-size: 14px;
            color: #95a5a6;
            margin-bottom: 20px;
        }
        
        .stats {
            display: flex;
            justify-content: center;
            gap: 30px;
            margin-bottom: 40px;
        }
        
        .stat-card {
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 12px;
            padding: 20px 30px;
            text-align: center;
            backdrop-filter: blur(10px);
        }
        
        .stat-value {
            font-size: 36px;
            font-weight: 700;
            margin-bottom: 5px;
        }
        
        .stat-value.running { color: #2ecc71; }
        .stat-value.stopped { color: #e74c3c; }
        .stat-value.total { color: #3498db; }
        
        .stat-label {
            font-size: 12px;
            color: #95a5a6;
        }
        
        .actions {
            display: flex;
            justify-content: center;
            gap: 15px;
            margin-bottom: 40px;
            flex-wrap: wrap;
        }
        
        .btn {
            padding: 12px 24px;
            border: none;
            border-radius: 8px;
            font-size: 14px;
            font-weight: 500;
            cursor: pointer;
            transition: all 0.3s ease;
            display: flex;
            align-items: center;
            gap: 8px;
        }
        
        .btn-primary {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
        }
        
        .btn-primary:hover {
            transform: translateY(-2px);
            box-shadow: 0 10px 30px rgba(102, 126, 234, 0.3);
        }
        
        .btn-secondary {
            background: rgba(255, 255, 255, 0.1);
            color: #ecf0f1;
            border: 1px solid rgba(255, 255, 255, 0.2);
        }
        
        .btn-secondary:hover {
            background: rgba(255, 255, 255, 0.2);
        }
        
        .btn-success {
            background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%);
            color: white;
        }
        
        .btn-success:hover {
            transform: translateY(-2px);
            box-shadow: 0 10px 30px rgba(17, 153, 142, 0.3);
        }
        
        .btn-danger {
            background: linear-gradient(135deg, #eb3349 0%, #f45c43 100%);
            color: white;
        }
        
        .btn-danger:hover {
            transform: translateY(-2px);
            box-shadow: 0 10px 30px rgba(235, 51, 73, 0.3);
        }
        
        .services-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
            gap: 24px;
            margin-bottom: 40px;
        }
        
        .service-card {
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 16px;
            padding: 28px;
            transition: all 0.3s ease;
            position: relative;
            overflow: hidden;
        }
        
        .service-card::before {
            content: '';
            position: absolute;
            top: 0;
            left: 0;
            right: 0;
            height: 4px;
            background: var(--service-color, #3498db);
            opacity: 0.8;
        }
        
        .service-card:hover {
            transform: translateY(-4px);
            box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3);
        }
        
        .service-card.stopped {
            opacity: 0.7;
        }
        
        .service-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 16px;
        }
        
        .service-icon {
            font-size: 36px;
        }
        
        .service-status {
            display: flex;
            align-items: center;
            gap: 8px;
            padding: 6px 14px;
            border-radius: 20px;
            font-size: 12px;
            font-weight: 500;
        }
        
        .status-indicator {
            width: 8px;
            height: 8px;
            border-radius: 50%;
            animation: pulse 2s infinite;
        }
        
        @keyframes pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.5; }
        }
        
        .status-running {
            background: rgba(46, 213, 115, 0.2);
            color: #2ed573;
        }
        
        .status-running .status-indicator {
            background: #2ed573;
        }
        
        .status-stopped {
            background: rgba(231, 76, 60, 0.2);
            color: #e74c3c;
        }
        
        .status-stopped .status-indicator {
            background: #e74c3c;
            animation: none;
        }
        
        .status-starting {
            background: rgba(241, 196, 15, 0.2);
            color: #f1c40f;
        }
        
        .status-starting .status-indicator {
            background: #f1c40f;
        }
        
        .service-name {
            font-size: 20px;
            font-weight: 600;
            margin-bottom: 8px;
        }
        
        .service-desc {
            font-size: 13px;
            color: #95a5a6;
            line-height: 1.6;
            margin-bottom: 20px;
        }
        
        .service-info {
            display: flex;
            gap: 20px;
            margin-bottom: 20px;
            font-size: 13px;
        }
        
        .info-item {
            display: flex;
            align-items: center;
            gap: 6px;
            color: #bdc3c7;
        }
        
        .info-label {
            color: #7f8c8d;
        }
        
        .tags {
            display: flex;
            flex-wrap: wrap;
            gap: 8px;
            margin-bottom: 20px;
        }
        
        .tag {
            padding: 4px 12px;
            background: rgba(255, 255, 255, 0.1);
            border-radius: 12px;
            font-size: 11px;
            color: #bdc3c7;
        }
        
        .service-actions {
            display: flex;
            gap: 12px;
        }
        
        .access-btn {
            flex: 1;
            padding: 10px 16px;
            background: var(--service-color, #3498db);
            color: white;
            border: none;
            border-radius: 8px;
            font-size: 13px;
            font-weight: 500;
            cursor: pointer;
            transition: all 0.3s ease;
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 6px;
            text-decoration: none;
        }
        
        .access-btn:hover {
            transform: translateY(-2px);
            box-shadow: 0 8px 20px rgba(0, 0, 0, 0.3);
        }
        
        .access-btn:disabled {
            opacity: 0.5;
            cursor: not-allowed;
            transform: none;
        }
        
        .footer {
            text-align: center;
            color: #7f8c8d;
            font-size: 12px;
            padding: 20px;
            border-top: 1px solid rgba(255, 255, 255, 0.1);
        }
        
        .refresh-time {
            display: inline-block;
            margin-left: 15px;
            color: #7f8c8d;
        }
        
        .auto-refresh {
            position: fixed;
            bottom: 20px;
            right: 20px;
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 30px;
            padding: 10px 20px;
            display: flex;
            align-items: center;
            gap: 10px;
            font-size: 12px;
            cursor: pointer;
            transition: all 0.3s ease;
        }
        
        .auto-refresh:hover {
            background: rgba(255, 255, 255, 0.1);
        }
        
        .auto-refresh input {
            accent-color: #667eea;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <div class="logo">🌌</div>
            <h1 class="title">璇玑系统 - 服务管理面板</h1>
            <p class="subtitle">实时监控所有服务状态，一键访问应用入口</p>
        </div>
        
        <div class="stats">
            <div class="stat-card">
                <div class="stat-value total" id="stat-total">3</div>
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
            <button class="btn btn-primary" onclick="refreshStatus()">
                🔄 刷新状态
            </button>
            <button class="btn btn-secondary" onclick="openAllServices()">
                📑 打开所有服务
            </button>
        </div>
        
        <div class="services-grid" id="services-grid">
            <!-- 服务卡片将通过 JavaScript 动态生成 -->
        </div>
        
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
        
        async function refreshStatus() {
            try {
                const response = await fetch('/api/status');
                const data = await response.json();
                renderServices(data);
                updateTimestamp();
            } catch (e) {
                console.error('刷新失败:', e);
            }
        }
        
        function renderServices(services) {
            const grid = document.getElementById('services-grid');
            grid.innerHTML = '';
            
            let runningCount = 0;
            let stoppedCount = 0;
            
            services.forEach(service => {
                if (service.status === 'running') runningCount++;
                else stoppedCount++;
                
                const card = createServiceCard(service);
                grid.appendChild(card);
            });
            
            document.getElementById('stat-running').textContent = runningCount;
            document.getElementById('stat-stopped').textContent = stoppedCount;
        }
        
        function createServiceCard(service) {
            const card = document.createElement('div');
            card.className = `service-card ${service.status}`;
            card.style.setProperty('--service-color', service.color);
            
            const statusClass = service.status === 'running' ? 'status-running' : 
                               service.status === 'starting' ? 'status-starting' : 'status-stopped';
            const statusText = service.status === 'running' ? '运行中' :
                              service.status === 'starting' ? '启动中' : '已停止';
            
            const tagsHTML = service.tags.map(tag => `<span class="tag">${tag}</span>`).join('');
            
            card.innerHTML = `
                <div class="service-header">
                    <div class="service-icon">${service.icon}</div>
                    <div class="service-status ${statusClass}">
                        <span class="status-indicator"></span>
                        ${statusText}
                    </div>
                </div>
                <h3 class="service-name">${service.name}</h3>
                <p class="service-desc">${service.description}</p>
                <div class="service-info">
                    <div class="info-item">
                        <span class="info-label">端口:</span>
                        <span>:${service.port}</span>
                    </div>
                    <div class="info-item">
                        <span class="info-label">URL:</span>
                        <span>${service.url}</span>
                    </div>
                </div>
                <div class="tags">${tagsHTML}</div>
                <div class="service-actions">
                    <a href="${service.url}" target="_blank" class="access-btn" ${service.status !== 'running' ? 'disabled' : ''}>
                        ${service.status === 'running' ? '🚀 访问服务' : '⏸ 服务未运行'}
                    </a>
                </div>
            `;
            
            return card;
        }
        
        function openAllServices() {
            const cards = document.querySelectorAll('.service-card');
            cards.forEach((card, index) => {
                setTimeout(() => {
                    const btn = card.querySelector('.access-btn');
                    if (btn && !btn.hasAttribute('disabled')) {
                        window.open(btn.href, '_blank');
                    }
                }, index * 300);
            });
        }
        
        function updateTimestamp() {
            const now = new Date();
            const timeStr = now.toLocaleTimeString('zh-CN');
            document.getElementById('refresh-time').textContent = `上次刷新: ${timeStr}`;
        }
        
        function toggleAutoRefresh() {
            const checkbox = document.getElementById('auto-refresh-checkbox');
            checkbox.checked = !checkbox.checked;
            if (checkbox.checked) {
                startAutoRefresh();
            } else {
                stopAutoRefresh();
            }
        }
        
        function startAutoRefresh() {
            if (autoRefreshInterval) clearInterval(autoRefreshInterval);
            autoRefreshInterval = setInterval(refreshStatus, 10000);
        }
        
        function stopAutoRefresh() {
            if (autoRefreshInterval) {
                clearInterval(autoRefreshInterval);
                autoRefreshInterval = null;
            }
        }
        
        // 初始化
        refreshStatus();
        startAutoRefresh();
    </script>
</body>
</html>'''

# ============ HTTP 处理器 ============

class MonitorHandler(BaseHTTPRequestHandler):
    """服务监控 HTTP 处理器"""
    
    def do_GET(self):
        if self.path == '/' or self.path == '/index.html':
            self._serve_html()
        elif self.path == '/api/status':
            self._serve_status()
        else:
            self.send_error(404)
    
    def _serve_html(self):
        self.send_response(200)
        self.send_header('Content-Type', 'text/html; charset=utf-8')
        self.end_headers()
        self.wfile.write(HTML_TEMPLATE.encode('utf-8'))
    
    def _serve_status(self):
        status = get_all_service_status()
        data = list(status.values())
        
        self.send_response(200)
        self.send_header('Content-Type', 'application/json; charset=utf-8')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.end_headers()
        self.wfile.write(json.dumps(data, ensure_ascii=False).encode('utf-8'))
    
    def log_message(self, format, *args):
        # 静默日志
        pass

# ============ 主入口 ============

def main():
    """启动服务监控面板"""
    import argparse
    
    parser = argparse.ArgumentParser(description='璇玑系统 - 服务管理面板')
    parser.add_argument('--port', type=int, default=MONITOR_PORT, help=f'监控面板端口 (默认: {MONITOR_PORT})')
    parser.add_argument('--no-browser', action='store_true', help='启动时不自动打开浏览器')
    args = parser.parse_args()
    
    port = args.port
    
    # 检查端口是否被占用
    if is_port_in_use(port):
        print(f"⚠ 端口 {port} 已被占用，请关闭占用进程或使用 --port 参数指定其他端口")
        print(f"   示例: python service_monitor.py --port 9998")
        sys.exit(1)
    
    # 创建 HTTP 服务器
    try:
        server = HTTPServer(('0.0.0.0', port), MonitorHandler)
    except Exception as e:
        print(f"❌ 无法启动服务: {e}")
        sys.exit(1)
    
    monitor_url = f"http://localhost:{port}"
    
    print("\n" + "=" * 60)
    print("  璇玑系统 - 服务管理面板")
    print("=" * 60)
    print(f"\n  🌐 管理面板地址: {monitor_url}")
    print(f"\n  服务列表:")
    for key, service in SERVICES.items():
        status = get_service_status(key)
        status_icon = '🟢' if status == 'running' else '🔴' if status == 'stopped' else '🟡'
        print(f"    {status_icon} {service['name']:<20s} http://localhost:{service['port']}")
    print(f"\n  访问方式:")
    print(f"    1. 浏览器打开: {monitor_url}")
    print(f"    2. Python 启动: python service_manager.py start")
    print(f"    3. 查看状态:   python service_manager.py status")
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