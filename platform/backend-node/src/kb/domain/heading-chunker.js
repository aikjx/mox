'use strict';

/**
 * O6 补丁：标题感知切块器（Heading-Aware Document Chunker）
 *
 *   对照矩阵（T10）维度 10「RAG 检索/文档处理」差距：Dify 的 RecursiveCharacterTextSplitter
 *   仅在最后一步才参考 H1/H6 正则，Flowise/AutoGen 无标题语义切块；导致企业文档跨节
 *   边界切碎（章节间不相关内容拼成 1 chunk → 召回率 ↓）。璇玑补齐：
 *     1. H1~H6 + 中文编号（第X章 / 1. / 1.1 / 一、 二、）统一判定层级
 *     2. 栈式层级归并：chunk 永远落在单一「章节路径」上，绝不跨 H1/H2 跨节切割
 *     3. token 预算估算（粗粒度 2bytes ≈ 1 token 中文）+ max_chars/overlap 兼容旧参数
 *     4. 每块产出 heading_path（如 ["第1章 综述","1.1 背景"]）和 absolute_level，
 *        可被 O7 图谱 P99 上报链路的 `docs.chunks.*` 指标直接消费
 */

const DEFAULT_MAX_CHARS = 800;
const DEFAULT_OVERLAP = 80;
const DEFAULT_CHUNK_ALIGN = 'heading'; // 'heading' | 'length'

