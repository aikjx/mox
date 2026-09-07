"""Batch fix remaining cross-domain violations:
- kb-svc: drop mox-ai-expert-svc, use existing mox-ai-expert-proto
- alliance-executor-svc: swap to mox-ai-expert-proto
- Replace svc paths with proto paths in all .rs files
"""
import os

def patch_crate(crate_dir, svc_name, proto_name, factory_subs=None):
    """Patch Cargo.toml + src files of a crate."""
    # Cargo.toml
    cargo = os.path.join(crate_dir, 'Cargo.toml')
    c = open(cargo, encoding='utf-8').read()
    if f'{svc_name} = ' in c:
        if proto_name and f'{proto_name} = ' not in c:
            c = c.replace(f'{svc_name} = {{ workspace = true }}\n', f'{proto_name} = {{ workspace = true }}\n')
            c = c.replace(f'{svc_name} = {{', f'{proto_name} = {{')
        else:
            c = c.replace(f'{svc_name} = {{ workspace = true }}\n', '')
    open(cargo, 'w', encoding='utf-8').write(c)
    print(f'  Cargo.toml: {svc_name} -> {proto_name}')

    # src files
    factory_subs = factory_subs or {}
    for root, _, fns in os.walk(os.path.join(crate_dir, 'src')):
        for fn in fns:
            if not fn.endswith('.rs'): continue
            fp = os.path.join(root, fn)
            c = open(fp, encoding='utf-8').read()
            orig = c
            # Replace factory calls first (before generic name swap)
            for svc_path, proto_path in factory_subs.items():
                c = c.replace(svc_path, proto_path)
            # Generic module name swap
            svc_mod = svc_name.replace('-', '_')
            proto_mod = proto_name.replace('-', '_') if proto_name else svc_mod
            c = c.replace(svc_mod + '::', proto_mod + '::')
            if c != orig:
                open(fp, 'w', encoding='utf-8').write(c)
                print(f'  Patched: {os.path.relpath(fp, ".")}')

# ====== 1. kb-svc: already has expert-proto, just drop expert-svc ======
print('=== kb-svc ===')
patch_crate(
    'platform/domains/kg/svc/mox-kb-svc',
    'mox-ai-expert-svc',
    None,
    factory_subs={
        'mox_ai_expert_svc::expert_traits::llm_consultant()':
            '/* DIP: inject ExpertConsultant from caller */ panic!("kb-svc requires injected ExpertConsultant")',
    }
)

# ====== 2. alliance-executor-svc ======
print('=== alliance-executor-svc ===')
patch_crate(
    'platform/domains/alliance/svc/mox-alliance-executor-svc',
    'mox-ai-expert-svc',
    'mox-ai-expert-proto',
    factory_subs={
        'mox_ai_expert_svc::llm::consultant::strict_llm_consultant_from_env':
            '/* inject consultant */ panic!("executor requires injected ExpertConsultant")',
    }
)

print('Done')
