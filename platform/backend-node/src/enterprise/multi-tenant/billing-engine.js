'use strict';

/**
 * MOX Enterprise · 计费引擎
 * =========================
 * 基于用量数据生成账单，支持多种计费模式
 *
 * 计费模式：
 *  - 订阅制（包月/包年）
 *  - 按量付费（Pay-as-you-go）
 *  - 阶梯定价（用量越多单价越低）
 *  - 预留容量（承诺使用量折扣）
 *  - 混合模式（基础订阅 + 超额按量）
 *
 * 账单周期：
 *  - 实时预估
 *  - 日结
 *  - 月结（正式账单）
 *
 * 支付集成：
 *  - 支付宝/微信支付
 *  - 信用卡（Stripe）
 *  - 银行转账（企业客户）
 *  - 预付费账户余额
 */

const { EventEmitter } = require('events');
const crypto = require('crypto');

// ─── 计费模式 ───
const BILLING_MODE = {
  SUBSCRIPTION: 'subscription',       // 订阅制
  PAY_AS_YOU_GO: 'pay_as_you_go',    // 按量付费
  TIERED: 'tiered',                   // 阶梯定价
  RESERVED: 'reserved',               // 预留容量
  HYBRID: 'hybrid',                   // 混合模式
};

// ─── 账单状态 ───
const INVOICE_STATUS = {
  DRAFT: 'draft',               // 草稿（预估中）
  PENDING: 'pending',           // 待支付
  PAID: 'paid',                 // 已支付
  OVERDUE: 'overdue',           // 逾期
  CANCELLED: 'cancelled',       // 已取消
  REFUNDED: 'refunded',         // 已退款
  PARTIALLY_PAID: 'partially_paid', // 部分支付
};

// ─── 计费项目类型 ───
const BILLING_ITEM_TYPE = {
  STORAGE: 'storage',
  API_CALLS: 'api_calls',
  EGRESS: 'egress',
  COMPUTE: 'compute',
  SUBSCRIPTION_FEE: 'subscription_fee',
  OVERAGE: 'overage',
  DISCOUNT: 'discount',
  TAX: 'tax',
};

// ─── 定价表（示例） ───
const PRICING = {
  storage: {
    unit: 'GB/month',
    tiers: [
      { upTo: 1024, price: 0.08 },      // 前 1TB: ¥0.08/GB/月
      { upTo: 10240, price: 0.06 },     // 1-10TB: ¥0.06/GB/月
      { upTo: 102400, price: 0.04 },    // 10-100TB: ¥0.04/GB/月
      { upTo: Infinity, price: 0.025 },  // >100TB: ¥0.025/GB/月
    ],
  },
  api_calls: {
    unit: 'per 1000 calls',
    price: 0.01,
    freeTier: 1000000, // 每月 100 万次免费
  },
  egress: {
    unit: 'GB',
    price: 0.5,
    freeTier: 100, // 每月 100GB 免费
  },
  compute: {
    unit: 'core-hour',
    price: 0.5,
  },
  subscription: {
    free: { monthly: 0, yearly: 0 },
    pro: { monthly: 999, yearly: 9990 },
    enterprise: { monthly: 9999, yearly: 99990 },
  },
};

class BillingEngine extends EventEmitter {
  /**
   * @param {object} options
   * @param {object} options.usageMeter     用量采集器
   * @param {object} options.icebergWriter   Iceberg 写入器
   * @param {object} options.paymentGateway  支付网关
   * @param {string} options.currency        货币（默认 CNY）
   * @param {number} options.taxRate         税率（默认 0.06 = 6%）
   * @param {number} options.billingDay      账单日（默认 1 号）
   */
  constructor(options = {}) {
    super();
    this.usageMeter = options.usageMeter;
    this.icebergWriter = options.icebergWriter;
    this.paymentGateway = options.paymentGateway;
    this.currency = options.currency || 'CNY';
    this.taxRate = options.taxRate || 0.06;
    this.billingDay = options.billingDay || 1;

    // 租户账单配置
    this.tenantBilling = new Map(); // tenantId -> { mode, plan, paymentMethod, contactInfo }

    // 账单缓存
    this.invoices = new Map(); // invoiceId -> invoice

    // 账户余额（预付费）
    this.accountBalances = new Map(); // tenantId -> balance
  }

  /**
   * 配置租户计费
   */
  configureTenant(tenantId, config) {
    const billingConfig = {
      tenantId,
      mode: config.mode || BILLING_MODE.HYBRID,
      plan: config.plan || 'pro',
      paymentMethod: config.paymentMethod || 'balance',
      contactInfo: config.contactInfo || {},
      taxId: config.taxId || null,
      billingAddress: config.billingAddress || null,
      configuredAt: new Date().toISOString(),
    };

    this.tenantBilling.set(tenantId, billingConfig);
    this.emit('billing:tenant_configured', { tenantId, mode: billingConfig.mode });
    return billingConfig;
  }