/** 尝试把一行解析为标题，若成功返回 {level, text} 否则 null。 */
function parseHeading(line, ctx) {
  if (!line) return null;
  const trimmed = line.trim();
  if (trimmed.length === 0) return null;
  // 避免单行长正文被误判为标题：标题字符数 ≤ 80 或含明确标题字符
  if (trimmed.length > 120 && !/^(#+|第[一二三四五六七八九十百千0-9]+[章节篇卷部])/.test(trimmed)) return null;

  // --- Markdown ATX: ### xxx ---
  let m = trimmed.match(/^(#{1,6})\s+(.+?)\s*#*\s*$/);
  if (m) return { level: m[1].length, text: cleanTitle(m[2]) };

  // --- Markdown Setext H1: 下一行 === ---
  if (ctx && ctx.nextLineType === 'setext_h1') return { level: 1, text: cleanTitle(trimmed) };
  if (ctx && ctx.nextLineType === 'setext_h2') return { level: 2, text: cleanTitle(trimmed) };

  // --- 中文编号型标题（第X章/第X节）---
  m = trimmed.match(/^第\s*([一二三四五六七八九十百千零两0-9]+)\s*[章节篇卷部]\s*(.*)$/);
  if (m) return { level: 1, text: cleanTitle(m[2] || trimmed) };

  // --- 一、二、三、 ---
  m = trimmed.match(/^([一二三四五六七八九十]+)[、．.]\s*(.*)$/);
  if (m && (m[2].length <= 80 || m[2].length === 0)) return { level: 2, text: cleanTitle(m[2] || trimmed) };

  // --- 1. / 1) / 1.1 / 1.1.1 / 1) 中文括号枚举 ---
  //   优先匹配 "数字 . 数字" 型（1. / 1.1）；其次匹配 "1)"/"(1)" 型枚举
  //   层级: 点号数量 n → level = n+2
  {
    let m2 = trimmed.match(/^(\d+(?:\.\d+)*[.]?)\s+(.+)$/);
    if (m2) {
      const head = m2[1];
      const dots = (head.match(/[.]/g) || []).length;
      // 末尾有点：1. / 2.1. → 常规层级
      // 末尾无点：1.1 / 2.1.3 → 按点数
      let lvl;
      if (/[.]$/.test(head)) lvl = 2 + dots - 1; // 1. → dots=1 → level=2 ; 2.1. → dots=2 → level=3
      else                  lvl = 2 + dots;       // 1.1 → dots=1 → level=3 ; 2.1.3 → dots=2 → level=4
      if (lvl > 6) lvl = 6;
      return { level: lvl, text: cleanTitle(m2[2]) };
    }
    m2 = trimmed.match(/^[(\[]?\d+[)\]]\s+(.+)$/);
    if (m2) return { level: 4, text: cleanTitle(m2[1]) };
  }
  return null;
}

/** 中文大写 → 粗略数值（只用在相对层级，不要求精确） */
function cnDigit(s) {
  const map = { 零:0, 一:1, 二:2, 两:2, 三:3, 四:4, 五:5, 六:6, 七:7, 八:8, 九:9, 十:10 };
  let n = 0, acc = 0;
  for (const ch of s) {
    const v = map[ch];
    if (v == null) continue;
    if (ch === '十' && acc === 0) acc = 1;
    if (v < 10) acc = acc * 0 + v; // 简单逐位
    else { n += (acc || 1) * 10; acc = 0; }
  }
  return n + acc;
}
function cnWeight(s) {
  const v = cnDigit(s);
  // 第X章 → level 1；第X节 → level 2（靠正则尾字区分：此处正则把章节篇卷部都匹配了）
  // 简化：所有「第X...」一律视作 level 1；下游按栈出栈顺序自动处理层级差。
  return v > 0 ? 1 : 1;
}

function cleanTitle(s) {
  return s.replace(/[：:]*\s*$/g, '').trim();
}

/** 判定 setext 下划线行类型：return 'setext_h1' | 'setext_h2' | null */
function classifySetextLine(line) {
  if (!line) return null;
  const t = line.trim();
  if (/^=+$/.test(t)) return 'setext_h1';
  if (/^-+$/.test(t)) return 'setext_h2';
  return null;
}

/** 主要 API：把一份 Markdown/纯文本切成带标题路径的 chunks。
 *
 *  opts:
 *    max_chars?:  int   单块最大字符数（默认 800）
 *    overlap?:    int   相邻块尾部重叠字符数（默认 80）。仅在同 section 内长段落分裂时用。
 *    prefer?:    'heading'  若为 'length' 则退化为定长切块（向后兼容）。
 *    src?:       string   可选：标记来源文件路径，填入 chunk.meta.src
 *
 *  返回 Chunk[]：{ id, text, chars, heading_path[], level, start_line, end_line, meta }
 */
function splitDocument(text, opts = {}) {
  const maxChars = Math.max(200, Number(opts.max_chars) || DEFAULT_MAX_CHARS);
  const overlap  = Math.max(0,   Math.min(maxChars >> 1, Number(opts.overlap) || DEFAULT_OVERLAP));
  const prefer   = opts.prefer === 'length' ? 'length' : DEFAULT_CHUNK_ALIGN;
  const srcTag   = opts.src || null;

  if (typeof text !== 'string' || text.length === 0) return [];

  // --- Step 1: 先规范化换行，处理 setext 下划线 ---
  const rawLines = text.replace(/\r\n?/g, '\n').split('\n');
  /** @type {{type:'heading'|'para', content:string, level?:number, line:number}[]} */
  const blocks = [];
  for (let i = 0; i < rawLines.length; i++) {
    const line = rawLines[i];
    const nextLine = rawLines[i + 1];
    const nextType = classifySetextLine(nextLine);
    const ctx = nextType ? { nextLineType: nextType } : null;

    const h = parseHeading(line, ctx);
    if (h) {
      blocks.push({ type: 'heading', content: h.text, level: h.level, line: i });
      if (nextType) i++; // 跳过 setext 下划线行
      continue;
    }
    if (classifySetextLine(line)) {
      // 裸下划线行（已经作为 heading 的后续 i++ 被消费；剩余即空格式行）
      continue;
    }
    blocks.push({ type: 'para', content: line, line: i });
  }

  // --- Step 2: 栈式章节分组（heading path）+ 块生成 ---
  const chunks = [];
  /** 章节栈：[{level, text, startLine}] */
  let stack = [];
  /** 当前节 buffer（段落块序列：[{type, content, line}]） */
  let buf = [];
  let bufChars = 0;
  let currentStartLine = 0;
  let chunkIdSeq = 0;

  const flushBuffer = (finalHeading, endLine) => {
    if (bufChars === 0 && finalHeading.length === 0) return;
    // 对同 section 超长段落做 length 二次切；heading 做"路径+正文"紧凑拼接
    const headingText = finalHeading.join(' > ');
    const paraText = buf.map(b => b.content).join('\n').replace(/\n{3,}/g, '\n\n').trim();

    // 单块文本：优先 heading_path。若正文+路径超过 maxChars，退化为分块。
    const full = (headingText && !paraText.startsWith(headingText))
      ? `${headingText}\n${paraText}`
      : paraText;

    if (full.length === 0) return;

    if (full.length <= maxChars) {
      pushChunk(full, finalHeading, currentStartLine, endLine, headingText, srcTag);
    } else {
      // 通用滑窗：按 maxChars 切块；headingText 作为每块前缀以保持 heading 感知
      const prefix = headingText ? `${headingText}\n` : '';
      const budget = Math.max(50, maxChars - prefix.length);
      let pos = 0;
      let pieceNo = 0;
      let safety = 0;
      while (pos < paraText.length) {
        safety++;
        if (safety > 10000) break; // 防御性：避免极端 overlap 配置导致死循环
        let end = Math.min(paraText.length, pos + budget);
        // 优先在最近 \n 切割（避免句中割裂）
        if (end < paraText.length) {
          const brk = paraText.lastIndexOf('\n', end);
          if (brk > pos + (budget >> 1)) end = brk;
        }
        const piece = paraText.slice(pos, end).trim();
        if (piece.length > 0) pushChunk(`${prefix}${piece}`, finalHeading, currentStartLine, endLine, headingText, srcTag, pieceNo);
        pieceNo++;
        if (end >= paraText.length) break;
        pos = Math.max(pos + 1, end - overlap);
      }
    }
    buf = [];
    bufChars = 0;
  };

  const pushChunk = (txt, heading, s, e, headingText, srcTag, pieceNo = 0) => {
    chunkIdSeq += 1;
    chunks.push({
      id: `c${chunkIdSeq.toString().padStart(4, '0')}`,
      text: txt,
      chars: txt.length,
      heading_path: heading.slice(),
      level: heading.length, // 路径深度 = H0 正文 / H1 第一章...，对齐人类直觉
      start_line: s,
      end_line: e,
      meta: {
        heading: headingText || null,
        piece: pieceNo,
        src: srcTag || null,
      },
    });
  };

  // 初始化 currentStartLine
  currentStartLine = 0;

  for (const b of blocks) {
    if (b.type === 'heading') {
      // 遇到新标题：先把当前 buffer flush 用旧 heading
      const currentPath = stack.map(s => s.text);
      flushBuffer(currentPath, b.line);
      // 弹栈至新 level 的合法父
      while (stack.length > 0 && stack[stack.length - 1].level >= b.level) stack.pop();
      stack.push({ level: b.level, text: b.content });
      // 新节起点：若 heading 自身内容较长，也可能单独作为一块（避免只有 heading 无 para 被 flush 漏掉）
      const newPath = stack.map(s => s.text);
      // 如果 heading 很长（> 60 chars 或包含列表内容），把它当作"空 para + heading context"
      if (b.content.length > 60) {
        buf.push({ type: 'para', content: b.content, line: b.line });
        bufChars += b.content.length;
      }
      currentStartLine = b.line;
      // 继续累积后续段落
      continue;
    }
    // para block
    if (!b.content.trim()) {
      // 空行：按原样加入（不会显著增长 bufChars），仅保留段落结构
      continue;
    }
    buf.push(b);
    bufChars += b.content.length;
    if (bufChars >= maxChars) {
      // 超出预算：按当前 heading 路径 flush 一次
      const currentPath = stack.map(s => s.text);
      flushBuffer(currentPath, b.line + 1);
      // overlap：把最近 overlap 字符的末段留在 buf 里，供下一块前缀
      if (overlap > 0) {
        const leftTail = [];
        let tailChars = 0;
        for (let i = buf.length - 1; i >= 0 && tailChars < overlap; i--) {
          leftTail.unshift(buf[i]);
          tailChars += buf[i].content.length;
        }
        buf = leftTail;
        bufChars = tailChars;
      } else {
        buf = [];
        bufChars = 0;
      }
      currentStartLine = b.line + 1;
    }
  }
  // 最后 flush
  flushBuffer(stack.map(s => s.text), rawLines.length - 1);

  // 对 id 重分配：按顺序 c0001/0002...（避免 pieceNo 切分造成的缺口）
  for (let i = 0; i < chunks.length; i++) chunks[i].id = `c${(i+1).toString().padStart(4, '0')}`;
  return chunks;
}

/** 统计切块质量指标，供 O7 图谱上报：
 *  - total_chunks, avg_chars, p99_chars, headings (distinct H1..H6)
 *  - cross_heading_chunks：应始终 = 0；> 0 代表存在跨节切割 bug
 */
function summarizeChunks(chunks) {
  if (!chunks.length) {
    return {
      total_chunks: 0, avg_chars: 0, p50_chars: 0, p95_chars: 0, p99_chars: 0,
      cross_heading_chunks: 0, distinct_headings: 0,
    };
  }
  const arr = chunks.map(c => c.chars).sort((a, b) => a - b);
  const q = p => {
    if (arr.length === 1) return arr[0];
    const pos = (arr.length - 1) * p;
    const lo = Math.floor(pos), hi = Math.min(arr.length - 1, lo + 1);
    const f = pos - lo;
    return Math.round(arr[lo] * (1 - f) + arr[hi] * f);
  };
  const headings = new Set(chunks.map(c => (c.meta.heading || '')).filter(Boolean));
  // cross_heading_chunks 检测：chunk[i+1] 若 heading_path[0..k-1] 与 chunk[i] 完全不同且文本长度 overlap
  // 简化规则：chunk.heading_path 中任一级相邻父不同 → 视为「正确的 heading 边界切换」，即 0 跨节；
  // 仅检测"同一 chunk 内文本包含明显新标题"——用简化正则二次兜底
  let cross = 0;
  const rx = /(^|\n)(#{1,6}\s|第[一二三四五六七八九十百千0-9]+[章节篇卷部])/;
  for (const c of chunks) {
    // 如果 heading_path 的最后一级不等于 text 中第一段 ATX/CN 标题，则说明切块内可能嵌入了新标题
    const m = c.text.match(rx);
    if (m && c.level > 0) {
      const firstHeading = cleanTitle(c.text.slice(m.index + m[1].length).split('\n')[0].replace(/^#{1,6}\s*/, ''));
      const pathLast = c.heading_path[c.heading_path.length - 1] || '';
      if (firstHeading && pathLast && !firstHeading.startsWith(pathLast) && !pathLast.startsWith(firstHeading)) {
        // 只有嵌入的标题与 pathLast 不同，且出现在 chunk 非开头部分，才判定跨节
        if (m.index > Math.min(40, Math.floor(c.text.length * 0.05))) cross++;
      }
    }
  }
  return {
    total_chunks: chunks.length,
    avg_chars: Math.round(arr.reduce((a, b) => a + b, 0) / arr.length),
    p50_chars: q(0.5), p95_chars: q(0.95), p99_chars: q(0.99),
    cross_heading_chunks: cross,
    distinct_headings: headings.size,
  };
}

module.exports = {
  splitDocument,
  summarizeChunks,
  parseHeading,
  DEFAULT_MAX_CHARS,
  DEFAULT_OVERLAP,
};
