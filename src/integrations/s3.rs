use crate::config::tests::bootstrap_integration_tests;
use crate::s3::{FileObject, Garage, S3};

fn setup_s3() -> Garage {
    let config = bootstrap_integration_tests();
    Garage::new(
        config.s3_endpoint.parse().expect("Invalid S3 endpoint"),
        &config.key_id,
        &config.secret_key,
    )
}

#[tokio::test]
async fn test_show_buckets() {
    let s3 = setup_s3();
    insta::assert_debug_snapshot!(s3.show_buckets().await);
}

#[tokio::test]
async fn test_put_object() {
    let s3 = setup_s3();
    let file = FileObject {
        data: vec![1, 2, 3],
        content_type: "application/octet-stream".to_string(),
    };
    let res = s3.put_object("test", "test.txt", file).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_get_object() {
    let s3 = setup_s3();
    let file = FileObject {
        data: vec![1, 2, 3],
        content_type: "application/octet-stream".to_string(),
    };
    let _ = s3.put_object("test", "test2.txt", file).await;
    let res = s3.get_object("test", "test2.txt").await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_mime_types_on_object() {
    let s3 = setup_s3();
    let file = FileObject {
        data: "test".as_bytes().to_vec(),
        content_type: "text/plain".to_string(),
    };
    let _ = s3
        .put_object("test", "test4.txt", file)
        .await
        .expect("should upload the file");
    let (_, mime_type) = s3
        .get_object("test", "test4.txt")
        .await
        .expect("should be able to retrieve file");
    assert_eq!(mime_type, "text/plain".to_string());
}
