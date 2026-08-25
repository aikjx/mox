#!/usr/bin/env node
// T4: Rust Workspace Dependency Validator
// Validates 100% workspace inheritance for dependencies (no direct version strings).
// RED -> GREEN TDD harness.

const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const ROOT_CARGO = path.join(ROOT, 'Cargo.toml');

// 16 workspace members (from root Cargo.toml)
const CRATE_PATHS = [
    'platform/services/operator-core/Cargo.toml',
    'platform/services/operator-wasm/Cargo.toml',
    'platform/services/graph-algorithms/Cargo.toml',
    'platform/services/optimizer/Cargo.toml',
    'platform/services/flow-ai/Cargo.toml',
    'platform/services/mox-expert/Cargo.toml',
    'platform/services/hermes-flow-bridge/Cargo.toml',
    'platform/services/business-catalog/Cargo.toml',
    'platform/services/ai-agent/Cargo.toml',
    'platform/services/template-market/Cargo.toml',
    'platform/gateway/runtime/Cargo.toml',
    'platform/services/mox-system/Cargo.toml',
    'platform/services/primiflow-core/Cargo.toml',
    'platform/services/primiflow-fusion/Cargo.toml',
    'platform/services/kg-hub/Cargo.toml',
];

/**
 * Parse a simple TOML file into:
 *  { sections: { name: [lines...] }, rawLines: [...] }
 * Sections are [name], [dependencies], [dev-dependencies], [workspace.dependencies], etc.
 */
function parseSimpleToml(filePath) {
    const content = fs.readFileSync(filePath, 'utf8');
    const rawLines = content.split(/\r?\n/);
    const sections = {};
    let current = '__root__';
    sections[current] = [];

    for (let i = 0; i < rawLines.length; i++) {
        let line = rawLines[i];
        let stripped = line;
        // Check ARRAY of tables header [[section]] first (higher precedence)
        const arrHeader = stripped.match(/^\s*\[\[([^\]]+)\]\]\s*$/);
        if (arrHeader) {
            // Use '__array__:name' prefix so it doesn't collide with [section]
            current = '__array__:' + arrHeader[1].trim();
            sections[current] = sections[current] || [];
            continue;
        }
        // check regular section header [section]
        const header = stripped.match(/^\s*\[([^\]]+)\]\s*$/);
        if (header) {
            current = header[1].trim();
            sections[current] = sections[current] || [];
            continue;
        }
        sections[current].push({ line: stripped, lineno: i + 1 });
    }
    return { sections, rawLines, content };
}

/**
 * Extract dependency names from [workspace.dependencies] section lines.
 * Formats accepted:
 *   name = "version"
 *   name = { version = "x", ... }
 */
function extractWorkspaceDepNames(wsDepsSectionLines) {
    const names = new Set();
    for (const entry of wsDepsSectionLines) {
        const line = entry.line;
        const m = line.match(/^\s*([A-Za-z0-9_-]+)\s*=/);
        if (m) names.add(m[1]);
    }
    return names;
}

/**
 * Extract the reqwest version from workspace.dependencies (if present).
 */
function extractWorkspaceReqwestVersion(wsDepsSectionLines) {
    for (const entry of wsDepsSectionLines) {
        const line = entry.line;
        const m = line.match(/^\s*reqwest\s*=\s*(.*)$/);
        if (m) {
            const rest = m[1];
            // version = "X"
            const vm = rest.match(/version\s*=\s*"([^"]+)"/);
            if (vm) return vm[1];
            const qvm = rest.match(/^"([^"]+)"/);
            if (qvm) return qvm[1];
        }
    }
    return null;
}

/**
 * Classify a dependency entry line.
 * Returns { status, name, detail, isTableForm }
 *   status: 'pass_workspace' | 'pass_path' | 'fail_version_string' | 'fail_version_inline' | 'skip'
 */
