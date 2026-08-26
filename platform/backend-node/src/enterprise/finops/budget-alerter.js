'use strict';

/**
 * MOX Enterprise · 预算告警器
 * ============================
 * 多级预算管理与实时告警
 *
 * 预算层级：
 *  - 全局预算（月度/年度）
 *  - 部门预算
 *  - 租户预算
 *  - 服务预算
 *  - 资源类型预算
 *
 * 告警阈值：
 *  - 50%：提醒（信息级）
 *  - 80%：警告（P2）
 *  - 90%：严重（P1）
 *  - 100%：超预算（P0，触发限流/降级）
 */

const { EventEmitter } = require('events');
const crypto = require('crypto');

// ─── 预算周期 ───
const BUDGET_PERIOD = {
  DAILY: 'daily',
  WEEKLY: 'weekly',
  MONTHLY: 'monthly',
  QUARTERLY: 'quarterly',
  YEARLY: 'yearly',
};

// ─── 告警级别 ───
const ALERT_LEVEL = {
  INFO: 'info',       // 50%
  WARNING: 'warning', // 80%
  CRITICAL: 'critical', // 90%
  BREACH: 'breach',   // 100%+
};

// ─── 超预算动作 ───
const OVER_BUDGET_ACTION = {
  NOTIFY: 'notify',           // 仅通知
  RESTRICT: 'restrict',       // 限制非关键操作
  THROTTLE: 'throttle',       // 限流
  SHUTDOWN_NON_CRITICAL: 'shutdown_non_critical', // 关闭非关键服务
  FULL_SHUTDOWN: 'full_shutdown', // 全量关闭（极端）
};

class BudgetAlerter extends EventEmitter {
  /**
   * @param {object} options
   * @param {object} options.costCollector 成本采集器实例
   * @param {object} options.notifier      通知器实例
   * @param {number} options.checkIntervalMs 检查间隔（默认 1 小时）
   * @param {object} options.thresholds    自定义阈值 { info: 0.5, warning: 0.8, ... }
   */
  constructor(options = {}) {
    super();
    this.costCollector = options.costCollector;
    this.notifier = options.notifier;
    this.checkIntervalMs = options.checkIntervalMs || 3600000;
    this.thresholds = options.thresholds || {
      info: 0.5,
      warning: 0.8,
      critical: 0.9,
      breach: 1.0,
    };

    // 预算定义：budgetId -> { ... }
    this.budgets = new Map();

    // 告警历史
    this.alertHistory = [];
    this._alertCount = 0;

    // 当前告警状态（避免重复告警）
    this.activeAlerts = new Map(); // budgetId -> level

    this._startCheckLoop();
  }

  /**
   * 创建预算
   * @param {object} budget
   * @param {string} budget.name        预算名称
   * @param {string} budget.scope       预算范围（global/department/tenant/service/resource_type）
   * @param {string} budget.scopeId     范围 ID（租户 ID/服务名等）
   * @param {number} budget.amount      预算金额
   * @param {string} budget.currency    货币（默认 CNY）
   * @param {string} budget.period      周期（daily/weekly/monthly/...）
   * @param {string} budget.overBudgetAction 超预算动作
   * @param {string[]} budget.alertChannels 告警渠道
   */
  createBudget(budget) {
    const budgetId = `bud-${crypto.randomBytes(6).toString('hex')}`;
    const fullBudget = {
      budgetId,
      name: budget.name,
      scope: budget.scope || 'global',
      scopeId: budget.scopeId || null,
      amount: budget.amount,
      currency: budget.currency || 'CNY',
      period: budget.period || BUDGET_PERIOD.MONTHLY,
      overBudgetAction: budget.overBudgetAction || OVER_BUDGET_ACTION.NOTIFY,
      alertChannels: budget.alertChannels || ['email'],
      createdAt: new Date().toISOString(),
      status: 'active',
    };

    this.budgets.set(budgetId, fullBudget);
    this.emit('budget:created', fullBudget);
    return fullBudget;
  }

  /**
   * 更新预算
   */
  updateBudget(budgetId, updates) {
    const budget = this.budgets.get(budgetId);
    if (!budget) throw new Error(`预算不存在: ${budgetId}`);
    Object.assign(budget, updates, { updatedAt: new Date().toISOString() });
    this.emit('budget:updated', budget);
    return budget;
  }

  /**
   * 删除预算
   */
  deleteBudget(budgetId) {
    const budget = this.budgets.get(budgetId);
    if (!budget) return false;
    budget.status = 'deleted';
    budget.deletedAt = new Date().toISOString();
    this.budgets.delete(budgetId);
    this.emit('budget:deleted', { budgetId });
    return true;
  }

  /**
   * 检查所有预算
   */
  async checkAllBudgets() {
    const results = [];
    for (const [budgetId, budget] of this.budgets) {
      if (budget.status !== 'active') continue;
      try {
        const result = await this._checkBudget(budget);
        results.push(result);
      } catch (err) {
        this.emit('budget:check_error', { budgetId, error: err.message });
      }
    }
    return results;
  }

  async _checkBudget(budget) {
    // 获取当前周期已花费金额
    const spent = await this._getSpentAmount(budget);
    const percentage = spent / budget.amount;
    const level = this._getAlertLevel(percentage);

    const result = {
      budgetId: budget.budgetId,
      budgetName: budget.name,
      budgetAmount: budget.amount,
      spent,
      remaining: budget.amount - spent,
      percentage,
      level,
      currency: budget.currency,
      period: budget.period,
      checkedAt: new Date().toISOString(),
    };

    // 检查是否需要告警（级别变化时才告警，避免重复）
    const previousLevel = this.activeAlerts.get(budget.budgetId);
    if (level && level !== previousLevel) {
      this.activeAlerts.set(budget.budgetId, level);
      await this._triggerAlert(budget, result);
    } else if (!level && previousLevel) {
      // 预算恢复正常
      this.activeAlerts.delete(budget.budgetId);
      this.emit('budget:recovered', result);
    }

    // 超预算动作
    if (level === ALERT_LEVEL.BREACH && budget.overBudgetAction !== OVER_BUDGET_ACTION.NOTIFY) {
      await this._executeOverBudgetAction(budget, result);
    }

    return result;
  }

