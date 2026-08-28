# MOX 一体化部署指南

> 版本: 1.0 | 日期: 2026-08-28

---

## 一、部署方式总览

| 方式 | 适用场景 | 复杂度 | 推荐度 |
|------|---------|--------|--------|
| 本地部署 | 开发/测试/小型生产 | 低 | ★★★ |
| Docker部署 | 标准生产/快速上线 | 中 | ★★★★★ |
| 远程部署 | 多服务器/集群 | 高 | ★★★★ |

---

## 二、本地一体化部署

### 2.1 环境要求

- Python 3.10+
- Redis 6+（可选，无Redis时使用内存缓存）
- 512MB+ 内存

### 2.2 一键部署

```bash
# 完整部署(安装依赖+初始化+导入数据+启动)
python tools/deploy.py local --data exports/mox-export-mox-*.json --start

# 仅部署不启动
python tools/deploy.py local --data exports/mox-export-mox-*.json

# 空数据库部署(不导入数据)
python tools/deploy.py local --start
```

### 2.3 手动部署步骤

```bash
# 1. 安装依赖
cd platform/mox-server
pip install -r requirements.txt

# 2. 初始化数据库(自动建表)
python -c "from app.database import init_db; init_db()"

# 3. 导入初始化数据
cd ../..
python tools/import_data.py exports/mox-export-mox-*.json

# 4. 启动服务
cd platform/mox-server
python run.py 8600

# 5. 访问
# 后端API: http://localhost:8600/api/health
# 前端官网: file:///绝对路径/frontend-ui/mox-website/index.html
```

### 2.4 前端部署到Nginx

```bash
# 前端静态文件拷贝到Nginx
cp -r frontend-ui/mox-website /var/www/mox-website

# Nginx配置(见 deploy/nginx.conf)
sudo cp deploy/nginx.conf /etc/nginx/conf.d/mox.conf
sudo nginx -t && sudo systemctl reload nginx
```

---

## 三、Docker一体化部署（推荐生产）

### 3.1 前置要求

- Docker 20+
- Docker Compose 2+

### 3.2 一键部署

```bash
# 构建并启动
python tools/deploy.py docker

# 或手动
docker-compose up -d --build

# 查看状态
docker-compose ps

# 查看日志
docker-compose logs -f mox-server

# 停止
docker-compose down
```

### 3.3 docker-compose.yml

```yaml
version: '3.8'
services:
  mox-server:
    build:
      context: .
      dockerfile: deploy/Dockerfile
    ports:
      - "8600:8600"
    volumes:
      - mox-data:/app/data
      - ./frontend-ui:/app/frontend:ro
    environment:
      - MOX_HOST=0.0.0.0
      - MOX_PORT=8600
      - MOX_REDIS=redis://redis:6379/0
    restart: unless-stopped
    depends_on:
      - redis
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8600/api/health"]
      interval: 30s
      timeout: 5s
      retries: 3

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    restart: unless-stopped

volumes:
  mox-data:
  redis-data:
```

### 3.4 Dockerfile

```dockerfile
FROM python:3.11-slim
WORKDIR /app
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
COPY platform/mox-server/requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY platform/mox-server/ ./
COPY tools/ /app/tools/
EXPOSE 8600
CMD ["python", "run.py", "8600"]
```

### 3.5 导入数据到Docker容器

```bash
# 拷贝导出文件到容器
docker cp exports/mox-export-mox-*.json mox-server:/tmp/

# 在容器内执行导入
docker exec mox-server python /app/tools/import_data.py /tmp/mox-export-mox-*.json

# 或使用docker-compose exec
docker-compose exec mox-server python /app/tools/import_data.py /tmp/mox-export-mox-*.json
```

---

## 四、Nginx反向代理配置（生产必备）

### 4.1 配置文件 deploy/nginx.conf