function classifyDepLine(rawLine, entryNo, depSectionName, cratePath, workspaceDepNames) {
    // Join lines if they are multi-line {...}  - simple brace-count continuation
    let line = rawLine;
    // strip inline comment outside of strings
    const noComment = stripComment(line);

    // handle simple name = "ver" (quoted string)
    const simpleVersion = noComment.match(/^\s*([A-Za-z0-9_-]+)\s*=\s*"([^"]+)"\s*$/);
    if (simpleVersion) {
        const name = simpleVersion[1];
        const version = simpleVersion[2];
        return {
            status: 'fail_version_string',
            name,
            detail: `${depSectionName}: "${name}" = "${version}" (direct quoted version, not workspace=true)`,
            version,
            line: rawLine,
        };
    }

    // handle inline table form: name = { ... }
    const tableMatch = noComment.match(/^\s*([A-Za-z0-9_-]+)\s*=\s*\{(.*)\}\s*$/);
    if (tableMatch) {
        const name = tableMatch[1];
        const inside = tableMatch[2];

        // path deps are always OK
        const hasPath = /\bpath\s*=\s*"/.test(inside);
        if (hasPath) {
            return { status: 'pass_path', name, detail: `${depSectionName}: "${name}" uses path dep`, line: rawLine };
        }

        const hasWorkspaceTrue = /\bworkspace\s*=\s*true\b/.test(inside);
        if (hasWorkspaceTrue) {
            return { status: 'pass_workspace', name, detail: `${depSectionName}: "${name}" uses workspace=true`, line: rawLine };
        }

        const versionMatch = inside.match(/\bversion\s*=\s*"([^"]+)"/);
        if (versionMatch) {
            return {
                status: 'fail_version_inline',
                name,
                version: versionMatch[1],
                detail: `${depSectionName}: "${name}" = { version = "${versionMatch[1]}", ... } (inline version, not workspace=true)`,
                line: rawLine,
            };
        }

        // No version and no workspace=true... may be git dep? Skip.
        return { status: 'skip', name, detail: `${depSectionName}: "${name}" unclassified (likely git/dep alias)`, line: rawLine };
    }

    // skip empty / comment-only lines
    if (!noComment.trim() || noComment.trim().startsWith('#')) {
        return { status: 'skip', name: null, detail: '', line: rawLine };
    }

    // Could be part of multi-line. skip for now
    return { status: 'skip', name: '?', detail: `${depSectionName}: unmatched line: ${rawLine.substring(0, 80)}`, line: rawLine };
}

function stripComment(line) {
    // naive: remove first '#' that's not inside quotes
    let inSingle = false, inDouble = false;
    for (let i = 0; i < line.length; i++) {
        const c = line[i];
        if (c === '"' && !inSingle) inDouble = !inDouble;
        else if (c === "'" && !inDouble) inSingle = !inSingle;
        else if (c === '#' && !inDouble && !inSingle) {
            return line.substring(0, i);
        }
    }
    return line;
}

