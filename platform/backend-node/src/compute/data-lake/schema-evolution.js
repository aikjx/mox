'use strict';

/**
 * MOX Enterprise · Iceberg Schema 演进管理器
 * ============================================
 * 管理数据湖表的 Schema 变更，支持：
 *  - 加列（ADD COLUMN）
 *  - 删列（DROP COLUMN）
 *  - 改列名（RENAME COLUMN）
 *  - 改类型（ALTER COLUMN TYPE）
 *  - 改注释（SET COLUMN COMMENT）
 *  - 加分区（ADD PARTITION FIELD）
 *  - 删分区（DROP PARTITION FIELD）
 *
 * Iceberg Schema 演进优势：
 *  - 不需要重写历史数据文件
 *  - 旧数据文件按旧 Schema 读取，新数据按新 Schema 写入
 *  - 读取时自动做列映射和类型转换
 *  - 支持回滚到任意历史 Schema 版本
 */

const { EventEmitter } = require('events');
const crypto = require('crypto');

// ─── Schema 变更类型 ───
const SCHEMA_CHANGE_TYPE = {
  ADD_COLUMN: 'add_column',
  DROP_COLUMN: 'drop_column',
  RENAME_COLUMN: 'rename_column',
  ALTER_COLUMN_TYPE: 'alter_column_type',
  SET_COLUMN_COMMENT: 'set_column_comment',
  ADD_PARTITION_FIELD: 'add_partition_field',
  DROP_PARTITION_FIELD: 'drop_partition_field',
  SET_PROPERTIES: 'set_properties',
};

// ─── 类型兼容性矩阵（哪些类型可以安全转换） ───
const TYPE_COMPATIBILITY = {
  'int': ['bigint', 'double', 'decimal', 'string'],
  'bigint': ['double', 'decimal', 'string'],
  'float': ['double', 'decimal', 'string'],
  'double': ['decimal', 'string'],
  'decimal': ['string'],
  'string': [], // string 不能安全转换为其他类型
  'date': ['timestamp', 'string'],
  'timestamp': ['string'],
  'boolean': ['string'],
};

class SchemaEvolutionManager extends EventEmitter {
  /**
   * @param {object} options
   * @param {string} options.warehousePath 数据湖根路径
   * @param {object} options.metadataStore 元数据存储
   * @param {boolean} options.autoValidate  自动验证变更安全性（默认 true）
   * @param {boolean} options.dryRunDefault  默认 dry-run（默认 false）
   */
  constructor(options = {}) {
    super();
    this.warehousePath = options.warehousePath || './data-lake';
    this.metadataStore = options.metadataStore;
    this.autoValidate = options.autoValidate !== false;
    this.dryRunDefault = options.dryRunDefault || false;

    // Schema 变更历史
    this.changeHistory = [];
    this._changeCount = 0;
  }

  /**
   * 加列
   * @param {string} tableName  表名
   * @param {object} column     列定义 { name, type, required, comment, defaultValue }
   * @param {object} [options]
   */
  async addColumn(tableName, column, options = {}) {
    return this._applyChange(tableName, {
      type: SCHEMA_CHANGE_TYPE.ADD_COLUMN,
      column,
      ...options,
    });
  }

  /**
   * 删列
   */
  async dropColumn(tableName, columnName, options = {}) {
    return this._applyChange(tableName, {
      type: SCHEMA_CHANGE_TYPE.DROP_COLUMN,
      columnName,
      ...options,
    });
  }

  /**
   * 改列名
   */
  async renameColumn(tableName, oldName, newName, options = {}) {
    return this._applyChange(tableName, {
      type: SCHEMA_CHANGE_TYPE.RENAME_COLUMN,
      oldName,
      newName,
      ...options,
    });
  }

  /**
   * 改列类型
   */
  async alterColumnType(tableName, columnName, newType, options = {}) {
    return this._applyChange(tableName, {
      type: SCHEMA_CHANGE_TYPE.ALTER_COLUMN_TYPE,
      columnName,
      newType,
      ...options,
    });
  }

