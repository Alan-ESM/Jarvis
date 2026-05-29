use crate::models::{AiRequest, AiResponse, ModelRoute, ModelTier};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};

#[async_trait]
pub trait AiProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn generate(&self, request: AiRequest) -> Result<AiResponse>;
}

#[derive(Default)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn AiProvider>>,
    routes: HashMap<ModelTier, ModelRoute>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_provider(&mut self, provider: Arc<dyn AiProvider>) {
        self.providers.insert(provider.name().to_string(), provider);
    }

    pub fn set_route(&mut self, route: ModelRoute) {
        self.routes.insert(route.tier, route);
    }

    pub async fn generate_for_tier(
        &self,
        tier: ModelTier,
        mut request: AiRequest,
    ) -> Result<AiResponse> {
        let route = self
            .routes
            .get(&tier)
            .ok_or_else(|| anyhow!("no route configured for tier {tier}"))?;
        let provider = self
            .providers
            .get(&route.provider)
            .ok_or_else(|| anyhow!("provider {} is not registered", route.provider))?;

        request.tier = tier;
        request.model = route.model.clone();
        provider.generate(request).await
    }
}
