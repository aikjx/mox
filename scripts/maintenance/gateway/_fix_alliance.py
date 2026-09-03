fpath = r'D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance.rs'
with open(fpath, encoding='utf-8', errors='replace') as f:
    content = f.read()

fixes = []

# Fix 1: Add ExpertMatcher trait import
old_import = 'use mox_alliance_scheduler_core::{InMemoryTaskRepository, RuleBasedExpertMatcher, TaskRepository};'
new_import = 'use mox_alliance_scheduler_core::{ExpertMatcher, InMemoryTaskRepository, RuleBasedExpertMatcher, TaskRepository};'
if old_import in content:
    content = content.replace(old_import, new_import, 1)
    fixes.append('Added ExpertMatcher import')

# Fix 2: ExpertHealth::Healthy -> ExpertHealth::default()
n = content.count('mox_alliance_common_proto::ExpertHealth::Healthy')
if n > 0:
    content = content.replace('mox_alliance_common_proto::ExpertHealth::Healthy', 'ExpertHealth::default()')
    fixes.append(f'Replaced ExpertHealth::Healthy x{n}')

# Fix 3: AllianceMode match - add missing Debate, Voting variants
old_mode = '''fn mode_str(m: AllianceMode) -> &'static str {
    match m {
        AllianceMode::Sequential => "single_expert",
        AllianceMode::Parallel => "expert_alliance",
        AllianceMode::Iterative => "human_in_loop",
        AllianceMode::Hierarchical => "autonomous",
    }
}'''
new_mode = '''fn mode_str(m: AllianceMode) -> &'static str {
    match m {
        AllianceMode::Sequential => "single_expert",
        AllianceMode::Parallel => "expert_alliance",
        AllianceMode::Iterative => "human_in_loop",
        AllianceMode::Hierarchical => "autonomous",
        AllianceMode::Debate => "debate",
        AllianceMode::Voting => "voting",
    }
}'''
if old_mode in content:
    content = content.replace(old_mode, new_mode, 1)
    fixes.append('Fixed AllianceMode match')

# Fix 4: FusionStrategy match - add missing variants
old_fusion = '''fn fusion_strategy_str(f: FusionStrategy) -> &'static str {
    match f {
        FusionStrategy::BestOf => "first_wins",
        FusionStrategy::Weighted => "weighted_voting",
        FusionStrategy::Voting => "rrf",
        FusionStrategy::ConfidenceWeighted => "llm_judge",
        FusionStrategy::Concatenation => "consensus",
    }
}'''
new_fusion = '''fn fusion_strategy_str(f: FusionStrategy) -> &'static str {
    match f {
        FusionStrategy::BestOf => "first_wins",
        FusionStrategy::Weighted => "weighted_voting",
        FusionStrategy::Voting => "rrf",
        FusionStrategy::ConfidenceWeighted => "llm_judge",
        FusionStrategy::Concatenation => "consensus",
        FusionStrategy::Stacking => "stacking",
        FusionStrategy::Debate => "debate",
        FusionStrategy::MapReduce => "map_reduce",
        FusionStrategy::Iterative => "iterative",
    }
}'''
if old_fusion in content:
    content = content.replace(old_fusion, new_fusion, 1)
    fixes.append('Fixed FusionStrategy match')

# Fix 5: ExpertStatus variants
content = content.replace('ExpertStatus::Offline => "offline"', 'ExpertStatus::Inactive => "offline"')
content = content.replace('ExpertStatus::Busy => "busy"', 'ExpertStatus::Maintenance => "busy"')
content = content.replace('ExpertStatus::Error => "error"', 'ExpertStatus::Deprecated => "error"')
fixes.append('Fixed ExpertStatus variants')

# Fix 6: Debug trait - check if AllianceGatewayState derives Debug
# We'll handle this by checking if there's a derive(Debug) on the state struct
# and removing it if the fields don't implement Debug

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

print('Fixes applied:')
for f in fixes:
    print(f'  - {f}')
