const ALLIANCE_BASE = import.meta.env?.VITE_API_BASE ?? "";
function getBearerToken() {
  const env = import.meta.env ?? {};
  return env.VITE_OUS_API_TOKEN || env.OUS_API_TOKEN || null;
}
function authHeaders(extra = {}) {
  const h = { ...extra };
  const t = getBearerToken();
  if (t) h["Authorization"] = `Bearer ${t}`;
  return h;
}
async function getAllianceCapabilities() {
  const r = await fetch(`${ALLIANCE_BASE}/ai/engine/alliance/capabilities`, {
    method: "GET",
    headers: authHeaders({ "Accept": "application/json" })
  });
  if (!r.ok) throw new Error(`alliance/capabilities HTTP ${r.status}`);
  return await r.json();
}
async function runAllianceFullSSE(req, onFrame) {
  const resp = await fetch(`${ALLIANCE_BASE}/ai/engine/alliance/full`, {
    method: "POST",
    headers: authHeaders({
      "Content-Type": "application/json",
      "Accept": "text/event-stream"
    }),
    body: JSON.stringify(req)
  });
  if (!resp.ok) {
    throw new Error(`alliance/full HTTP ${resp.status}`);
  }
  const reader = resp.body?.getReader();
  if (!reader) throw new Error("No readable stream");
  const decoder = new TextDecoder("utf-8");
  let buffer = "";
  let lastTraceId = "";
  let currentEventName = "";
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    let idx;
    while ((idx = buffer.indexOf("\n\n")) >= 0) {
      const rawEvent = buffer.slice(0, idx);
      buffer = buffer.slice(idx + 2);
      let data = "";
      for (const line of rawEvent.split("\n")) {
        if (line.startsWith("event:")) currentEventName = line.slice(6).trim();
        else if (line.startsWith("data:")) data += line.slice(5).trimStart();
      }
      if (!data) continue;
      if (data === "[DONE]") {
        reader.releaseLock();
        return lastTraceId;
      }
      try {
        const frame = JSON.parse(data);
        if (frame.trace_id) lastTraceId = frame.trace_id;
        const cont = onFrame(frame);
        if (cont === false) {
          reader.releaseLock();
          return lastTraceId;
        }
      } catch (e) {
        console.warn("[alliance.sse] frame parse failed:", data, e);
      }
    }
  }
  reader.releaseLock();
  return lastTraceId;
}
async function getVoiceHealth() {
  try {
    const r = await fetch(`${ALLIANCE_BASE}/voice/health`, { method: "GET", headers: authHeaders({ Accept: "application/json" }) });
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
    return await r.json();
  } catch (e) {
    return {
      ok: false,
      upstream_unreachable: true,
      fallback_action: "AC-22 \u4E09\u5C42\u56DE\u9000\uFF08\u8FDE\u4E0D\u4E0A Rust \u7F51\u5173\uFF09\uFF1A\u76F4\u63A5 browser Web Speech Synthesis",
      tts: {
        ready: false,
        active: "browser_tts",
        engines: [
          { name: "cosyvoice2", available: false, license: "Apache-2.0" },
          { name: "fish_s2_pro", available: false, license: "Research", note: "\u9ED8\u8BA4\u7981\u7528\uFF0CResearch License" }
        ]
      }
    };
  }
}
async function allianceRegisterExpert(payload) {
  const r = await fetch(`${ALLIANCE_BASE}/experts`, {
    method: "POST",
    headers: authHeaders({
      "Content-Type": "application/json",
      "Accept": "application/json"
    }),
    body: JSON.stringify(payload)
  });
  if (!r.ok) throw new Error(`experts/register HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) {
    if (!data.success) throw new Error(data.error || data.message || "\u6CE8\u518C\u5931\u8D25");
    return data.data;
  }
  return data;
}
async function allianceGetExperts(params = {}) {
  const query = new URLSearchParams();
  if (params.page != null) query.set("page", String(params.page));
  if (params.page_size != null) query.set("page_size", String(params.page_size));
  if (params.domain) query.set("domain", params.domain);
  if (params.status) query.set("status", params.status);
  if (params.keyword) query.set("keyword", params.keyword);
  if (params.tags?.length) query.set("tags", params.tags.join(","));
  if (params.enterprise_id) query.set("enterprise_id", params.enterprise_id);
  if (params.sort_by) query.set("sort_by", params.sort_by);
  if (params.sort_order) query.set("sort_order", params.sort_order);
  const url = `${ALLIANCE_BASE}/experts${query.toString() ? `?${query.toString()}` : ""}`;
  const r = await fetch(url, {
    method: "GET",
    headers: authHeaders({ "Accept": "application/json" })
  });
  if (!r.ok) throw new Error(`experts/list HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) {
    if (!data.success) throw new Error(data.error || data.message || "\u83B7\u53D6\u4E13\u5BB6\u5217\u8868\u5931\u8D25");
    return data.data;
  }
  return data;
}
async function allianceConsultExpert(expertId, payload) {
  const r = await fetch(`${ALLIANCE_BASE}/experts/${encodeURIComponent(expertId)}/consult`, {
    method: "POST",
    headers: authHeaders({
      "Content-Type": "application/json",
      "Accept": "application/json"
    }),
    body: JSON.stringify({ ...payload, expert_id: expertId })
  });
  if (!r.ok) throw new Error(`experts/consult HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) {
    if (!data.success) throw new Error(data.error || data.message || "\u54A8\u8BE2\u5931\u8D25");
    return data.data;
  }
  return data;
}
async function allianceMultiExpertConsult(payload) {
  const r = await fetch(`${ALLIANCE_BASE}/experts/multi-consult`, {
    method: "POST",
    headers: authHeaders({
      "Content-Type": "application/json",
      "Accept": "application/json"
    }),
    body: JSON.stringify(payload)
  });
  if (!r.ok) throw new Error(`experts/multi-consult HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) {
    if (!data.success) throw new Error(data.error || data.message || "\u591A\u4E13\u5BB6\u534F\u540C\u5931\u8D25");
    return data.data;
  }
  return data;
}
async function allianceExpertDebate(payload, onFrame) {
  if (!payload.stream || !onFrame) {
    const r = await fetch(`${ALLIANCE_BASE}/experts/debate`, {
      method: "POST",
      headers: authHeaders({
        "Content-Type": "application/json",
        "Accept": "application/json"
      }),
      body: JSON.stringify(payload)
    });
    if (!r.ok) throw new Error(`experts/debate HTTP ${r.status}`);
    const data = await r.json();
    if (data && typeof data === "object" && "success" in data) {
      if (!data.success) throw new Error(data.error || data.message || "\u8FA9\u8BBA\u5931\u8D25");
      return data.data;
    }
    return data;
  }
  const resp = await fetch(`${ALLIANCE_BASE}/experts/debate`, {
    method: "POST",
    headers: authHeaders({
      "Content-Type": "application/json",
      "Accept": "text/event-stream"
    }),
    body: JSON.stringify(payload)
  });
  if (!resp.ok) throw new Error(`experts/debate HTTP ${resp.status}`);
  const reader = resp.body?.getReader();
  if (!reader) throw new Error("No readable stream");
  const decoder = new TextDecoder("utf-8");
  let buffer = "";
  let finalResult = null;
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    let idx;
    while ((idx = buffer.indexOf("\n\n")) >= 0) {
      const rawEvent = buffer.slice(0, idx);
      buffer = buffer.slice(idx + 2);
      let data = "";
      for (const line of rawEvent.split("\n")) {
        if (line.startsWith("data:")) data += line.slice(5).trimStart();
      }
      if (!data) continue;
      if (data === "[DONE]") {
        reader.releaseLock();
        if (!finalResult) throw new Error("\u8FA9\u8BBA\u672A\u8FD4\u56DE\u6700\u7EC8\u7ED3\u679C");
        return finalResult;
      }
      try {
        const parsed = JSON.parse(data);
        if (parsed.type === "result" || parsed.conclusion) {
          finalResult = parsed;
        } else if (onFrame) {
          const frame = parsed;
          const cont = onFrame(frame);
          if (cont === false) {
            reader.releaseLock();
            return finalResult || {};
          }
        }
      } catch (e) {
        console.warn("[alliance.debate] frame parse failed:", data, e);
      }
    }
  }
  reader.releaseLock();
  if (!finalResult) throw new Error("\u8FA9\u8BBA\u672A\u8FD4\u56DE\u6700\u7EC8\u7ED3\u679C");
  return finalResult;
}
async function allianceRouteExperts(payload) {
  const r = await fetch(`${ALLIANCE_BASE}/experts/route`, {
    method: "POST",
    headers: authHeaders({
      "Content-Type": "application/json",
      "Accept": "application/json"
    }),
    body: JSON.stringify(payload)
  });
  if (!r.ok) throw new Error(`experts/route HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) {
    if (!data.success) throw new Error(data.error || data.message || "\u667A\u80FD\u8DEF\u7531\u5931\u8D25");
    return data.data;
  }
  return data;
}
async function allianceIntelligentConsult(payload) {
  const r = await fetch(`${ALLIANCE_BASE}/experts/intelligent-consult`, {
    method: "POST",
    headers: authHeaders({
      "Content-Type": "application/json",
      "Accept": "application/json"
    }),
    body: JSON.stringify(payload)
  });
  if (!r.ok) throw new Error(`experts/intelligent-consult HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) {
    if (!data.success) throw new Error(data.error || data.message || "\u667A\u80FD\u54A8\u8BE2\u5931\u8D25");
    return data.data;
  }
  return data;
}
async function allianceAlgorithmAnalysis(payload) {
  const r = await fetch(`${ALLIANCE_BASE}/experts/algorithm-analysis`, {
    method: "POST",
    headers: authHeaders({
      "Content-Type": "application/json",
      "Accept": "application/json"
    }),
    body: JSON.stringify(payload)
  });
  if (!r.ok) throw new Error(`experts/algorithm-analysis HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) {
    if (!data.success) throw new Error(data.error || data.message || "\u7B97\u6CD5\u5206\u6790\u5931\u8D25");
    return data.data;
  }
  return data;
}
async function allianceGetExpertOverview() {
  const r = await fetch(`${ALLIANCE_BASE}/experts/overview`, {
    method: "GET",
    headers: authHeaders({ "Accept": "application/json" })
  });
  if (!r.ok) throw new Error(`experts/overview HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) {
    if (!data.success) throw new Error(data.error || data.message || "\u83B7\u53D6\u6982\u89C8\u5931\u8D25");
    return data.data;
  }
  return data;
}
async function allianceGetExpertMetrics() {
  const r = await fetch(`${ALLIANCE_BASE}/experts/metrics`, {
    method: "GET",
    headers: authHeaders({ "Accept": "application/json" })
  });
  if (!r.ok) throw new Error(`experts/metrics HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) {
    if (!data.success) throw new Error(data.error || data.message || "\u83B7\u53D6\u6307\u6807\u5931\u8D25");
    return data.data;
  }
  return data;
}
async function allianceGetSingleExpertMetrics(expertId) {
  const r = await fetch(`${ALLIANCE_BASE}/experts/${encodeURIComponent(expertId)}/metrics`, {
    method: "GET",
    headers: authHeaders({ "Accept": "application/json" })
  });
  if (!r.ok) throw new Error(`experts/metrics/${expertId} HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) {
    if (!data.success) throw new Error(data.error || data.message || "\u83B7\u53D6\u4E13\u5BB6\u6307\u6807\u5931\u8D25");
    return data.data;
  }
  return data;
}
async function createAllianceTask(payload) {
  const r = await fetch(`${ALLIANCE_BASE}/alliance/tasks`, {
    method: "POST",
    headers: authHeaders({
      "Content-Type": "application/json",
      "Accept": "application/json"
    }),
    body: JSON.stringify(payload)
  });
  if (!r.ok) throw new Error(`alliance/tasks HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) {
    if (!data.success) throw new Error(data.error || data.message || "\u521B\u5EFA\u4EFB\u52A1\u5931\u8D25");
    return data.data;
  }
  return data;
}
async function getAllianceTasks(params = {}) {
  const query = new URLSearchParams();
  if (params.page != null) query.set("page", String(params.page));
  if (params.page_size != null) query.set("page_size", String(params.page_size));
  if (params.status) query.set("status", params.status);
  if (params.keyword) query.set("keyword", params.keyword);
  if (params.strategy) query.set("strategy", params.strategy);
  if (params.sort_by) query.set("sort_by", params.sort_by);
  if (params.sort_order) query.set("sort_order", params.sort_order);
  const url = `${ALLIANCE_BASE}/alliance/tasks${query.toString() ? `?${query.toString()}` : ""}`;
  const r = await fetch(url, {
    method: "GET",
    headers: authHeaders({ "Accept": "application/json" })
  });
  if (!r.ok) throw new Error(`alliance/tasks/list HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) {
    if (!data.success) throw new Error(data.error || data.message || "\u83B7\u53D6\u4EFB\u52A1\u5217\u8868\u5931\u8D25");
    return data.data;
  }
  return data;
}
async function getAllianceTask(taskId) {
  const r = await fetch(`${ALLIANCE_BASE}/alliance/tasks/${encodeURIComponent(taskId)}`, {
    method: "GET",
    headers: authHeaders({ "Accept": "application/json" })
  });
  if (!r.ok) throw new Error(`alliance/task HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) {
    if (!data.success) throw new Error(data.error || data.message || "\u83B7\u53D6\u4EFB\u52A1\u8BE6\u60C5\u5931\u8D25");
    return data.data;
  }
  return data;
}
async function getCollaborationPlan(taskId) {
  const r = await fetch(`${ALLIANCE_BASE}/alliance/tasks/${encodeURIComponent(taskId)}/plan`, {
    method: "GET",
    headers: authHeaders({ "Accept": "application/json" })
  });
  if (!r.ok) throw new Error(`alliance/task/plan HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) {
    if (!data.success) throw new Error(data.error || data.message || "\u83B7\u53D6\u534F\u4F5C\u8BA1\u5212\u5931\u8D25");
    return data.data;
  }
  return data;
}
async function getExecutionLogsSSE(taskId, onLog) {
  const resp = await fetch(`${ALLIANCE_BASE}/alliance/tasks/${encodeURIComponent(taskId)}/logs/stream`, {
    method: "GET",
    headers: authHeaders({
      "Accept": "text/event-stream"
    })
  });
  if (!resp.ok) throw new Error(`alliance/task/logs HTTP ${resp.status}`);
  const reader = resp.body?.getReader();
  if (!reader) throw new Error("No readable stream");
  const decoder = new TextDecoder("utf-8");
  let buffer = "";
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    let idx;
    while ((idx = buffer.indexOf("\n\n")) >= 0) {
      const rawEvent = buffer.slice(0, idx);
      buffer = buffer.slice(idx + 2);
      let data = "";
      for (const line of rawEvent.split("\n")) {
        if (line.startsWith("data:")) data += line.slice(5).trimStart();
      }
      if (!data) continue;
      if (data === "[DONE]") {
        reader.releaseLock();
        return;
      }
      try {
        const entry = JSON.parse(data);
        const cont = onLog(entry);
        if (cont === false) {
          reader.releaseLock();
          return;
        }
      } catch (e) {
        console.warn("[alliance.logs] frame parse failed:", data, e);
      }
    }
  }
  reader.releaseLock();
}
async function getFusionResults(taskId) {
  const r = await fetch(`${ALLIANCE_BASE}/alliance/tasks/${encodeURIComponent(taskId)}/fusion`, {
    method: "GET",
    headers: authHeaders({ "Accept": "application/json" })
  });
  if (!r.ok) throw new Error(`alliance/task/fusion HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) {
    if (!data.success) throw new Error(data.error || data.message || "\u83B7\u53D6\u878D\u5408\u7ED3\u679C\u5931\u8D25");
    return data.data;
  }
  return data;
}
async function pauseAllianceTask(taskId) {
  const r = await fetch(`${ALLIANCE_BASE}/alliance/tasks/${encodeURIComponent(taskId)}/pause`, {
    method: "POST",
    headers: authHeaders({ "Accept": "application/json" })
  });
  if (!r.ok) throw new Error(`alliance/task/pause HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) return data;
  return { success: true };
}
async function resumeAllianceTask(taskId) {
  const r = await fetch(`${ALLIANCE_BASE}/alliance/tasks/${encodeURIComponent(taskId)}/resume`, {
    method: "POST",
    headers: authHeaders({ "Accept": "application/json" })
  });
  if (!r.ok) throw new Error(`alliance/task/resume HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) return data;
  return { success: true };
}
async function cancelAllianceTask(taskId) {
  const r = await fetch(`${ALLIANCE_BASE}/alliance/tasks/${encodeURIComponent(taskId)}/cancel`, {
    method: "POST",
    headers: authHeaders({ "Accept": "application/json" })
  });
  if (!r.ok) throw new Error(`alliance/task/cancel HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) return data;
  return { success: true };
}
async function retryAllianceTask(taskId) {
  const r = await fetch(`${ALLIANCE_BASE}/alliance/tasks/${encodeURIComponent(taskId)}/retry`, {
    method: "POST",
    headers: authHeaders({ "Accept": "application/json" })
  });
  if (!r.ok) throw new Error(`alliance/task/retry HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) return data.data || data;
  return { success: true, task_id: taskId };
}
async function getAllianceStats() {
  const r = await fetch(`${ALLIANCE_BASE}/alliance/stats`, {
    method: "GET",
    headers: authHeaders({ "Accept": "application/json" })
  });
  if (!r.ok) throw new Error(`alliance/stats HTTP ${r.status}`);
  const data = await r.json();
  if (data && typeof data === "object" && "success" in data) {
    if (!data.success) throw new Error(data.error || data.message || "\u83B7\u53D6\u8054\u76DF\u7EDF\u8BA1\u5931\u8D25");
    return data.data;
  }
  return data;
}
export {
  ALLIANCE_BASE,
  allianceAlgorithmAnalysis,
  allianceConsultExpert,
  allianceExpertDebate,
  allianceGetExpertMetrics,
  allianceGetExpertOverview,
  allianceGetExperts,
  allianceGetSingleExpertMetrics,
  allianceIntelligentConsult,
  allianceMultiExpertConsult,
  allianceRegisterExpert,
  allianceRouteExperts,
  authHeaders,
  cancelAllianceTask,
  createAllianceTask,
  getAllianceCapabilities,
  getAllianceStats,
  getAllianceTask,
  getAllianceTasks,
  getCollaborationPlan,
  getExecutionLogsSSE,
  getFusionResults,
  getVoiceHealth,
  pauseAllianceTask,
  resumeAllianceTask,
  retryAllianceTask,
  runAllianceFullSSE
};
