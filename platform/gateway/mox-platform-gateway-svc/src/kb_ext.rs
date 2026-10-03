//! Knowledge entity endpoints have moved to mox_kb_svc::handlers.
//! Production routing uses modules::protected_kb_router for all KB endpoints.
//! This historical module has no routes or tenant-less persistent state.
//! Legacy kb_ext.entity_relations / data/kb_entity_relations.json remain untouched;
//! migration requires an explicit document/source/tenant ownership mapping.
