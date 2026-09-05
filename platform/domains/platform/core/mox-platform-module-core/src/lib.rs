//! A host-independent composition policy. No HTTP, database, environment or domain service imports.
//! Hosts initialize in `ModulePlan::order`, then record actual outcomes with `Startup::ready/failed`.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub module: String,
    pub contract_major: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleSpec {
    pub id: String,
    pub contract_major: u16,
    /// Required for this deployment to become ready; optional modules may fail visibly.
    pub required: bool,
    pub dependencies: Vec<Dependency>,
    /// Exclusive route ownership. Authentication must still be installed by the host.
    pub route_prefix: Option<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ModuleError {
    #[error("invalid module specification: {0}")]
    Invalid(String),
    #[error("duplicate module: {0}")]
    Duplicate(String),
    #[error("module {module} requires missing module {dependency}")]
    Missing { module: String, dependency: String },
    #[error("module {module} requires {dependency} contract major {required}, installed {installed}")]
    Version { module: String, dependency: String, required: u16, installed: u16 },
    #[error("route ownership conflicts between {0} and {1}")]
    RouteConflict(String, String),
    #[error("module dependency cycle or cycle-dependent modules: {0:?}")]
    Cycle(Vec<String>),
    #[error("invalid module startup transition: {0}")]
    Transition(String),
}

#[derive(Debug, Clone)]
pub struct ModulePlan {
    modules: BTreeMap<String, ModuleSpec>,
    order: Vec<String>,
}

impl ModulePlan {
    pub fn new(specs: Vec<ModuleSpec>) -> Result<Self, ModuleError> {
        let mut modules = BTreeMap::<String, ModuleSpec>::new();
        for spec in specs {
            if spec.id.is_empty() || !spec.id.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-') || spec.contract_major == 0 {
                return Err(ModuleError::Invalid(spec.id));
            }
            if let Some(prefix) = &spec.route_prefix {
                if !prefix.starts_with('/') || prefix.ends_with('/') || prefix.contains("//") || prefix.contains(['?', '#', '*', ':', '%', '\\']) || prefix.split('/').any(|p| matches!(p, "." | "..")) {
                    return Err(ModuleError::Invalid(spec.id));
                }
                for other in modules.values() {
                    if let Some(path) = &other.route_prefix {
                        if path == prefix || path.starts_with(&format!("{prefix}/")) || prefix.starts_with(&format!("{path}/")) {
                            return Err(ModuleError::RouteConflict(other.id.clone(), spec.id));
                        }
                    }
                }
            }
            if modules.contains_key(&spec.id) { return Err(ModuleError::Duplicate(spec.id)); }
            modules.insert(spec.id.clone(), spec);
        }
        for spec in modules.values() {
            for dep in &spec.dependencies {
                let installed = modules.get(&dep.module).ok_or_else(|| ModuleError::Missing { module: spec.id.clone(), dependency: dep.module.clone() })?;
                if dep.contract_major != installed.contract_major {
                    return Err(ModuleError::Version { module: spec.id.clone(), dependency: dep.module.clone(), required: dep.contract_major, installed: installed.contract_major });
                }
            }
        }
        let mut order = Vec::new();
        let mut resolved = BTreeSet::new();
        while order.len() < modules.len() {
            let next = modules.values().find(|s| !resolved.contains(&s.id) && s.dependencies.iter().all(|d| resolved.contains(&d.module)));
            match next {
                Some(spec) => { resolved.insert(spec.id.clone()); order.push(spec.id.clone()); }
                None => return Err(ModuleError::Cycle(modules.keys().filter(|id| !resolved.contains(*id)).cloned().collect())),
            }
        }
        Ok(Self { modules, order })
    }
    pub fn order(&self) -> &[String] { &self.order }
    pub fn spec(&self, id: &str) -> Option<&ModuleSpec> { self.modules.get(id) }
    pub fn startup(self) -> Startup {
        let states = self.modules.keys().map(|id| (id.clone(), ModuleState::Pending)).collect();
        Startup { plan: self, states }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ModuleState { Pending, Ready, Failed { message: String } }

#[derive(Debug, Clone, Serialize)]
pub struct StartupReport {
    pub ready: bool,
    pub degraded: bool,
    pub modules: BTreeMap<String, ModuleState>,
}

pub struct Startup {
    plan: ModulePlan,
    states: BTreeMap<String, ModuleState>,
}
impl Startup {
    pub fn can_start(&self, id: &str) -> Result<(), ModuleError> {
        let spec = self.plan.spec(id).ok_or_else(|| ModuleError::Transition(id.into()))?;
        if self.states.get(id) != Some(&ModuleState::Pending) { return Err(ModuleError::Transition(id.into())); }
        if spec.dependencies.iter().any(|d| self.states.get(&d.module) != Some(&ModuleState::Ready)) {
            return Err(ModuleError::Transition(format!("{id}: dependencies not ready")));
        }
        Ok(())
    }
    pub fn ready(&mut self, id: &str) -> Result<(), ModuleError> {
        self.can_start(id)?;
        self.finish(id, ModuleState::Ready)
    }
    pub fn failed(&mut self, id: &str, message: impl Into<String>) -> Result<(), ModuleError> {
        self.finish(id, ModuleState::Failed { message: message.into() })
    }
    fn finish(&mut self, id: &str, state: ModuleState) -> Result<(), ModuleError> {
        if self.states.get(id) != Some(&ModuleState::Pending) { return Err(ModuleError::Transition(id.into())); }
        self.states.insert(id.into(), state);
        Ok(())
    }
    pub fn report(&self) -> StartupReport {
        StartupReport {
            ready: self.plan.modules.values().filter(|s| s.required).all(|s| self.states.get(&s.id) == Some(&ModuleState::Ready)),
            degraded: self.states.values().any(|s| s != &ModuleState::Ready),
            modules: self.states.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spec(id: &str, deps: &[&str], required: bool) -> ModuleSpec {
        ModuleSpec { id: id.into(), contract_major: 1, required, route_prefix: Some(format!("/{id}")), dependencies: deps.iter().map(|id| Dependency { module: (*id).into(), contract_major: 1 }).collect() }
    }
    #[test]
    fn plans_independently_of_registration_order() {
        let plan = ModulePlan::new(vec![spec("tasks", &["identity"], true), spec("identity", &[], true)]).unwrap();
        assert_eq!(plan.order(), &["identity", "tasks"]);
        let mut run = plan.startup();
        assert!(!run.report().ready);
        assert!(run.ready("tasks").is_err());
        run.ready("identity").unwrap(); run.ready("tasks").unwrap();
        assert!(run.report().ready); assert!(!run.report().degraded);
        assert!(run.failed("tasks", "late failure").is_err());
    }
    #[test]
    fn rejects_missing_incompatible_and_cyclic_dependencies() {
        assert!(matches!(ModulePlan::new(vec![spec("tasks", &["identity"], true)]), Err(ModuleError::Missing { .. })));
        let mut identity = spec("identity", &[], true); identity.contract_major = 2;
        assert!(matches!(ModulePlan::new(vec![spec("tasks", &["identity"], true), identity]), Err(ModuleError::Version { .. })));
        assert!(matches!(ModulePlan::new(vec![spec("a", &["b"], true), spec("b", &["a"], true)]), Err(ModuleError::Cycle(_))));
    }
    #[test]
    fn optional_failure_does_not_hide_a_required_dependency_failure() {
        let mut run = ModulePlan::new(vec![spec("tasks", &["identity"], true), spec("identity", &[], false)]).unwrap().startup();
        run.failed("identity", "unavailable").unwrap();
        assert!(run.ready("tasks").is_err()); assert!(!run.report().ready);
        let mut run = ModulePlan::new(vec![spec("tasks", &[], true), spec("search", &[], false)]).unwrap().startup();
        run.ready("tasks").unwrap(); run.failed("search", "unavailable").unwrap();
        assert!(run.report().ready); assert!(run.report().degraded);
    }
    #[test]
    fn rejects_duplicate_identity_and_route_shadowing() {
        let mut duplicate = spec("tasks", &[], true); duplicate.route_prefix = None;
        assert!(matches!(ModulePlan::new(vec![spec("tasks", &[], true), duplicate]), Err(ModuleError::Duplicate(_))));
        let mut nested = spec("nested", &[], true); nested.route_prefix = Some("/tasks/private".into());
        assert!(matches!(ModulePlan::new(vec![spec("tasks", &[], true), nested]), Err(ModuleError::RouteConflict(..))));
        assert!(ModulePlan::new(vec![spec("task", &[], true), spec("tasks", &[], true)]).is_ok());
    }
}
