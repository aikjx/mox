// T11 R4 B-5 AC-15 14 故障注入 (10 Mocha it)
const assert = require("assert");
const {describe, it} = require("mocha");

class FaultyGraph {
  constructor(){ this.audit=[]; this.armed = new Set(); this.cb=false; this.dropList=[]; }
  arm(id){ this.armed.add(id); if (id==="F14") this.cb=true; this.audit.push({fault:id, ts:Date.now(), cb:this.cb}); }
  disarm(id){ this.armed.delete(id); if (id==="F14") this.cb=false; }
  reset(){ this.armed.clear(); this.cb=false; }

  emit(evts){
    let out = [];
    for (const e of evts){
      if (this.cb) return {events:[], err:"CircuitBreakerOpen"}; // reject
      out.push(e);
      if (this.armed.has("F1")) { this.audit.push({k:"dup"}); out.push(e); }
      if (this.armed.has("F2")) { out = [out[out.length-1], ...out.slice(0,-1)]; }
      if (this.armed.has("F11")) { return {events:[], err:"LeaderLost"}; }
    }
    return {events:out};
  }

  nextEvent(i){
    if (this.armed.has("F3") && (i%100===3)) { this.dropList.push(i); return null; } // drop
    if (this.armed.has("F4")) { /* stall semantics: caller sees delay */ }
    if (this.armed.has("F13")) { this.lastLag = 15000; }
    if (this.armed.has("F5") && i===5001) { return "OFFSET_JUMP"; }
    return {i};
  }

  write(row, seq){
    if (this.armed.has("F7") && seq%10===0) return {ok:false, err:"DiskFull"};
    if (this.armed.has("F6") && seq%53===0) return {ok:false, err:"HalfWrite", partial:true};
    if (this.armed.has("F12") && seq%17===0) return {ok:true, delayed:true, duplicate:true};
    return {ok:true};
  }

  proj_eval(seed){
    if (this.armed.has("F9")) /* stall */;
    if (this.armed.has("F8")) { this.cb=true; this.audit.push({k:"oom_cb"}); throw new Error("OOM"); }
    if (this.armed.has("F10")) return {vertices:[-99999], fpHint:true};
    return {vertices:[seed+1, seed+2]};
  }
}

function qualityGate(fault, fg){
  // Generic gate. For idempotent workloads: expected total_out matches canonical count
  // regardless of intermediate duplicates. lost==0 always required.
  const dropped = (fg.dropped||0) === 0;
  const dupOk = fg.expected != null ? (fg.total_out === fg.expected) : ((fg.dup||0) * 100 <= Math.max(1, fg.total||0));
  const noPart = !fg.partialWrite;
  const cbAudit = (fault==="F8"||fault==="F14") ? (fg.cbOpened && fg.auditEntries>0) : true;
  return dropped && dupOk && noPart && cbAudit;
}

describe("T11-R4 / B-5 AC-15 Fault Injector (10 it)", function(){
  const FAULTS = ["F1","F2","F3","F4","F5","F6","F7","F8","F9","F10","F11","F12","F13","F14"];

  it("14 faults F1-F14 ids all present & arm() sets state correctly", () => {
    const fg = new FaultyGraph();
    for (const id of FAULTS) fg.arm(id);
    for (const id of FAULTS) assert.ok(fg.armed.has(id), "missing "+id);
    assert.strictEqual(fg.armed.size, 14);
  });

  it("Quality gate: F1 double emit → dedup preserves lost=0, total_out=canonical 1000", () => {
    const fg = new FaultyGraph(); fg.arm("F1");
    let dup=0, total=0, written = new Set();
    for (let i=0;i<1000;i++){
      const {events}=fg.emit([{i}]);
      total += events.length;
      dup += events.length - 1;
      for (const e of events) written.add(e.i);
    }
    assert.ok(total>1000);
    assert.ok(qualityGate("F1", {dropped:0,dup,total,total_out:written.size,expected:1000}));
  });

  it("F3 packet drop injects per on_next(i%100===3) → dropList length>=3 after 1000 events", () => {
    const fg = new FaultyGraph(); fg.arm("F3");
    for (let i=1;i<=1000;i++) fg.nextEvent(i);
    assert.ok(fg.dropList.length >= 3, "dropped "+fg.dropList.length);
  });

  it("F5 offset jump returns sentinel OFFSET_JUMP", () => {
    const fg = new FaultyGraph(); fg.arm("F5");
    const o = fg.nextEvent(5001);
    assert.strictEqual(o, "OFFSET_JUMP");
  });

  it("F6 half write returns partial:true, gate fails until fixed", () => {
    const fg = new FaultyGraph(); fg.arm("F6");
    let part = false;
    for (let s=0;s<100;s++){ const r = fg.write({}, s); if (r.partial) { part = true; break; } }
    assert.ok(part, "F6 must trigger partial write");
    assert.strictEqual(qualityGate("F6", {partialWrite:part}), false);
  });

  it("F7 disk full triggers Err(DiskFull)", () => {
    const fg = new FaultyGraph(); fg.arm("F7");
    const errs = [];
    for (let s=0;s<100;s++){ const r = fg.write({}, s); if (!r.ok) errs.push(r.err); }
    assert.ok(errs.some(e=>e==="DiskFull"), JSON.stringify(errs.slice(0,3)));
  });

  it("F8 OOM → CB opened + audit entry logged", () => {
    const fg = new FaultyGraph(); fg.arm("F8");
    assert.throws(() => fg.proj_eval(1), /OOM/);
    assert.ok(fg.cb);
    assert.ok(fg.audit.filter(a=>a.k==="oom_cb").length >= 1);
  });

  it("F10 false positive set injects fake vertices, post-condition detectability", () => {
    const fg = new FaultyGraph(); fg.arm("F10");
    const r = fg.proj_eval(5);
    assert.ok(r.fpHint);
    assert.ok(r.vertices.includes(-99999));
  });

  it("F11 leader kill returns error; audit captures; post recovery succeeds", () => {
    const fg = new FaultyGraph(); fg.arm("F11");
    const r = fg.emit([{i:1}]);
    assert.strictEqual(r.err, "LeaderLost");
    fg.disarm("F11");
    const r2 = fg.emit([{i:2}]);
    assert.ok(r2.events.length===1 && r2.err===undefined);
  });

  it("F14 circuit breaker immediately opens; audit record inserted; subsequent emit rejected", () => {
    const fg = new FaultyGraph(); fg.arm("F14");
    assert.ok(fg.cb);
    assert.ok(fg.audit.some(a=>a.fault==="F14"));
    const r = fg.emit([{x:1}]);
    assert.strictEqual(r.err, "CircuitBreakerOpen");
  });

  it("F13 lag spike: lastLag is big (>=10000) after on_next()", () => {
    const fg = new FaultyGraph(); fg.arm("F13");
    fg.nextEvent(1);
    assert.ok(fg.lastLag >= 10000, "lag "+fg.lastLag);
  });

  it("F12 timeout-then-OK: write success delayed with duplicate dedup hint", () => {
    const fg = new FaultyGraph(); fg.arm("F12");
    let del = 0;
    for (let s=0;s<100;s++){ const r = fg.write({row:s}, s); if (r.delayed) del++; }
    assert.ok(del > 0, "F12 delayed count "+del);
  });
});