  /**
   * 生成预估账单（实时）
   */
  async generateEstimate(tenantId, period = 'current_month') {
    const config = this.tenantBilling.get(tenantId);
    if (!config) throw new Error(`租户未配置计费: ${tenantId}`);

    const usage = await this.usageMeter.getTenantUsageSummary(tenantId, period);
    const items = this._calculateBillItems(config, usage);
    const subtotal = items.reduce((s, item) => s + item.amount, 0);
    const tax = subtotal * this.taxRate;
    const total = subtotal + tax;

    return {
      tenantId,
      period,
      mode: config.mode,
      plan: config.plan,
      currency: this.currency,
      items,
      subtotal,
      tax,
      taxRate: this.taxRate,
      total,
      estimated: true,
      generatedAt: new Date().toISOString(),
    };
  }

  /**
   * 生成正式账单（月结）
   */
  async generateInvoice(tenantId, month = null) {
    const config = this.tenantBilling.get(tenantId);
    if (!config) throw new Error(`租户未配置计费: ${tenantId}`);

    const invoiceId = `INV-${crypto.randomBytes(6).toString('hex').toUpperCase()}`;
    const billingPeriod = month || this._getPreviousMonth();
    const estimate = await this.generateEstimate(tenantId, billingPeriod);

    const invoice = {
      invoiceId,
      tenantId,
      invoiceNumber: this._generateInvoiceNumber(),
      billingPeriod,
      issueDate: new Date().toISOString(),
      dueDate: this._getDueDate(),
      mode: config.mode,
      plan: config.plan,
      currency: this.currency,
      items: estimate.items,
      subtotal: estimate.subtotal,
      tax: estimate.tax,
      taxRate: this.taxRate,
      total: estimate.total,
      status: INVOICE_STATUS.PENDING,
      paymentMethod: config.paymentMethod,
      contactInfo: config.contactInfo,
      billingAddress: config.billingAddress,
      taxId: config.taxId,
      createdAt: new Date().toISOString(),
    };

    this.invoices.set(invoiceId, invoice);

    // 写入数据湖
    if (this.icebergWriter) {
      await this.icebergWriter.append('invoices', [invoice]);
    }

    this.emit('billing:invoice_generated', invoice);
    return invoice;
  }

  /**
   * 支付账单
   */
  async payInvoice(invoiceId, paymentMethod = null) {
    const invoice = this.invoices.get(invoiceId);
    if (!invoice) throw new Error(`账单不存在: ${invoiceId}`);
    if (invoice.status === INVOICE_STATUS.PAID) return invoice;

    const method = paymentMethod || invoice.paymentMethod;

    try {
      // 账户余额支付
      if (method === 'balance') {
        const balance = this.accountBalances.get(invoice.tenantId) || 0;
        if (balance >= invoice.total) {
          this.accountBalances.set(invoice.tenantId, balance - invoice.total);
          invoice.status = INVOICE_STATUS.PAID;
          invoice.paidAt = new Date().toISOString();
          invoice.paymentMethod = 'balance';
          this.emit('billing:invoice_paid', { invoiceId, amount: invoice.total, method: 'balance' });
          return invoice;
        }
        throw new Error(`账户余额不足: 余额 ${balance}, 应付 ${invoice.total}`);
      }

      // 支付网关支付
      if (this.paymentGateway) {
        const result = await this.paymentGateway.charge({
          amount: invoice.total,
          currency: this.currency,
          method,
          description: `MOX 账单 ${invoice.invoiceNumber}`,
          metadata: { invoiceId, tenantId: invoice.tenantId },
        });

        invoice.status = INVOICE_STATUS.PAID;
        invoice.paidAt = new Date().toISOString();
        invoice.paymentMethod = method;
        invoice.paymentTransactionId = result.transactionId;
        this.emit('billing:invoice_paid', { invoiceId, amount: invoice.total, method });
        return invoice;
      }

      throw new Error(`不支持的支付方式: ${method}`);

    } catch (err) {
      this.emit('billing:payment_failed', { invoiceId, error: err.message });
      throw err;
    }
  }

  /**
   * 充值账户余额
   */
  async rechargeBalance(tenantId, amount, paymentMethod = 'alipay') {
    if (amount <= 0) throw new Error('充值金额必须大于 0');

    // 支付网关扣款
    if (this.paymentGateway) {
      await this.paymentGateway.charge({
        amount,
        currency: this.currency,
        method: paymentMethod,
        description: `MOX 账户充值 ¥${amount}`,
        metadata: { tenantId, type: 'recharge' },
      });
    }

    const currentBalance = this.accountBalances.get(tenantId) || 0;
    const newBalance = currentBalance + amount;
    this.accountBalances.set(tenantId, newBalance);

    this.emit('billing:balance_recharged', { tenantId, amount, newBalance });
    return { tenantId, amount, newBalance, currency: this.currency };
  }

  /**
   * 获取账户余额
   */
  getBalance(tenantId) {
    return this.accountBalances.get(tenantId) || 0;
  }

