use anyhow::Result;
use fbc_starter::{ServiceEndpoint, get_load_balancer};
use tonic::Request;
use tracing::{debug, error, info};

// 引入生成的 gRPC 代码
pub mod health_proto {
    tonic::include_proto!("grpc.health.v1");
}

use health_proto::{HealthCheckRequest, HealthCheckResponse, health_client::HealthClient};

/// IM 服务健康检查客户端
pub struct ImHealthClient;

impl ImHealthClient {
    /// 检查 IM 服务健康状态（带轮询负载均衡）
    ///
    /// # 参数
    /// - `service_name`: Nacos 中注册的服务名称
    ///
    /// # 返回
    /// - `Ok(HealthCheckResponse)` - 健康检查响应
    /// - `Err` - 调用失败
    pub async fn check_health(service_name: &str) -> Result<HealthCheckResponse> {
        info!("🔍 开始检查 IM 服务健康状态: {}", service_name);

        // 使用 fbc-starter 提供的全局负载均衡器
        let balancer = get_load_balancer(service_name);
        let endpoint = balancer
            .next_endpoint()
            .ok_or_else(|| anyhow::anyhow!("未找到可用的服务实例: {}", service_name))?;

        info!(
            "🎯 轮询选择端点: {} (实例 ID: {})",
            endpoint.address, endpoint.instance_id
        );

        // 直接连接到选中的端点
        let channel = endpoint.endpoint.connect().await?;
        let mut client = HealthClient::new(channel);

        // 调用健康检查
        let request = Request::new(HealthCheckRequest {
            service: service_name.to_string(),
        });

        let response = client.check(request).await?;
        let health_response = response.into_inner();

        info!(
            "✅ 健康检查响应: status = {} (来自 {})",
            health_response.status, endpoint.instance_id
        );

        Ok(health_response)
    }

    /// 直接连接到指定端点进行健康检查（不使用 Nacos）
    ///
    /// # 参数
    /// - `endpoint`: 服务端点（如 "http://127.0.0.1:3000"）
    /// - `service_name`: 要检查的服务名称
    pub async fn check_health_direct(
        endpoint: &str,
        service_name: &str,
    ) -> Result<HealthCheckResponse> {
        info!("直接连接到端点进行健康检查: {}", endpoint);

        let channel = tonic::transport::Channel::from_shared(endpoint.to_string())?
            .connect()
            .await?;

        let mut client = HealthClient::new(channel);

        let request = Request::new(HealthCheckRequest {
            service: service_name.to_string(),
        });

        let response = client.check(request).await?;
        let health_response = response.into_inner();

        info!("健康检查响应: status = {}", health_response.status);

        Ok(health_response)
    }

    /// 检查多个服务实例的健康状态
    pub async fn check_all_instances(
        service_name: &str,
    ) -> Result<Vec<(ServiceEndpoint, HealthCheckResponse)>> {
        info!("检查所有服务实例的健康状态: {}", service_name);

        let endpoints = fbc_starter::get_service_endpoints(service_name);

        if endpoints.is_empty() {
            error!("未找到可用的服务实例: {}", service_name);
            anyhow::bail!("未找到可用的服务实例: {}", service_name);
        }

        let mut results = Vec::new();

        for endpoint in endpoints {
            debug!("检查实例: {}", endpoint.address);

            match Self::check_health_direct(&endpoint.address, service_name).await {
                Ok(response) => {
                    results.push((endpoint.clone(), response));
                }
                Err(e) => {
                    error!("实例 {} 健康检查失败: {}", endpoint.address, e);
                }
            }
        }

        Ok(results)
    }
}