// ======= Main =======
function main() {
    const root = parseSimpleToml(ROOT_CARGO);
    const wsDepsLines = root.sections['workspace.dependencies'] || [];
    const workspaceDepNames = extractWorkspaceDepNames(wsDepsLines);
    const workspaceReqwestVer = extractWorkspaceReqwestVersion(wsDepsLines);

    console.log('=== Rust Workspace Dependency Governance Validator ===');
    console.log(`Workspace members scanned: ${CRATE_PATHS.length}`);
    console.log(`Workspace.deps count (root): ${workspaceDepNames.size}`);
    console.log(`Workspace reqwest version: ${workspaceReqwestVer || '(N/A)'}`);
    console.log('');

    const violations = [];       // all failures
    const reqwestDrift = [];     // crates with reqwest != workspace

    for (const relPath of CRATE_PATHS) {
        const absPath = path.join(ROOT, relPath);
        if (!fs.existsSync(absPath)) {
            console.warn(`[WARN] Missing Cargo.toml: ${relPath}`);
            continue;
        }
        const parsed = parseSimpleToml(absPath);
        const crateName = relPath.split('/').slice(-2, -1)[0] || path.basename(path.dirname(absPath));

        const depBlocks = [
            { name: '[dependencies]', sectionKey: 'dependencies', acTag: 'AC-09' },
            { name: '[dev-dependencies]', sectionKey: 'dev-dependencies', acTag: 'AC-10' },
        ];

        for (const block of depBlocks) {
            const lines = parsed.sections[block.sectionKey] || [];
            for (const entry of lines) {
                const classified = classifyDepLine(entry.line, entry.lineno, block.name, relPath, workspaceDepNames);
                if (classified.status === 'fail_version_string' || classified.status === 'fail_version_inline') {
                    violations.push({
                        crate: crateName,
                        cratePath: relPath,
                        lineno: entry.lineno,
                        section: block.sectionKey,
                        acTag: block.acTag,
                        ...classified,
                    });

                    // reqwest drift check
                    if (classified.name === 'reqwest' && workspaceReqwestVer) {
                        const v = classified.version || '';
                        // normalize major.minor
                        const crateMajorMinor = v.split('.').slice(0, 2).join('.');
                        const wsMajorMinor = workspaceReqwestVer.split('.').slice(0, 2).join('.');
                        if (crateMajorMinor !== wsMajorMinor) {
                            reqwestDrift.push({
                                crate: crateName,
                                cratePath: relPath,
                                version: v,
                                workspaceVersion: workspaceReqwestVer,
                                lineno: entry.lineno,
                                section: block.sectionKey,
                            });
                        }
                    }
                }
            }
        }
    }

    // Summarize by rule
    const ac09 = violations.filter(v => v.section === 'dependencies');
    const ac10 = violations.filter(v => v.section === 'dev-dependencies');

    console.log('--- VIOLATIONS ---');
    if (violations.length === 0) {
        console.log('  (none)');
    } else {
        violations.forEach((v, idx) => {
            console.log(`  [${idx + 1}] ${v.cratePath}:L${v.lineno} | ${v.section} | ${v.detail}`);
        });
    }
    console.log('');

    console.log('--- REQWEST VERSION DRIFT ---');
    if (reqwestDrift.length === 0) {
        console.log('  (none - all reqwest versions match workspace)');
    } else {
        reqwestDrift.forEach(d => {
            console.log(`  * ${d.cratePath}:L${d.lineno} [${d.section}] reqwest=${d.version} vs workspace=${d.workspaceVersion}  <-- DRIFT`);
        });
    }
    console.log('');

    // Rule assertions
    console.log('--- RULE ASSERTIONS ---');
    const pass_ac09 = ac09.length === 0;
    const pass_ac10 = ac10.length === 0;
    const pass_reqwest = reqwestDrift.length === 0;
    console.log(`  AC-09 (direct version= in [dependencies] count=0):        ${pass_ac09 ? 'PASS' : 'FAIL'}  (found ${ac09.length})`);
    console.log(`  AC-10 (direct version= in [dev-dependencies] count=0):    ${pass_ac10 ? 'PASS' : 'FAIL'}  (found ${ac10.length})`);
    console.log(`  REQWEST drift (all crates match workspace reqwest ver):   ${pass_reqwest ? 'PASS' : 'FAIL'}  (drifted=${reqwestDrift.length})`);

    const overall = pass_ac09 && pass_ac10 && pass_reqwest;
    console.log('');
    console.log(`OVERALL: ${overall ? 'ALL PASS (GREEN)' : 'FAILURES DETECTED (RED)'}`);

    // Print first 5 examples (for RED evidence)
    console.log('');
    console.log(`Summary: violations=${violations.length}, reqwest_drift=${reqwestDrift.length}, ac09=${ac09.length}, ac10=${ac10.length}`);
    console.log('Examples (first 5):');
    violations.slice(0, 5).forEach((v, i) => {
        console.log(`  ${i + 1}. ${v.cratePath}:L${v.lineno} [${v.section}] ${v.name}: ${v.status === 'fail_version_string' ? 'direct "' + v.version + '"' : '{version="' + v.version + '"}'}`);
    });

    process.exit(overall ? 0 : 1);
}

main();