  _calculateBillItems(config, usage) {
    const items = [];

    // 订阅费
    if (config.mode === BILLING_MODE.SUBSCRIPTION || config.mode === BILLING_MODE.HYBRID) {
      const planPrice = PRICING.subscription[config.plan]?.monthly || 0;
      if (planPrice > 0) {
        items.push({
          type: BILLING_ITEM_TYPE.SUBSCRIPTION_FEE,
          description: `${config.plan} 套餐月费`,
          quantity: 1,
          unitPrice: planPrice,
          amount: planPrice,
        });
      }
    }

    // 存储费（阶梯定价）
    const storageGB = (usage.usage?.storage_bytes || 0) / (1024 ** 3);
    if (storageGB > 0) {
      const storageCost = this._calculateTieredCost(storageGB, PRICING.storage.tiers);
      items.push({
        type: BILLING_ITEM_TYPE.STORAGE,
        description: '对象存储费用',
        quantity: Math.round(storageGB * 100) / 100,
        unit: 'GB/month',
        amount: Math.round(storageCost * 100) / 100,
      });
    }

    // API 调用费
    const apiCalls = usage.usage?.api_calls || 0;
    const billableCalls = Math.max(0, apiCalls - PRICING.api_calls.freeTier);
    if (billableCalls > 0) {
      const apiCost = (billableCalls / 1000) * PRICING.api_calls.price;
      items.push({
        type: BILLING_ITEM_TYPE.API_CALLS,
        description: 'API 调用费用（超出免费额度）',
        quantity: billableCalls,
        unit: 'calls',
        unitPrice: PRICING.api_calls.price / 1000,
        amount: Math.round(apiCost * 100) / 100,
      });
    }

    // 出口流量费
    const egressGB = (usage.usage?.egress_bytes || 0) / (1024 ** 3);
    const billableEgress = Math.max(0, egressGB - PRICING.egress.freeTier);
    if (billableEgress > 0) {
      const egressCost = billableEgress * PRICING.egress.price;
      items.push({
        type: BILLING_ITEM_TYPE.EGRESS,
        description: '出口流量费（超出免费额度）',
        quantity: Math.round(billableEgress * 100) / 100,
        unit: 'GB',
        unitPrice: PRICING.egress.price,
        amount: Math.round(egressCost * 100) / 100,
      });
    }

    // 计算费
    const computeHours = (usage.usage?.compute_core_seconds || 0) / 3600;
    if (computeHours > 0) {
      const computeCost = computeHours * PRICING.compute.price;
      items.push({
        type: BILLING_ITEM_TYPE.COMPUTE,
        description: '计算资源费用',
        quantity: Math.round(computeHours * 100) / 100,
        unit: 'core-hour',
        unitPrice: PRICING.compute.price,
        amount: Math.round(computeCost * 100) / 100,
      });
    }

    return items;
  }

  _calculateTieredCost(usage, tiers) {
    let cost = 0;
    let remaining = usage;
    let previousLimit = 0;

    for (const tier of tiers) {
      const tierUsage = Math.min(remaining, tier.upTo - previousLimit);
      if (tierUsage <= 0) break;
      cost += tierUsage * tier.price;
      remaining -= tierUsage;
      previousLimit = tier.upTo;
      if (remaining <= 0) break;
    }

    return cost;
  }

  _getPreviousMonth() {
    const now = new Date();
    return new Date(now.getFullYear(), now.getMonth() - 1, 1).toISOString().slice(0, 7);
  }

  _getDueDate() {
    const due = new Date();
    due.setDate(due.getDate() + 15);
    return due.toISOString();
  }

  _generateInvoiceNumber() {
    const now = new Date();
    const seq = String(this.invoices.size + 1).padStart(6, '0');
    return `MOX${now.getFullYear()}${String(now.getMonth() + 1).padStart(2, '0')}${seq}`;
  }

  /**
   * 获取租户账单列表
   */
  getInvoices(tenantId = null, status = null) {
    let invoices = Array.from(this.invoices.values());
    if (tenantId) invoices = invoices.filter(i => i.tenantId === tenantId);
    if (status) invoices = invoices.filter(i => i.status === status);
    return invoices.sort((a, b) => new Date(b.createdAt) - new Date(a.createdAt));
  }

  /**
   * 获取统计
   */
  getStats() {
    const allInvoices = Array.from(this.invoices.values());
    return {
      totalTenants: this.tenantBilling.size,
      totalInvoices: allInvoices.length,
      paidInvoices: allInvoices.filter(i => i.status === INVOICE_STATUS.PAID).length,
      pendingInvoices: allInvoices.filter(i => i.status === INVOICE_STATUS.PENDING).length,
      overdueInvoices: allInvoices.filter(i => i.status === INVOICE_STATUS.OVERDUE).length,
      totalRevenue: allInvoices.filter(i => i.status === INVOICE_STATUS.PAID).reduce((s, i) => s + i.total, 0),
      currency: this.currency,
      taxRate: this.taxRate,
      billingDay: this.billingDay,
      modes: Array.from(this.tenantBilling.values()).reduce((acc, t) => {
        acc[t.mode] = (acc[t.mode] || 0) + 1;
        return acc;
      }, {}),
    };
  }
}

module.exports = {
  BillingEngine,
  BILLING_MODE,
  INVOICE_STATUS,
  BILLING_ITEM_TYPE,
  PRICING,
};