  /**
   * 设置列注释
   */
  async setColumnComment(tableName, columnName, comment, options = {}) {
    return this._applyChange(tableName, {
      type: SCHEMA_CHANGE_TYPE.SET_COLUMN_COMMENT,
      columnName,
      comment,
      ...options,
    });
  }

  /**
   * 加分区字段
   */
  async addPartitionField(tableName, fieldName, transform, options = {}) {
    return this._applyChange(tableName, {
      type: SCHEMA_CHANGE_TYPE.ADD_PARTITION_FIELD,
      fieldName,
      transform,
      ...options,
    });
  }

  /**
   * 删分区字段
   */
  async dropPartitionField(tableName, fieldName, options = {}) {
    return this._applyChange(tableName, {
      type: SCHEMA_CHANGE_TYPE.DROP_PARTITION_FIELD,
      fieldName,
      ...options,
    });
  }

  async _applyChange(tableName, change) {
    const changeId = `sch-${crypto.randomBytes(6).toString('hex')}`;
    const dryRun = change.dryRun !== undefined ? change.dryRun : this.dryRunDefault;
    const startTime = Date.now();

    this.emit('schema:change:start', { changeId, tableName, change, dryRun });

    try {
      // 1. 获取当前 Schema
      const currentSchema = await this._getCurrentSchema(tableName);

      // 2. 验证变更安全性
      if (this.autoValidate) {
        const validation = this._validateChange(currentSchema, change);
        if (!validation.safe) {
          throw new Error(`Schema 变更不安全: ${validation.reason}`);
        }
      }

      // 3. 生成新 Schema
      const newSchema = this._applyChangeToSchema(currentSchema, change);

      // 4. 应用变更（或 dry-run）
      if (!dryRun) {
        await this._commitSchemaChange(tableName, newSchema, change, changeId);
      }

      const result = {
        changeId,
        tableName,
        changeType: change.type,
        dryRun,
        oldSchema: currentSchema,
        newSchema,
        applied: !dryRun,
        durationMs: Date.now() - startTime,
        timestamp: new Date().toISOString(),
      };

      this.changeHistory.push(result);
      this._changeCount++;

      this.emit('schema:change:completed', result);
      return result;

    } catch (err) {
      this.emit('schema:change:failed', { changeId, tableName, error: err.message });
      throw err;
    }
  }

  _validateChange(currentSchema, change) {
    switch (change.type) {
      case SCHEMA_CHANGE_TYPE.ADD_COLUMN:
        // 加列总是安全的（如果有默认值或允许 null）
        if (change.column.required && !change.column.defaultValue) {
          return { safe: false, reason: '加必填列必须提供默认值' };
        }
        if (currentSchema.columns.some(c => c.name === change.column.name)) {
          return { safe: false, reason: `列 ${change.column.name} 已存在` };
        }
        return { safe: true };

      case SCHEMA_CHANGE_TYPE.DROP_COLUMN:
        // 删列是破坏性变更
        return { safe: true, warning: '删列是破坏性变更，历史数据中的该列将被忽略' };

      case SCHEMA_CHANGE_TYPE.RENAME_COLUMN:
        if (!currentSchema.columns.some(c => c.name === change.oldName)) {
          return { safe: false, reason: `列 ${change.oldName} 不存在` };
        }
        if (currentSchema.columns.some(c => c.name === change.newName)) {
          return { safe: false, reason: `列名 ${change.newName} 已被占用` };
        }
        return { safe: true };

      case SCHEMA_CHANGE_TYPE.ALTER_COLUMN_TYPE:
        const col = currentSchema.columns.find(c => c.name === change.columnName);
        if (!col) return { safe: false, reason: `列 ${change.columnName} 不存在` };
        const compatible = TYPE_COMPATIBILITY[col.type?.toLowerCase()] || [];
        if (!compatible.includes(change.newType.toLowerCase())) {
          return { safe: false, reason: `类型 ${col.type} 不能安全转换为 ${change.newType}` };
        }
        return { safe: true };

      case SCHEMA_CHANGE_TYPE.ADD_PARTITION_FIELD:
        return { safe: true, warning: '加分区字段只影响新数据，历史数据不会重新分区' };

      case SCHEMA_CHANGE_TYPE.DROP_PARTITION_FIELD:
        return { safe: true, warning: '删分区字段是破坏性变更' };

      default:
        return { safe: true };
    }
  }

