import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

folder = Path(__file__).resolve().parent
root = folder.parents[2]
checks = {
    'paths_selftest': ['python', 'scripts/gate/check-script-paths.py', '--selftest'],
    'paths': ['python', 'scripts/gate/check-script-paths.py'],
    'docs': ['python', 'scripts/gate/check-doc-links.py'],
    'ports': ['python', 'scripts/gate/verify-ports.py'],
    'frontend_module': ['python', 'scripts/gate/check-frontend-module.py'],
    'api_bindings': ['python', 'frontend-ui/scripts/gate/check-api-binding-kinds.py', '--check'],
    'feedback_imports': ['python', 'frontend-ui/scripts/gate/check-ep-feedback-imports.py', '--check'],
    'framework_imports': ['python', 'frontend-ui/scripts/gate/check-framework-imports.py', '--check'],
    'theme': ['python', 'frontend-ui/scripts/gate/check-theme-tokens.py', '--check'],
    'locale': ['python', 'scripts/gate/check-locale-format-outlets.py'],
    'catalog': ['python', 'scripts/registry/module_catalog.py', '--check'],
    'formulas': ['python', 'scripts/gate/check-doc-formulas.py', '--selftest'],
    'api_surface': ['python', 'scripts/gate/check-api-surface.py', '--selftest'],
    'diagrams': ['node', 'reports/data/20261004-kb-entity-normalization/parse-diagrams.mjs'],
    'node_transport': ['node', '--test', 'frontend-ui/src/modules/expert-alliance/contract/event-stream.node-test.mjs'],
    'diff': ['git', 'diff', '--check'],
    'format': ['rustfmt', '--edition', '2021', '--check', 'platform/domains/kg/svc/mox-kb-svc/src/entity.rs', 'platform/domains/kg/svc/mox-kb-svc/tests/entity_associations.rs', 'platform/gateway/mox-platform-gateway-svc/tests/kb_entity_gateway.rs'],
}

def run(item):
    name, command = item
    result = subprocess.run([sys.executable, str(folder / 'run_check.py'), name] + command,
                            cwd=str(root), stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print(name, result.returncode, flush=True)
    if result.returncode:
        sys.stdout.buffer.write(result.stdout[-1400:])
    return result.returncode

status = run(('paths_selftest', checks.pop('paths_selftest')))
with ThreadPoolExecutor(max_workers=4) as pool:
    statuses = list(pool.map(run, checks.items()))
sys.exit(bool(status or any(statuses)))
