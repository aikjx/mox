// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 预置种子模板（商城行业等）

use std::collections::BTreeMap;

use super::types::{Domain, MarketError, SystemTemplate};
use super::market::TemplateMarket;
use anyhow::Result;

impl TemplateMarket {
    /// 预置「商城行业」种子模板：一份完整可复用的电商系统蓝图 +
    /// 由 codegen 风格的 DDL / 前端骨架组成的 artifacts。
    /// 用于在空白市场上提供可立即引用/二开的"通用商城模块"。
    pub fn seed_mall_templates(&self) -> Result<Vec<SystemTemplate>, MarketError> {
        let mut seeded = Vec::new();

        // ---- 种子1：标准电商商城（商品 / 购物车 / 下单 / 支付）----
        let mall_graph = serde_json::json!({
            "name": "标准电商商城",
            "nodes": [
                {"id":"start","type":"start","name":"开始"},
                {"id":"f1","type":"data_input","name":"浏览商品","action":"浏览","entities":["商品"],"depends_on":["start"]},
                {"id":"f2","type":"operator","name":"加入购物车","action":"加购","entities":["购物车","商品"],"depends_on":["f1"]},
                {"id":"f3","type":"operator","name":"提交订单","action":"下单","entities":["订单","购物车"],"depends_on":["f2"]},
                {"id":"f4","type":"operator","name":"支付订单","action":"支付","entities":["订单","支付"],"depends_on":["f3"]},
                {"id":"f5","type":"data_output","name":"通知发货","action":"通知","entities":["订单"],"depends_on":["f4"]},
                {"id":"end","type":"end","name":"结束"}
            ]
        });
        let mut mall_artifacts = BTreeMap::new();
        mall_artifacts.insert("generated/schema.sql".into(), mall_schema_sql());
        mall_artifacts.insert("generated/App.vue".into(), mall_app_vue());
        let mut mall = SystemTemplate::new(
            "标准电商商城",
            "通用商城系统：商品浏览、购物车、下单、支付、发货通知。可直接引用二开。",
            vec![Domain::Mall],
            mall_graph,
        )
        .with_artifacts(mall_artifacts);
        mall.reuse_count = 128; // 预置为热门模板（持续学习信号）
        mall.rating = 4.8;
        self.publish(&mall)?;
        seeded.push(mall);

        // ---- 种子2：会员制商城（在种子1基础上扩展会员/积分）----
        let member_graph = serde_json::json!({
            "name": "会员制商城",
            "nodes": [
                {"id":"start","type":"start","name":"开始"},
                {"id":"f1","type":"operator","name":"会员登录","action":"登录","entities":["会员"],"depends_on":["start"]},
                {"id":"f2","type":"data_input","name":"浏览商品","action":"浏览","entities":["商品"],"depends_on":["f1"]},
                {"id":"f3","type":"operator","name":"下单并积分","action":"下单","entities":["订单","会员","积分"],"depends_on":["f2"]},
                {"id":"f4","type":"operator","name":"支付","action":"支付","entities":["订单"],"depends_on":["f3"]},
                {"id":"end","type":"end","name":"结束"}
            ]
        });
        let mut member_artifacts = BTreeMap::new();
        member_artifacts.insert("generated/schema.sql".into(), member_schema_sql());
        let mut member = SystemTemplate::new(
            "会员制商城",
            "在通用商城基础上增加会员登录与积分体系，适合订阅/复购场景。",
            vec![Domain::Mall, Domain::ProductDesign],
            member_graph,
        )
        .with_artifacts(member_artifacts);
        member.reuse_count = 64;
        member.rating = 4.6;
        self.publish(&member)?;
        seeded.push(member);

        Ok(seeded)
    }

    /// 幂等播种：仅当市场为空（无任何模板）时写入种子。
    /// 适合在程序首次启动时调用，避免覆盖用户已发布的模板。
    pub fn ensure_seeded(&self) -> Result<usize, MarketError> {
        let existing = self.list(None, None)?;
        if !existing.is_empty() {
            return Ok(0);
        }
        let seeded = self.seed_mall_templates()?;
        Ok(seeded.len())
    }
}

/// 标准电商商城 DDL（PostgreSQL 风格）
fn mall_schema_sql() -> String {
    r#"-- 标准电商商城 · 自动生成 DDL（PostgreSQL）
CREATE TABLE IF NOT EXISTS product (
    id          BIGSERIAL PRIMARY KEY,
    name        VARCHAR(255) NOT NULL,
    price       NUMERIC(12,2) NOT NULL DEFAULT 0,
    stock       INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS cart (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL,
    product_id  BIGINT NOT NULL REFERENCES product(id),
    qty         INT NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS orders (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL,
    total       NUMERIC(12,2) NOT NULL DEFAULT 0,
    status      VARCHAR(32) NOT NULL DEFAULT 'pending',
    paid_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS payment (
    id          BIGSERIAL PRIMARY KEY,
    order_id    BIGINT NOT NULL REFERENCES orders(id),
    amount      NUMERIC(12,2) NOT NULL,
    channel     VARCHAR(32) NOT NULL,
    paid_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
"#
    .to_string()
}

/// 会员制商城 DDL（在商城基础上增加会员/积分）
fn member_schema_sql() -> String {
    let mut s = mall_schema_sql();
    s.push_str(
        r#"
-- 会员 / 积分扩展
CREATE TABLE IF NOT EXISTS member (
    id          BIGSERIAL PRIMARY KEY,
    username    VARCHAR(64) NOT NULL UNIQUE,
    level       INT NOT NULL DEFAULT 1,
    points      INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS point_log (
    id          BIGSERIAL PRIMARY KEY,
    member_id   BIGINT NOT NULL REFERENCES member(id),
    delta       INT NOT NULL,
    reason      VARCHAR(64),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
"#,
    );
    s
}

/// 标准电商商城前端骨架（Vue3）
fn mall_app_vue() -> String {
    r#"<template>
  <div class="mall">
    <h1>商城</h1>
    <ul>
      <li v-for="p in products" :key="p.id">
        {{ p.name }} —— ¥{{ p.price }}
        <button @click="addToCart(p)">加入购物车</button>
      </li>
    </ul>
    <button @click="checkout">去结算</button>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
const products = ref([])
async function load() {
  const r = await fetch('/api/products')
  products.value = await r.json()
}
function addToCart(p) { /* 调用 /api/cart */ }
function checkout() { /* 调用 /api/order */ }
onMounted(load)
</script>
"#
    .to_string()
}