  _applyChangeToSchema(currentSchema, change) {
    const newSchema = JSON.parse(JSON.stringify(currentSchema));
    newSchema.schemaVersion = (currentSchema.schemaVersion || 0) + 1;
    newSchema.columns = newSchema.columns || [];

    switch (change.type) {
      case SCHEMA_CHANGE_TYPE.ADD_COLUMN:
        newSchema.columns.push({
          id: newSchema.columns.length + 1,
          name: change.column.name,
          type: change.column.type,
          required: change.column.required || false,
          comment: change.column.comment || null,
          defaultValue: change.column.defaultValue || null,
        });
        break;

      case SCHEMA_CHANGE_TYPE.DROP_COLUMN:
        newSchema.columns = newSchema.columns.filter(c => c.name !== change.columnName);
        break;

      case SCHEMA_CHANGE_TYPE.RENAME_COLUMN:
        const col = newSchema.columns.find(c => c.name === change.oldName);
        if (col) col.name = change.newName;
        break;

      case SCHEMA_CHANGE_TYPE.ALTER_COLUMN_TYPE:
        const alterCol = newSchema.columns.find(c => c.name === change.columnName);
        if (alterCol) alterCol.type = change.newType;
        break;

      case SCHEMA_CHANGE_TYPE.SET_COLUMN_COMMENT:
        const commentCol = newSchema.columns.find(c => c.name === change.columnName);
        if (commentCol) commentCol.comment = change.comment;
        break;
    }

    return newSchema;
  }

  async _getCurrentSchema(tableName) {
    // 从元数据存储获取当前 Schema
    if (this.metadataStore) {
      return this.metadataStore.getTableSchema(tableName);
    }
    return { tableName, schemaVersion: 0, columns: [] };
  }

  async _commitSchemaChange(tableName, newSchema, change, changeId) {
    // 提交 Schema 变更到元数据存储
    if (this.metadataStore) {
      await this.metadataStore.updateTableSchema(tableName, newSchema, {
        changeId,
        changeType: change.type,
        appliedAt: new Date().toISOString(),
      });
    }
  }

  /**
   * 获取 Schema 变更历史
   */
  getChangeHistory(tableName = null, limit = 50) {
    let history = this.changeHistory;
    if (tableName) history = history.filter(h => h.tableName === tableName);
    return history.slice(-limit).reverse();
  }

  /**
   * 回滚到指定 Schema 版本
   */
  async rollback(tableName, schemaVersion) {
    const history = this.getChangeHistory(tableName);
    const target = history.find(h => h.newSchema.schemaVersion === schemaVersion);
    if (!target) throw new Error(`未找到 Schema 版本 ${schemaVersion}`);

    return this._applyChange(tableName, {
      type: 'rollback',
      targetSchema: target.oldSchema,
      dryRun: false,
    });
  }

  /**
   * 获取统计
   */
  getStats() {
    return {
      totalChanges: this._changeCount,
      changeHistorySize: this.changeHistory.length,
      autoValidate: this.autoValidate,
      dryRunDefault: this.dryRunDefault,
      changesByType: this.changeHistory.reduce((acc, h) => {
        acc[h.changeType] = (acc[h.changeType] || 0) + 1;
        return acc;
      }, {}),
    };
  }
}

module.exports = {
  SchemaEvolutionManager,
  SCHEMA_CHANGE_TYPE,
  TYPE_COMPATIBILITY,
};
