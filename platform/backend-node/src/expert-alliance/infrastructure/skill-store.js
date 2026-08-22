'use strict';

/**
 * 学习技能仓储（infrastructure 层 · 唯一 IO 边界）
 * ------------------------------------------------------------------
 * 持久化专家联盟学习技能至 data/alliance_learned_skills.json
 * （独立于 ai-integration-engine 的 learned_skills.json，互不覆写）
 *
 * 企业级约束：
 *   - 原子写：tmp + rename（崩溃不产生半写文件）
 *   - 容量上限：最近 MAX 条（防无限膨胀）
 *   - 读写失败静默降级（技能库是增强资产，不得阻断主流程）
 */
const fs = require('fs');
const path = require('path');

const DATA_DIR = path.join(__dirname, '..', '..', '..', 'data');
const SKILL_FILE = path.join(DATA_DIR, 'alliance_learned_skills.json');
const MAX_SKILLS = 200;

class SkillStore {
  constructor(options = {}) {
    this.filePath = options.filePath || SKILL_FILE;
    this.max = options.max || MAX_SKILLS;
    this.skills = new Map();
    this._load();
  }

  _load() {
    try {
      if (!fs.existsSync(this.filePath)) return;
      const raw = fs.readFileSync(this.filePath, 'utf8');
      const list = raw ? JSON.parse(raw) : [];
      for (const s of list) {
        if (s && s.key) this.skills.set(s.key, s);
      }
    } catch (_e) {
      // 损坏文件：丢弃旧库从空开始（技能可再生，不阻断主流程）
      this.skills = new Map();
    }
  }

  /** 原子写：tmp + rename */
  _persist() {
    try {
      fs.mkdirSync(path.dirname(this.filePath), { recursive: true });
      const list = this._evict();
      const tmp = this.filePath + '.tmp';
      fs.writeFileSync(tmp, JSON.stringify(list, null, 2), 'utf8');
      fs.renameSync(tmp, this.filePath);
    } catch (_e) { /* best-effort：技能持久化失败不阻断咨询主流程 */ }
  }

  /** 容量收敛：超出上限时按 count 升序淘汰弱技能 */
  _evict() {
    const list = Array.from(this.skills.values());
    if (list.length <= this.max) return list;
    list.sort((a, b) => (b.count || 1) - (a.count || 1));
    const kept = list.slice(0, this.max);
    this.skills = new Map(kept.map(s => [s.key, s]));
    return kept;
  }

  /** 透传给 domain 层 synthesizeSkills 做去重强化 */
  all() {
    return this.skills;
  }

  /** 持久化合并结果（domain 层已去重） */
  save(merged) {
    this.skills = merged instanceof Map ? merged : new Map(Object.entries(merged || {}));
    this._persist();
  }

  list(limit) {
    const list = Array.from(this.skills.values());
    return typeof limit === 'number' ? list.slice(0, limit) : list;
  }

  stats() {
    return {
      total: this.skills.size,
      intents: Array.from(new Set(Array.from(this.skills.values()).map(s => s.intent))).length,
      file: path.basename(this.filePath),
      max: this.max
    };
  }
}

module.exports = { SkillStore };
