# AuthorWorks Event Communication Standard

## Overview

This document defines the standard for event-driven communication across AuthorWorks microservices. A consistent approach to event communication is essential for maintaining system reliability, scalability, and observability.

## Event Transport

AuthorWorks standardizes on **Apache Kafka** as the primary event transport mechanism. In specific cases where real-time updates are required (e.g., collaborative editing), WebSockets may be used as a supplementary transport.

### Kafka Configuration

- **Brokers**: Minimum of 3 brokers for fault tolerance
- **Topics**: Named according to service and event type
- **Partitioning**: Based on resource ID to ensure ordered processing
- **Retention**: Configurable per topic, default 7 days
- **Replication Factor**: 3 for production topics

## Event Schema

All events must adhere to a standard schema:

```json
{
  "meta": {
    "id": "unique-event-identifier",
    "type": "service.resource.action",
    "version": "1.0",
    "timestamp": "2023-06-15T14:22:31.123Z",
    "correlation_id": "original-request-id",
    "source": "source-service-name",
    "trace_context": {
      "trace_id": "distributed-tracing-identifier",
      "span_id": "span-identifier"
    }
  },
  "data": {
    // Event-specific payload
  }
}
```

### Event Type Naming Convention

Event types follow the pattern: `service.resource.action`

Examples:
- `user.account.created`
- `content.book.published`
- `subscription.payment.succeeded`

### Event Versioning

- Events are versioned independently of API versions
- Version is included in the event metadata
- Breaking changes require a new version
- Services should support at least the previous version for backward compatibility

## Producer Responsibilities

Event producers must:

1. Generate a unique event ID (UUID v4)
2. Include accurate timestamp in UTC
3. Set appropriate event type
4. Include correlation ID from the originating request
5. Ensure event schema validity
6. Handle publishing failures with appropriate retry logic
7. Log publishing attempts and failures

## Consumer Responsibilities

Event consumers must:

1. Validate event schema before processing
2. Handle duplicate events idempotently
3. Implement appropriate error handling
4. Track processing status
5. Propagate correlation IDs and trace context
6. Log consumption and processing outcomes

## Delivery Guarantees

The following delivery semantics apply:

- **Publishing**: At-least-once delivery
- **Processing**: Exactly-once semantics where possible
- **Ordering**: Guaranteed for events with the same partition key

## Error Handling

### Producer Errors

1. Implement retry with exponential backoff for publishing failures
2. After maximum retries, log detailed error and alert
3. Consider dead letter queue for undeliverable events

### Consumer Errors

1. For transient failures: retry with exponential backoff
2. For permanent failures (e.g., schema validation): send to dead letter topic
3. Log detailed error information including event ID
4. Alert on repeated failures

## Dead Letter Handling

1. Each service maintains a dead letter topic: `service.deadletter`
2. Original event is wrapped with error context
3. Administrative interface for reviewing and reprocessing
4. Retention period of 30 days for dead letter topics

## Event Types Registry

All event types must be registered in a central event catalog with:

1. Event name and version
2. Schema definition
3. Producing service
4. Expected consumers
5. Description and purpose
6. Sample event

## WebSocket Standards

For real-time updates via WebSockets:

1. Connection endpoint format: `/v1/{service}/ws`
2. Authentication via JWT in connection request
3. Message format matches Kafka event schema
4. Reconnection strategy with exponential backoff
5. Heartbeat mechanism every 30 seconds

## Example Implementation

### Producer (Rust)

```rust
#[derive(Serialize)]
struct EventMetadata {
    id: String,
    type_name: String,
    version: String,
    timestamp: String,
    correlation_id: String,
    source: String,
    trace_context: TraceContext,
}

#[derive(Serialize)]
struct TraceContext {
    trace_id: String,
    span_id: String,
}

#[derive(Serialize)]
struct Event<T> {
    meta: EventMetadata,
    data: T,
}

async fn publish_event<T: Serialize>(
    producer: &FutureProducer,
    event_type: &str,
    correlation_id: &str,
    data: T,
    partition_key: &str,
) -> Result<(), KafkaError> {
    let event = Event {
        meta: EventMetadata {
            id: Uuid::new_v4().to_string(),
            type_name: event_type.to_string(),
            version: "1.0".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            correlation_id: correlation_id.to_string(),
            source: "service-name".to_string(),
            trace_context: get_trace_context(),
        },
        data,
    };
    
    let event_json = serde_json::to_string(&event)?;
    let topic = event_type.split('.').next().unwrap_or("default");
    
    producer
        .send(
            FutureRecord::to(topic)
                .payload(&event_json)
                .key(partition_key),
            Duration::from_secs(5),
        )
        .await
        .map(|_| ())
        .map_err(|(e, _)| e)
}
```

### Consumer (Rust)

```rust
async fn consume_events(consumer: &StreamConsumer, processor: impl EventProcessor) {
    let mut message_stream = consumer.stream();
    
    while let Some(message) = message_stream.next().await {
        match message {
            Ok(msg) => {
                let payload = match msg.payload() {
                    Some(p) => p,
                    None => {
                        log::warn!("Empty payload");
                        consumer.commit_message(&msg, CommitMode::Async).await.unwrap();
                        continue;
                    }
                };
                
                match process_event(payload, &processor).await {
                    Ok(_) => {
                        consumer.commit_message(&msg, CommitMode::Async).await.unwrap();
                    }
                    Err(e) => {
                        log::error!("Failed to process event: {:?}", e);
                        // Handle based on error type
                    }
                }
            }
            Err(e) => {
                log::error!("Kafka error: {:?}", e);
            }
        }
    }
}

async fn process_event(payload: &[u8], processor: &impl EventProcessor) -> Result<(), EventError> {
    // Parse event, validate schema, and process
}
```

## Monitoring and Observability

For effective monitoring of the event system:

1. Track message rates (produced/consumed) per topic
2. Monitor consumer lag
3. Track failed deliveries and dead letter queue size
4. Implement distributed tracing across event boundaries
5. Set up alerts for abnormal patterns

## Testing Strategies

When testing event-driven components:

1. Unit test serialization and deserialization
2. Use in-memory Kafka for integration tests
3. Create event producer/consumer test doubles
4. Test retry logic and error handling
5. Verify idempotent processing

## Implementation Roadmap

Services should implement this standard according to the following phases:

1. **Phase 1**: Adapt existing event producers to the new schema
2. **Phase 2**: Update event consumers to handle both old and new formats
3. **Phase 3**: Implement monitoring and observability
4. **Phase 4**: Migrate all services to the new standard
5. **Phase 5**: Remove support for legacy formats 