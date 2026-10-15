use monolith::{
    backend::{router, BackendState},
    backend_client::BackendClient,
    backend_config::BackendConfig,
    style_config::bundled_style_catalog,
};

async fn serve(directory: &std::path::Path) -> (String, BackendClient) {
    let state = BackendState::new(
        BackendConfig {
            database_path: directory.join("backend.db").display().to_string(),
            ..BackendConfig::default()
        },
        directory,
    )
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router(state)).await.unwrap() });
    let url = format!("http://{address}");
    let client = BackendClient::new(&url).unwrap();
    (url, client)
}

#[tokio::test]
async fn styles_are_public_and_the_backend_client_fetches_them() {
    let directory = tempfile::tempdir().unwrap();
    let (url, client) = serve(directory.path()).await;

    let response = reqwest::get(format!("{url}/api/v1/styles")).await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let catalog = client.fetch_styles().await.unwrap();
    assert_eq!(catalog.presets.len(), 7);
    assert_eq!(catalog.presets[1].id, "dark");
}

#[tokio::test]
async fn configured_valid_catalog_is_served_but_an_invalid_one_falls_back_to_bundled() {
    let valid_dir = tempfile::tempdir().unwrap();
    let mut configured = bundled_style_catalog();
    configured.presets[0].name = "Clair du serveur".into();
    std::fs::write(
        valid_dir.path().join("style-presets.json"),
        serde_json::to_vec(&configured).unwrap(),
    )
    .unwrap();
    let (_, valid_client) = serve(valid_dir.path()).await;
    assert_eq!(
        valid_client.fetch_styles().await.unwrap().presets[0].name,
        "Clair du serveur"
    );

    let invalid_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        invalid_dir.path().join("style-presets.json"),
        br#"{"presets":[{"id":"oops"}]}"#,
    )
    .unwrap();
    let (_, invalid_client) = serve(invalid_dir.path()).await;
    assert_eq!(
        invalid_client.fetch_styles().await.unwrap().presets.len(),
        7
    );
}

#[tokio::test]
async fn backend_client_rejects_an_invalid_style_catalog() {
    let mut invalid = bundled_style_catalog();
    invalid.presets[0].corner_radius = 100.0;
    let body = serde_json::to_string(&invalid).unwrap();
    let app = axum::Router::new().route(
        "/api/v1/styles",
        axum::routing::get(move || {
            let body = body.clone();
            async move {
                (
                    [(&axum::http::header::CONTENT_TYPE, "application/json")],
                    body,
                )
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = BackendClient::new(format!("http://{address}")).unwrap();
    assert!(client.fetch_styles().await.is_err());
}

#[tokio::test]
async fn oversized_chunked_style_response_is_stopped_before_the_stream_finishes() {
    use axum::body::{Body, Bytes};
    use futures_util::stream::{self, StreamExt};
    let app = axum::Router::new().route(
        "/api/v1/styles",
        axum::routing::get(|| async {
            let first = stream::iter(vec![Ok::<Bytes, std::io::Error>(Bytes::from(vec![
                b'x';
                256 * 1024
                    + 1
            ]))]);
            let never = stream::pending::<Result<Bytes, std::io::Error>>();
            Body::from_stream(first.chain(never))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = BackendClient::new(format!("http://{address}")).unwrap();
    let result =
        tokio::time::timeout(std::time::Duration::from_millis(750), client.fetch_styles()).await;
    assert!(
        result.is_ok(),
        "le client doit arrêter le flux dès la limite atteinte"
    );
    assert!(result.unwrap().is_err());
}

#[tokio::test]
async fn incomplete_configured_catalog_falls_back_to_all_seven_presets() {
    let directory = tempfile::tempdir().unwrap();
    let mut incomplete = bundled_style_catalog();
    incomplete.presets.pop();
    std::fs::write(
        directory.path().join("style-presets.json"),
        serde_json::to_vec(&incomplete).unwrap(),
    )
    .unwrap();
    let (_, client) = serve(directory.path()).await;

    assert_eq!(client.fetch_styles().await.unwrap().presets.len(), 7);
}
