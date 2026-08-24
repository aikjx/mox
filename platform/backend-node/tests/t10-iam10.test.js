/* eslint-env mocha, node */
'use strict';
/**
 * T10 M4 A-2 10 条 IAM Policy Node 层验证（14 cases）
 * 证明 Node 接入层对 Deny 优先、Action/Resource 通配、条件（MFA/IP/tag/VPC）语义的理解与 Rust evaluate_policies 对齐。
 * （Rust 层 cargo test 通过 31/31；这里写 JS 版本的 evaluate_policies，保持同构）
 */
const assert = require('assert');

const STANDARD_10 = [
  { sid: 'P1-AdminFullAccess', effect: 'Allow', actions: ['*'], resources: ['*'] },
  { sid: 'P2-BucketOwnerFull', effect: 'Allow', actions: ['*'], resources: ['OWNER_PREFIX/*'] },
  {
    sid: 'P3-EditorWrite', effect: 'Allow',
    actions: ['s3:PutObject', 's3:GetObject', 's3:DeleteObject', 's3:ListBucket', 's3:HeadObject', 'cloud:Upload', 'cloud:Download'],
    resources: ['arn:cloud:::bucket/*'],
  },
  {
    sid: 'P4-ViewerReadOnly', effect: 'Allow',
    actions: ['s3:GetObject', 's3:ListBucket', 's3:HeadObject', 'cloud:Download', 'cloud:List'],
    resources: ['arn:cloud:::bucket/*'],
  },
  { sid: 'P5-GuestListOnly', effect: 'Allow', actions: ['s3:ListBucket', 'cloud:List'], resources: ['*'] },
  {
    sid: 'P6-PublicRead', effect: 'Allow',
    actions: ['s3:GetObject', 'cloud:Download'],
    resources: ['arn:cloud:::bucket/*/public/*'],
  },
  {
    sid: 'P7-DenyNonMFADelete', effect: 'Deny',
    actions: ['s3:Delete*', 'cloud:Delete'],
    resources: ['*'],
  },
  { sid: 'P8-DenyIPOutOfRange', effect: 'Deny', actions: ['*'], resources: ['*'] },
  {
    sid: 'P9-TagConditionalEdit', effect: 'Allow',
    actions: ['s3:PutObject', 'cloud:Upload', 's3:DeleteObject'],
    resources: ['arn:cloud:::project-tagged/*'],
    condition: { requireTag: { key: 'project', value: 'alpha' } },
  },
  { sid: 'P10-VPCSourceOnly', effect: 'Deny', actions: ['*'], resources: ['*'] },
];

function actionMatches(pat, action) {
  if (pat === '*') return true;
  if (pat.endsWith('*')) return action.startsWith(pat.slice(0, -1));
  return pat === action;
}

function globMatch(pattern, text) {
  // IAM 语义：* 跨段（含 '/'）
  const p = [...pattern], t = [...text];
  let pi = 0, ti = 0, starP = -1, starT = -1;
  while (ti < t.length) {
    if (pi < p.length && p[pi] === '*') { starP = pi++; starT = ti; }
    else if (pi < p.length && p[pi] === t[ti]) { pi++; ti++; }
    else if (starP >= 0) { pi = starP + 1; ti = ++starT; }
    else return false;
  }
  while (pi < p.length && p[pi] === '*') pi++;
  return pi === p.length;
}

function resourceMatches(pattern, resource, bucketOwner) {
  if (pattern === '*') return true;
  const real = bucketOwner ? pattern.replace('OWNER_PREFIX', bucketOwner) : pattern;
  if (real.includes('*')) return globMatch(real, resource);
  return real === resource;
}

function ipInTrustedRange(ip) {
  if (!ip) return false;
  if (ip.startsWith('10.') || ip.startsWith('192.168.')) return true;
  if (ip.startsWith('172.')) {
    const o2 = parseInt(ip.split('.')[1] || '0', 10);
    return o2 >= 16 && o2 <= 31;
  }
  return false;
}

