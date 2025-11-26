# Testing with Mockall and Axum

This project uses a trait-based approach to enable testing with mockall while maintaining clean, readable code.

## Architecture

### AppStateOperations Trait

The `AppStateOperations` trait (in `src/app.rs`) defines the interface for state operations:

```rust
#[automock]
pub trait AppStateOperations {
    fn config(&self) -> Option<Arc<Config>>;
    async fn upload(&self, bucket: &str, key: &str, body: Vec<u8>) -> Result<String, S3Error>;
    async fn show_buckets(&self) -> Result<Vec<String>, S3Error>;
}
```

The `#[automock]` macro from mockall automatically generates a `MockAppStateOperations` type.

### Production: AppState

The production `AppState` implements this trait:

```rust
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub service: Arc<ContentService>
}

impl AppStateOperations for AppState { ... }
```

### Testing: TestAppState

For tests, we use `TestAppState` which wraps the mock in an Arc to make it cloneable:

```rust
#[cfg(test)]
#[derive(Clone)]
pub struct TestAppState(Arc<MockAppStateOperations>);
```

## Writing Tests

### Example Test

```rust
use crate::{
    app::{MockAppStateOperations, TestAppState},
    config::Config,
    router::app_test,
};

#[tokio::test]
async fn test_put_object() {
    // Create the mock
    let mut mock = MockAppStateOperations::new();

    // Set up config expectations
    mock.expect_config()
        .returning(|| {
            Some(Arc::new(Config {
                origins: vec!["http://localhost:3000".to_string()],
                ..Default::default()
            }))
        });

    // Set up method expectations
    mock.expect_upload()
        .withf(|bucket, key, body| {
            bucket == "test-bucket"
            && key == "test-key"
            && body == &vec![1, 2, 3, 4]
        })
        .times(1)
        .returning(|_, _, _| Ok("upload-id-123".to_string()));

    // Create test state and server
    let test_state = TestAppState::new(mock);
    let app = app_test(test_state).await.unwrap();
    let server = TestServer::new(app).unwrap();

    // Make request
    let response = server.put("/test-bucket/test-key").await;

    // Assert
    response.assert_status_ok();
    response.assert_text("upload-id-123");
}
```

## Why This Approach?

1. **Type Safety**: Axum requires concrete types at compile time. We can't use generic types directly in route handlers.

2. **Clean Production Code**: The production code (`AppState`) remains simple and doesn't carry test-specific baggage.

3. **Test Isolation**: Tests use `TestAppState` which only exists in test builds (`#[cfg(test)]`).

4. **No Runtime Overhead**: The trait dispatch has zero cost in production builds.

5. **Readability**: The trait makes the contract explicit - it's clear what operations the state must support.

## Adding New State Methods

To add a new method:

1. Add it to the `AppStateOperations` trait
2. Implement it in `AppState`
3. Implement it in `TestAppState` (delegating to the mock)
4. Create test-specific handlers if needed (like `put_object_test`)

The pattern keeps everything organized and maintainable.
