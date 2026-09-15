# MOX 基础部署

默认拓扑只运行 `mox-server:3080`。KG、KB、Cloud、IAM 已作为模块编译进网关，Ingress 的所有流量都先经过网关鉴权与路由。

部署前创建 JWT Secret，再应用清单：

```bash
kubectl create namespace mox --dry-run=client -o yaml | kubectl apply -f -
kubectl -n mox create secret generic mox-gateway-secret \
  --from-literal=jwt-secret="$(openssl rand -hex 32)"
kubectl apply -f deploy/k8s/base/mox-platform.yaml
```

当前持久层是单节点 SQLite，因此 Deployment 使用一个副本、`Recreate` 策略和一个 `ReadWriteOnce` PVC。迁移到共享数据库后，才能安全提高副本数并启用 HPA。