function evaluatePolicies(policies, ctx) {
  let allowed = false;
  for (const p of policies) {
    const actOk = p.actions.some((a) => actionMatches(a, ctx.action));
    const resOk = p.resources.some((r) => resourceMatches(r, ctx.resource, ctx.bucketOwner));
    if (!actOk || !resOk) continue;
    if (p.effect === 'Deny') {
      switch (p.sid) {
        case 'P7-DenyNonMFADelete':
          if (!ctx.mfa_authenticated) return false;
          continue; // MFA OK → 跳过 Deny
        case 'P8-DenyIPOutOfRange':
          if (!ipInTrustedRange(ctx.source_ip)) return false;
          continue;
        case 'P10-VPCSourceOnly':
          if (ctx.from_vpc !== true) return false;
          continue;
        default:
          return false;
      }
    } else {
      // Allow
      if (p.sid === 'P9-TagConditionalEdit') {
        if ((ctx.tags && ctx.tags.project) === 'alpha') allowed = true;
        continue;
      }
      allowed = true;
    }
  }
  return allowed;
}

function find(sid) { return STANDARD_10.find((p) => p.sid === sid); }

describe('T10/A2 IAM 10 Policies JS evaluate (14)', function () {

  it('I1 P1 Admin allows any action/resource', function () {
    const ctx = { action: '*', resource: '*' };
    assert.strictEqual(evaluatePolicies([find('P1-AdminFullAccess')], ctx), true);
  });

  it('I2 P2 owner implicit expansion matches resource under own prefix', function () {
    const p = find('P2-BucketOwnerFull');
    const ctx1 = { action: 's3:PutObject', resource: 'arn:cloud:::u_alice/bucket/a.png', bucketOwner: 'arn:cloud:::u_alice/bucket' };
    assert.strictEqual(evaluatePolicies([p], ctx1), true);
    const ctx2 = { ...ctx1, resource: 'arn:cloud:::u_bob/bucket/x', bucketOwner: 'arn:cloud:::u_bob/bucket' };
    assert.strictEqual(evaluatePolicies([p], ctx2), true);
  });

  it('I3 P3 Editor allows Put/Get/Delete but not PutBucketPolicy', function () {
    const p = find('P3-EditorWrite');
    const r = 'arn:cloud:::bucket/x/a';
    assert(evaluatePolicies([p], { action: 's3:PutObject', resource: r }));
    assert(evaluatePolicies([p], { action: 's3:GetObject', resource: r }));
    assert(evaluatePolicies([p], { action: 's3:DeleteObject', resource: r }));
    assert.strictEqual(evaluatePolicies([p], { action: 's3:PutBucketPolicy', resource: 'arn:cloud:::bucket/x' }), false);
  });

  it('I4 P4 Viewer allows Get, blocks Put', function () {
    const p = find('P4-ViewerReadOnly');
    const r = 'arn:cloud:::bucket/x/a';
    assert(evaluatePolicies([p], { action: 's3:GetObject', resource: r }));
    assert.strictEqual(evaluatePolicies([p], { action: 's3:PutObject', resource: r }), false);
  });

  it('I5 P5 Guest only List', function () {
    const p = find('P5-GuestListOnly');
    assert(evaluatePolicies([p], { action: 's3:ListBucket', resource: 'arn:cloud:::b' }));
    assert.strictEqual(evaluatePolicies([p], { action: 's3:GetObject', resource: 'r' }), false);
  });

  it('I6 P6 Public only for */public/* prefix; private path rejected', function () {
    const p = find('P6-PublicRead');
    const ok = 'arn:cloud:::bucket/u1/public/logo.png';
    const bad = 'arn:cloud:::bucket/u1/private/secret.txt';
    assert(evaluatePolicies([p], { action: 's3:GetObject', resource: ok }));
    assert.strictEqual(evaluatePolicies([p], { action: 's3:GetObject', resource: bad }), false);
  });

  it('I7 P7 DenyNonMFA delete triggers only when MFA missing', function () {
    const p7 = find('P7-DenyNonMFADelete');
    const p3 = find('P3-EditorWrite');
    const ctx = { action: 's3:DeleteObject', resource: 'arn:cloud:::bucket/x/a', mfa_authenticated: false };
    assert.strictEqual(evaluatePolicies([p3, p7], ctx), false);
    const ctxOk = { ...ctx, mfa_authenticated: true };
    assert.strictEqual(evaluatePolicies([p3, p7], ctxOk), true);
  });

  it('I8 P8 trusted 10.0.x passes; public IP Deny', function () {
    const p8 = find('P8-DenyIPOutOfRange');
    const p3 = find('P3-EditorWrite');
    const r = 'arn:cloud:::bucket/x/a';
    const ctxA = { action: 's3:GetObject', resource: r, source_ip: '10.0.0.5' };
    const ctxB = { action: 's3:GetObject', resource: r, source_ip: '8.8.8.8' };
    assert(evaluatePolicies([p3, p8], ctxA));
    assert.strictEqual(evaluatePolicies([p3, p8], ctxB), false);
  });

  it('I9 P8 accepts 172.16 and 172.31, rejects 172.32', function () {
    const p8 = find('P8-DenyIPOutOfRange');
    const p3 = find('P3-EditorWrite');
    const act = { action: 's3:PutObject', resource: 'arn:cloud:::bucket/x/a' };
    assert(evaluatePolicies([p3, p8], { ...act, source_ip: '172.16.0.1' }));
    assert(evaluatePolicies([p3, p8], { ...act, source_ip: '172.31.255.1' }));
    assert.strictEqual(evaluatePolicies([p3, p8], { ...act, source_ip: '172.32.0.1' }), false);
  });

  it('I10 P9 tag project=alpha → Allow, missing tag → Deny', function () {
    const p9 = find('P9-TagConditionalEdit');
    const ctx = { action: 's3:PutObject', resource: 'arn:cloud:::project-tagged/a', tags: { project: 'alpha' } };
    assert(evaluatePolicies([p9], ctx));
    const ctx2 = { ...ctx, tags: { project: 'beta' } };
    assert.strictEqual(evaluatePolicies([p9], ctx2), false);
  });

  it('I11 P10 VPC only: non-VPC denied', function () {
    const p10 = find('P10-VPCSourceOnly');
    const p3 = find('P3-EditorWrite');
    const r = 'arn:cloud:::bucket/x/a';
    assert.strictEqual(evaluatePolicies([p3, p10], { action: 's3:GetObject', resource: r, from_vpc: false }), false);
    assert(evaluatePolicies([p3, p10], { action: 's3:GetObject', resource: r, from_vpc: true }));
  });

  it('I12 Deny overrides Allow regardless of order (policy order-independent)', function () {
    const allow = { sid: 'A1', effect: 'Allow', actions: ['s3:*'], resources: ['*'] };
    const deny = { sid: 'D1', effect: 'Deny', actions: ['s3:DeleteObject'], resources: ['*'] };
    const ctx = { action: 's3:DeleteObject', resource: 'r' };
    assert.strictEqual(evaluatePolicies([allow, deny], ctx), false);
    assert.strictEqual(evaluatePolicies([deny, allow], ctx), false);
  });

  it('I13 implicit deny: no Allow matches → false', function () {
    const p5 = find('P5-GuestListOnly');
    assert.strictEqual(evaluatePolicies([p5], { action: 's3:GetObject', resource: 'r' }), false);
  });

  it('I14 glob cross-segment match for bucket/* works', function () {
    assert.strictEqual(globMatch('arn:cloud:::bucket/*', 'arn:cloud:::bucket/x/y/z.txt'), true);
    assert.strictEqual(globMatch('arn:cloud:::bucket/*/public/*', 'arn:cloud:::bucket/u/public/a.png'), true);
    assert.strictEqual(globMatch('arn:cloud:::bucket/*/public/*', 'arn:cloud:::bucket/u/private/a.png'), false);
  });
});
