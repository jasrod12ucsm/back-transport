use crate::types::TenantId;
use async_nats::jetstream;
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info, warn};

#[derive(Debug, Error)]
pub enum EventError {
    #[error("NATS error: {0}")]
    NatsError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Handler error: {0}")]
    HandlerError(String),
}

// Implementar From para errores de NATS
impl From<async_nats::ConnectError> for EventError {
    fn from(err: async_nats::ConnectError) -> Self {
        EventError::NatsError(format!("Connect error: {:?}", err))
    }
}

impl From<async_nats::PublishError> for EventError {
    fn from(err: async_nats::PublishError) -> Self {
        EventError::NatsError(format!("Publish error: {:?}", err))
    }
}

impl From<async_nats::jetstream::context::CreateStreamError> for EventError {
    fn from(err: async_nats::jetstream::context::CreateStreamError) -> Self {
        EventError::NatsError(format!("Create stream error: {:?}", err))
    }
}

impl From<async_nats::jetstream::stream::ConsumerError> for EventError {
    fn from(err: async_nats::jetstream::stream::ConsumerError) -> Self {
        EventError::NatsError(format!("Consumer error: {:?}", err))
    }
}

impl From<async_nats::jetstream::context::GetStreamError> for EventError {
    fn from(err: async_nats::jetstream::context::GetStreamError) -> Self {
        EventError::NatsError(format!("Get stream error: {:?}", err))
    }
}

impl From<async_nats::jetstream::context::PublishError> for EventError {
    fn from(err: async_nats::jetstream::context::PublishError) -> Self {
        EventError::NatsError(format!("Jetstream publish error: {:?}", err))
    }
}

impl From<async_nats::jetstream::stream::InfoError> for EventError {
    fn from(err: async_nats::jetstream::stream::InfoError) -> Self {
        EventError::NatsError(format!("Stream info error: {:?}", err))
    }
}

/// Evento de database de tenant creada
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantDatabaseCreatedEvent {
    pub tenant_id: TenantId,
    pub tenant_name: String,
    pub database_name: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Evento de database de tenant actualizada
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantDatabaseUpdatedEvent {
    pub tenant_id: TenantId,
    pub database_name: String,
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
    pub status_changed: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Evento de database de tenant desactivada
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantDatabaseDeactivatedEvent {
    pub tenant_id: TenantId,
    pub database_name: String,
    pub reason: String,
    pub deactivated_at: chrono::DateTime<chrono::Utc>,
}

/// Evento de tenant completo creado (todas sus databases)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantCreatedEvent {
    pub tenant_id: TenantId,
    pub tenant_name: String,
    pub databases: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Evento de tenant completamente desactivado (todas sus databases)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantDeactivatedEvent {
    pub tenant_id: TenantId,
    pub reason: String,
    pub deactivated_at: chrono::DateTime<chrono::Utc>,
}

/// Enum de todos los eventos de tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TenantEvent {
    // Eventos de tenant completo
    TenantCreated(TenantCreatedEvent),
    TenantDeactivated(TenantDeactivatedEvent),

    // Eventos de database específica
    DatabaseCreated(TenantDatabaseCreatedEvent),
    DatabaseUpdated(TenantDatabaseUpdatedEvent),
    DatabaseDeactivated(TenantDatabaseDeactivatedEvent),
}

/// Publisher de eventos a NATS
pub struct TenantEventPublisher {
    context: jetstream::Context,
    stream_name: String,
}

impl TenantEventPublisher {
    pub async fn new(nats_url: &str, stream_name: &str) -> Result<Self, EventError> {
        let client = async_nats::connect(nats_url).await?;
        let jetstream = jetstream::new(client);

        // Crear o obtener stream
        let _stream = jetstream
            .get_or_create_stream(jetstream::stream::Config {
                name: stream_name.to_string(),
                subjects: vec![format!("{}.*", stream_name)],
                max_messages: 10_000,
                retention: jetstream::stream::RetentionPolicy::Limits,
                ..Default::default()
            })
            .await?;

        Ok(Self {
            context: jetstream,
            stream_name: stream_name.to_string(),
        })
    }

    /// Publica evento TenantCreated (tenant completo con todas sus databases)
    pub async fn publish_tenant_created(
        &self,
        event: TenantCreatedEvent,
    ) -> Result<(), EventError> {
        let subject = format!("{}.tenant_created", self.stream_name);
        self.publish(subject, TenantEvent::TenantCreated(event))
            .await
    }

    /// Publica evento TenantDeactivated (tenant completo)
    pub async fn publish_tenant_deactivated(
        &self,
        event: TenantDeactivatedEvent,
    ) -> Result<(), EventError> {
        let subject = format!("{}.tenant_deactivated", self.stream_name);
        self.publish(subject, TenantEvent::TenantDeactivated(event))
            .await
    }

    /// Publica evento DatabaseCreated (database específica de un tenant)
    pub async fn publish_database_created(
        &self,
        event: TenantDatabaseCreatedEvent,
    ) -> Result<(), EventError> {
        let subject = format!("{}.database_created", self.stream_name);
        self.publish(subject, TenantEvent::DatabaseCreated(event))
            .await
    }

    /// Publica evento DatabaseUpdated (database específica actualizada)
    pub async fn publish_database_updated(
        &self,
        event: TenantDatabaseUpdatedEvent,
    ) -> Result<(), EventError> {
        let subject = format!("{}.database_updated", self.stream_name);
        self.publish(subject, TenantEvent::DatabaseUpdated(event))
            .await
    }

    /// Publica evento DatabaseDeactivated (database específica desactivada)
    pub async fn publish_database_deactivated(
        &self,
        event: TenantDatabaseDeactivatedEvent,
    ) -> Result<(), EventError> {
        let subject = format!("{}.database_deactivated", self.stream_name);
        self.publish(subject, TenantEvent::DatabaseDeactivated(event))
            .await
    }

    async fn publish(&self, subject: String, event: TenantEvent) -> Result<(), EventError> {
        let payload = serde_json::to_vec(&event)?;

        self.context
            .publish(subject.clone(), payload.into())
            .await?
            .await?;

        info!(subject = %subject, "Event published");
        Ok(())
    }
}

impl Clone for TenantEventPublisher {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
            stream_name: self.stream_name.clone(),
        }
    }
}