  async _getSpentAmount(budget) {
    // 从成本采集器获取当前周期花费
    // const summary = await this.costCollector.getCostSummary(
    //   budget.scope === 'global' ? 'service' : budget.scope,
    //   this._getPeriodStart(budget.period),
    //   new Date()
    // );
    // return summary.summary.reduce((s, r) => s + r.total_cost, 0);
    return 0;
  }

  _getAlertLevel(percentage) {
    if (percentage >= this.thresholds.breach) return ALERT_LEVEL.BREACH;
    if (percentage >= this.thresholds.critical) return ALERT_LEVEL.CRITICAL;
    if (percentage >= this.thresholds.warning) return ALERT_LEVEL.WARNING;
    if (percentage >= this.thresholds.info) return ALERT_LEVEL.INFO;
    return null;
  }

  async _triggerAlert(budget, result) {
    const alertId = `alert-${crypto.randomBytes(6).toString('hex')}`;
    const alert = {
      alertId,
      budgetId: budget.budgetId,
      budgetName: budget.name,
      level: result.level,
      percentage: result.percentage,
      spent: result.spent,
      budgetAmount: result.budgetAmount,
      message: this._formatAlertMessage(budget, result),
      channels: budget.alertChannels,
      createdAt: new Date().toISOString(),
      status: 'sent',
    };

    this.alertHistory.push(alert);
    this._alertCount++;

    // 发送通知
    if (this.notifier) {
      for (const channel of budget.alertChannels) {
        try {
          await this.notifier.send(channel, alert);
        } catch (err) {
          this.emit('alert:send_error', { channel, error: err.message });
        }
      }
    }

    this.emit('budget:alert', alert);
    return alert;
  }

  _formatAlertMessage(budget, result) {
    const pct = (result.percentage * 100).toFixed(1);
    const levelText = {
      info: '提醒',
      warning: '警告',
      critical: '严重',
      breach: '超预算',
    }[result.level];

    return `【${levelText}】预算 ${budget.name} 已使用 ${pct}%（${result.spent.toFixed(2)}/${result.budgetAmount} ${result.currency}），剩余 ${result.remaining.toFixed(2)} ${result.currency}`;
  }

  async _executeOverBudgetAction(budget, result) {
    this.emit('budget:over_budget_action', {
      budgetId: budget.budgetId,
      action: budget.overBudgetAction,
      result,
    });

    switch (budget.overBudgetAction) {
      case OVER_BUDGET_ACTION.RESTRICT:
        // 限制非关键操作
        break;
      case OVER_BUDGET_ACTION.THROTTLE:
        // 限流
        break;
      case OVER_BUDGET_ACTION.SHUTDOWN_NON_CRITICAL:
        // 关闭非关键服务
        break;
      case OVER_BUDGET_ACTION.FULL_SHUTDOWN:
        // 全量关闭（需要人工确认）
        break;
    }
  }

  _getPeriodStart(period) {
    const now = new Date();
    switch (period) {
      case BUDGET_PERIOD.DAILY:
        return new Date(now.getFullYear(), now.getMonth(), now.getDate());
      case BUDGET_PERIOD.WEEKLY:
        const day = now.getDay() || 7;
        return new Date(now.getFullYear(), now.getMonth(), now.getDate() - day + 1);
      case BUDGET_PERIOD.MONTHLY:
        return new Date(now.getFullYear(), now.getMonth(), 1);
      case BUDGET_PERIOD.QUARTERLY:
        const quarter = Math.floor(now.getMonth() / 3);
        return new Date(now.getFullYear(), quarter * 3, 1);
      case BUDGET_PERIOD.YEARLY:
        return new Date(now.getFullYear(), 0, 1);
      default:
        return new Date(now.getFullYear(), now.getMonth(), 1);
    }
  }

  _startCheckLoop() {
    setInterval(() => this.checkAllBudgets().catch(err => {
      this.emit('check-loop:error', { error: err.message });
    }), this.checkIntervalMs);
  }

  /**
   * 获取预算列表
   */
  listBudgets(scope = null) {
    let budgets = Array.from(this.budgets.values());
    if (scope) budgets = budgets.filter(b => b.scope === scope);
    return budgets;
  }

  /**
   * 获取告警历史
   */
  getAlertHistory(limit = 50, level = null) {
    let history = this.alertHistory;
    if (level) history = history.filter(a => a.level === level);
    return history.slice(-limit).reverse();
  }

  /**
   * 获取统计
   */
  getStats() {
    return {
      totalBudgets: this.budgets.size,
      activeBudgets: Array.from(this.budgets.values()).filter(b => b.status === 'active').length,
      totalAlerts: this._alertCount,
      activeAlerts: this.activeAlerts.size,
      alertsByLevel: this.alertHistory.reduce((acc, a) => {
        acc[a.level] = (acc[a.level] || 0) + 1;
        return acc;
      }, {}),
      checkIntervalMs: this.checkIntervalMs,
      thresholds: this.thresholds,
    };
  }
}

module.exports = {
  BudgetAlerter,
  BUDGET_PERIOD,
  ALERT_LEVEL,
  OVER_BUDGET_ACTION,
};