```nginx
server {
    listen 80;
    server_name your-domain.com;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl http2;
    server_name your-domain.com;

    ssl_certificate     /etc/letsencrypt/live/your-domain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/your-domain.com/privkey.pem;

    # 前端静态文件
    root /var/www/mox-website;
    index index.html;

    # 后端API反向代理
    location /api/ {
        proxy_pass http://127.0.0.1:8600/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 60s;
        proxy_send_timeout 60s;
    }

    # SPA路由回退(hash路由不需要,但加上更稳)
    location / {
        try_files $uri $uri/ /index.html;
    }

    # 静态资源缓存
    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff2?)$ {
        expires 30d;
        add_header Cache-Control "public, immutable";
    }

    # 安全头
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
}
```

### 4.2 SSL证书（免费）

```bash
# 安装certbot
sudo apt install certbot python3-certbot-nginx

# 自动获取并配置SSL
sudo certbot --nginx -d your-domain.com

# 自动续期(已内置)
sudo certbot renew --dry-run
```

---

## 五、Systemd服务配置（生产守护）

### 5.1 deploy/systemd.service

```ini
[Unit]
Description=MOX Lowcode Platform Server
After=network.target redis.service

[Service]
Type=simple
User=www-data
WorkingDirectory=/opt/mox/platform/mox-server
ExecStart=/usr/bin/python3 run.py 8600
Restart=always
RestartSec=3
Environment=MOX_HOST=0.0.0.0
Environment=MOX_PORT=8600
Environment=MOX_REDIS=redis://localhost:6379/0

[Install]
WantedBy=multi-user.target
```

### 5.2 安装服务

```bash
sudo cp deploy/systemd.service /etc/systemd/system/mox-server.service
sudo systemctl daemon-reload
sudo systemctl enable --now mox-server
sudo systemctl status mox-server
```

---

## 六、一体化部署完整流程（生产推荐）

```bash
# === 第1步: 打包 ===
python tools/package.py --version 1.0.0 --format tar.gz --with-data

# === 第2步: 上传到服务器 ===
scp dist/mox-platform-1.0.0-*.tar.gz user@server:/tmp/

# === 第3步: 服务器上解压部署 ===
ssh user@server
cd /opt && sudo tar xzf /tmp/mox-platform-1.0.0-*.tar.gz
cd mox-platform-1.0.0-*

# === 第4步: Docker一键启动 ===
docker-compose up -d --build

# === 第5步: 导入数据 ===
docker cp exports/*.json $(docker-compose ps -q mox-server):/tmp/
docker-compose exec mox-server python /app/tools/import_data.py /tmp/mox-export-*.json

# === 第6步: 配置Nginx+SSL ===
sudo cp deploy/nginx.conf /etc/nginx/conf.d/mox.conf
sudo certbot --nginx -d your-domain.com
sudo nginx -t && sudo systemctl reload nginx

# === 第7步: 验证 ===
curl https://your-domain.com/api/health
# 浏览器访问 https://your-domain.com
```

---

## 七、部署验证清单

- [ ] 后端API健康检查通过: `/api/health` 返回 `{"status":"ok"}`
- [ ] 前端页面正常加载，无JS错误
- [ ] 所有路由页面正常（home/products/news/cases/about/contact/jobs/admin）
- [ ] 管理中心AI对话正常
- [ ] 数据导入完整（记录数与导出一致）
- [ ] SSL证书有效（https正常）
- [ ] Nginx反向代理正常（/api/路由到后端）
- [ ] 静态资源缓存生效
- [ ] 安全头配置正确
- [ ] 服务自动重启（kill进程后自动恢复）
- [ ] 日志正常输出

---

## 八、常见问题

### Q: 后端启动但前端连不上？
A: 检查API_BASE配置。file://协议自动用127.0.0.1:8600，http/https协议自动用相对路径/api。确保Nginx配置了`/api/`反向代理。

### Q: Docker容器内无法连接Redis？
A: docker-compose.yml中MOX_REDIS应使用`redis://redis:6379/0`（服务名），不是localhost。

### Q: 导入数据后管理中心看不到？
A: 检查app_key是否匹配。导出时`--app-key`和导入时`--target-app`需与前端URL的`?app=`参数一致。

### Q: 如何更新SQL模板而不影响业务数据？
A: `python tools/export_data.py --kernel-only` 导出内核，`python tools/import_data.py kernel.json --kernel-only` 导入。SQL模板自动版本+1，可回滚。