/// Trait para manejar eventos de tenant
#[async_trait]
pub trait TenantEventHandler: Send + Sync {
    /// Tenant completo creado (con todas sus databases)
    async fn on_tenant_created(
        &self,
        event: &TenantCreatedEvent,
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// Tenant completo desactivado (todas sus databases)
    async fn on_tenant_deactivated(
        &self,
        event: &TenantDeactivatedEvent,
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// Database específica creada
    async fn on_database_created(
        &self,
        event: &TenantDatabaseCreatedEvent,
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// Database específica actualizada (invalidar cache)
    async fn on_database_updated(
        &self,
        event: &TenantDatabaseUpdatedEvent,
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// Database específica desactivada
    async fn on_database_deactivated(
        &self,
        event: &TenantDatabaseDeactivatedEvent,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

/// Subscriber de eventos NATS
pub struct TenantEventSubscriber {
    consumer: jetstream::consumer::PullConsumer,
}

impl TenantEventSubscriber {
    pub async fn new(
        nats_url: &str,
        stream_name: &str,
        consumer_name: &str,
    ) -> Result<Self, EventError> {
        let client = async_nats::connect(nats_url).await?;
        let jetstream = jetstream::new(client);

        let stream = jetstream.get_stream(stream_name).await?;

        let consumer = stream
            .get_or_create_consumer(
                consumer_name,
                jetstream::consumer::pull::Config {
                    durable_name: Some(consumer_name.to_string()),
                    filter_subject: format!("{}.*", stream_name),
                    ack_policy: jetstream::consumer::AckPolicy::Explicit,
                    ..Default::default()
                },
            )
            .await?;

        Ok(Self { consumer })
    }

    /// Suscribe al stream y procesa eventos
    pub async fn subscribe<H>(self, handler: Arc<H>) -> Result<(), EventError>
    where
        H: TenantEventHandler + 'static,
    {
        use async_nats::jetstream::consumer::pull::Stream;

        let mut messages: Stream = self
            .consumer
            .messages()
            .await
            .map_err(|e| EventError::NatsError(format!("Failed to get messages: {:?}", e)))?;

        info!("Subscribed to tenant events");

        loop {
            match messages.next().await {
                Some(Ok(msg)) => {
                    let event: Result<TenantEvent, _> = serde_json::from_slice(&msg.payload);

                    match event {
                        Ok(TenantEvent::TenantCreated(data)) => {
                            info!(
                                tenant_id = %data.tenant_id,
                                databases = ?data.databases,
                                "Received TenantCreated event"
                            );

                            if let Err(e) = handler.on_tenant_created(&data).await {
                                error!(error = %e, "Failed to handle TenantCreated");
                            }
                        }
                        Ok(TenantEvent::TenantDeactivated(data)) => {
                            info!(
                                tenant_id = %data.tenant_id,
                                "Received TenantDeactivated event"
                            );

                            if let Err(e) = handler.on_tenant_deactivated(&data).await {
                                error!(error = %e, "Failed to handle TenantDeactivated");
                            }
                        }
                        Ok(TenantEvent::DatabaseCreated(data)) => {
                            info!(
                                tenant_id = %data.tenant_id,
                                database = %data.database_name,
                                "Received DatabaseCreated event"
                            );

                            if let Err(e) = handler.on_database_created(&data).await {
                                error!(error = %e, "Failed to handle DatabaseCreated");
                            }
                        }
                        Ok(TenantEvent::DatabaseUpdated(data)) => {
                            info!(
                                tenant_id = %data.tenant_id,
                                database = %data.database_name,
                                "Received DatabaseUpdated event"
                            );

                            if let Err(e) = handler.on_database_updated(&data).await {
                                error!(error = %e, "Failed to handle DatabaseUpdated");
                            }
                        }
                        Ok(TenantEvent::DatabaseDeactivated(data)) => {
                            info!(
                                tenant_id = %data.tenant_id,
                                database = %data.database_name,
                                "Received DatabaseDeactivated event"
                            );

                            if let Err(e) = handler.on_database_deactivated(&data).await {
                                error!(error = %e, "Failed to handle DatabaseDeactivated");
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "Failed to parse tenant event");
                        }
                    }

                    // Acknowledge message
                    if let Err(e) = msg.ack().await {
                        error!(error = %e, "Failed to ack message");
                    }
                }
                Some(Err(e)) => {
                    error!(error = ?e, "Error receiving message");
                }
                None => {
                    warn!("Message stream ended");
                    break;
                }
            }
        }

        Ok(())
    }
}

/// Helper para spawn subscriber en background
pub fn spawn_subscriber<H>(subscriber: TenantEventSubscriber, handler: Arc<H>)
where
    H: TenantEventHandler + 'static,
{
    tokio::spawn(async move {
        if let Err(e) = subscriber.subscribe(handler).await {
            error!(error = %e, "Event subscriber error");
        }
    });
}
