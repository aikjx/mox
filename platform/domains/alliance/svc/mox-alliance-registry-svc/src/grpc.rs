// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! gRPC 服务实现

use tonic::{Request, Response, Status};

// 生成的 proto 代码
pub mod registry_proto {
    tonic::include_proto!("mox.alliance.registry.v1");
}

use registry_proto::{
    expert_registry_server::ExpertRegistry,
    CreateExpertRequest, DeleteExpertRequest, Expert, ExpertMetrics,
    GetExpertRequest, ListExpertsRequest, ListExpertsResponse, PlatformOverview, UpdateExpertRequest,
};

use crate::storage::ExpertStore;

/// gRPC 服务实现
pub struct RegistryGrpcService {
    store: std::sync::Arc<ExpertStore>,
}

impl RegistryGrpcService {
    pub fn new(store: std::sync::Arc<ExpertStore>) -> Self {
        Self { store }
    }
}

#[tonic::async_trait]
impl ExpertRegistry for RegistryGrpcService {
    async fn list_experts(
        &self,
        request: Request<ListExpertsRequest>,
    ) -> Result<Response<ListExpertsResponse>, Status> {
        let req = request.into_inner();
        let experts = self.store.list()
            .map_err(|e| Status::internal(e.to_string()))?;

        let grpc_experts: Vec<Expert> = experts.into_iter().map(|e| Expert {
            id: e.id,
            name: e.name,
            title: e.title,
            organization: e.organization,
            domains: e.domains,
            skills: e.skills,
            bio: e.bio,
            enabled: e.enabled,
            rating: e.rating as f64,
            total_consultations: e.total_consultations as i32,
        }).collect();

        let total = grpc_experts.len() as i32;
        Ok(Response::new(ListExpertsResponse {
            experts: grpc_experts,
            total,
        }))
    }

    async fn get_expert(
        &self,
        request: Request<GetExpertRequest>,
    ) -> Result<Response<Expert>, Status> {
        let req = request.into_inner();
        let expert = self.store.get(&req.id)
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("expert not found"))?;

        Ok(Response::new(Expert {
            id: expert.id,
            name: expert.name,
            title: expert.title,
            organization: expert.organization,
            domains: expert.domains,
            skills: expert.skills,
            bio: expert.bio,
            enabled: expert.enabled,
            rating: expert.rating as f32,
            total_consultations: expert.total_consultations as u32,
        }))
    }

    async fn create_expert(
        &self,
        request: Request<CreateExpertRequest>,
    ) -> Result<Response<Expert>, Status> {
        use crate::models::Expert;
        let req = request.into_inner();
        let mut expert = Expert::new(req.name);
        if let Some(title) = req.title { expert.title = title; }
        if let Some(org) = req.organization { expert.organization = org; }
        expert.domains = req.domains;
        expert.skills = req.skills;
        if let Some(bio) = req.bio { expert.bio = bio; }

        self.store.create(&expert)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(Expert {
            id: expert.id,
            name: expert.name,
            title: expert.title,
            organization: expert.organization,
            domains: expert.domains,
            skills: expert.skills,
            bio: expert.bio,
            enabled: expert.enabled,
            rating: expert.rating as f32,
            total_consultations: expert.total_consultations as u32,
        }))
    }

    async fn update_expert(
        &self,
        request: Request<UpdateExpertRequest>,
    ) -> Result<Response<Expert>, Status> {
        let req = request.into_inner();
        let mut expert = self.store.get(&req.id)
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("expert not found"))?;

        if let Some(name) = req.name { expert.name = name; }
        if let Some(title) = req.title { expert.title = title; }
        if let Some(org) = req.organization { expert.organization = org; }
        if !req.domains.is_empty() { expert.domains = req.domains; }
        if !req.skills.is_empty() { expert.skills = req.skills; }
        if let Some(bio) = req.bio { expert.bio = bio; }
        if let Some(enabled) = req.enabled { expert.enabled = enabled; }

        self.store.update(&expert)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(Expert {
            id: expert.id,
            name: expert.name,
            title: expert.title,
            organization: expert.organization,
            domains: expert.domains,
            skills: expert.skills,
            bio: expert.bio,
            enabled: expert.enabled,
            rating: expert.rating as f32,
            total_consultations: expert.total_consultations as u32,
        }))
    }

    async fn delete_expert(
        &self,
        request: Request<DeleteExpertRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        self.store.delete(&req.id)
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(()))
    }

    async fn get_expert_metrics(
        &self,
        request: Request<GetExpertRequest>,
    ) -> Result<Response<ExpertMetrics>, Status> {
        let req = request.into_inner();
        let expert = self.store.get(&req.id)
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("expert not found"))?;

        Ok(Response::new(ExpertMetrics {
            expert_id: expert.id,
            avg_rating: expert.rating as f32,
            total_consultations: expert.total_consultations as u32,
            success_rate: 0.95,  // 预留
            avg_latency_ms: 120.0, // 预留
        }))
    }

    async fn get_platform_overview(
        &self,
        _request: Request<()>,
    ) -> Result<Response<PlatformOverview>, Status> {
        let experts = self.store.list()
            .map_err(|e| Status::internal(e.to_string()))?;

        let total = experts.len() as i32;
        let active = experts.iter().filter(|e| e.enabled).count() as i32;
        let domains: std::collections::HashSet<&String> = experts.iter()
            .flat_map(|e| &e.domains)
            .collect();

        Ok(Response::new(PlatformOverview {
            total_experts: total,
            active_experts: active,
            total_domains: domains.len() as i32,
            total_consultations: 0, // 预留
        }))
    }
}
