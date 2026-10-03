"""Read the existing capability authority, workspace and docs; never infer readiness from counts."""
import hashlib
import json
import re
from pathlib import Path
folder = Path(__file__).resolve().parent
root = folder.parents[2]
registry = json.loads((root / "docs/modules/enterprise-capabilities/registry.json").read_text(encoding="utf-8-sig"))
metadata = json.loads((folder / "metadata.log").read_text(encoding="utf-8"))
members = set(metadata["workspace_members"])
packages = [p for p in metadata["packages"] if p["id"] in members]
modules = []
for module in registry["modules"]:
    authority = root / module["authority"]
    content = authority.read_text(encoding="utf-8-sig")
    modules.append({"id":module["id"],"name":module["name"],"authority":module["authority"],
        "authority_sha256":hashlib.sha256(authority.read_bytes()).hexdigest(),
        "authority_headings":re.findall(r"^#{1,3} .+$",content,re.M)[:20],
        "depends_on":module["depends_on"],"owners":module["owners"],
        "flow":module["flow"],"requirements":module["requirements"],
        "implementation_status":"requires_per_requirement_real_verification", "whole_module_accepted_this_batch":False})
candidate_paths = []
for base in [root / "platform/gateway/mox-platform-gateway-svc/src", root / "platform/domains"]:
    for path in sorted(base.rglob("*.rs")):
        if any(part in ("target","legacy") for part in path.parts): continue
        content = path.read_text(encoding="utf-8",errors="replace")
        for number,line in enumerate(content.splitlines(),1):
            if re.search(r"\b(?:unimplemented!|todo!)\s*\(|\b(?:api_error|err)\s*\(\s*501\b",line):
                candidate_paths.append({"path":str(path.relative_to(root)).replace("\\","/"),"line":number,
                    "kind":"unimplemented_or_501_candidate","excerpt":line.strip()[:220]})
summary = {"capability_authority":"docs/modules/enterprise-capabilities/registry.json",
    "modules":modules,"module_count":len(modules),"requirements":sum(len(m["requirements"]) for m in modules),
    "workspace_packages":len(packages),"workspace_default_members":len(metadata["workspace_default_members"]),
    "packages":[{"name":p["name"],"manifest_path":p["manifest_path"]} for p in packages],
    "candidates":candidate_paths,"candidate_interpretation":"Static review candidates only, not proven defects or completed functionality",
    "accepted_whole_modules_this_batch":0,"purpose":"Snapshot evidence; not a second requirements or production state authority"}
(folder / "architecture-audit.json").write_text(json.dumps(summary,ensure_ascii=False,indent=2),encoding="utf-8")
print("Read module authorities:",len(modules),"requirements:",summary["requirements"],"workspace packages:",len(packages),"review candidates:",len(candidate_paths))
