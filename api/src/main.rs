use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Estructura para entregar datos completos al frontend
#[derive(Serialize)]
struct UserInfo {
    name: Option<String>,
    bio: Option<String>,
    avatar_url: Option<String>,
    browser_user_agent: String,
    github_date: String,
}

#[derive(Serialize)]
struct RepoInfo {
    name: String,
    full_name: String,
    html_url: String,
}

#[derive(Serialize)]
struct RepoResponse {
    repos: Vec<RepoInfo>,
}

#[derive(Deserialize, Debug)]
struct GraphQLError {
    message: String,
}

#[derive(Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Deserialize)]
struct UserData {
    user: Option<GitHubUserGQL>,
}

#[derive(Deserialize)]
struct GitHubUserGQL {
    name: Option<String>,
    bio: Option<String>,
    #[serde(rename = "avatarUrl")]
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct ReposData {
    user: Option<UserReposGQL>,
}

#[derive(Deserialize)]
struct UserReposGQL {
    repositories: ReposNodesGQL,
}

#[derive(Deserialize)]
struct ReposNodesGQL {
    nodes: Vec<GitHubRepoGQL>,
}

#[derive(Deserialize)]
struct GitHubRepoGQL {
    name: String,
    #[serde(rename = "nameWithOwner")]
    name_with_owner: String,
    url: String,
}

struct AppState {
    client: Client,
    token: String,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry().with(tracing_subscriber::fmt::layer()).init();

    let token = env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN must be set en variables de entorno o archivo .env");
    let client = Client::builder()
        .user_agent("rust-github-gql-agent/1.0")
        .build()
        .expect("Failed to create reqwest client");

    let shared_state = Arc::new(AppState { client, token });

    let app = Router::new()
        .route("/user/{username}", get(get_user_info))
        .route("/repos/{username}", get(get_user_repos))
        .with_state(shared_state);

    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(&addr).await
        .unwrap_or_else(|e| panic!("Error enlazando el servidor en {}: {}", addr, e));

    tracing::info!("Server GraphQL running on http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}

async fn get_user_info(
    Path(username): Path<String>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let browser_ua = headers.get(header::USER_AGENT).and_then(|v| v.to_str().ok()).unwrap_or("unknown").to_string();

    let query = r#"
        query($login: String!) {
            user(login: $login) {
                name
                avatarUrl
                bio
            }
        }
    "#;

    let response = state.client.post("https://api.github.com/graphql")
        .header(header::AUTHORIZATION, format!("Bearer {}", state.token))
        .json(&serde_json::json!({ "query": query, "variables": { "login": username } }))
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Error conectando con GitHub: {}", e)))?;

    let github_date = response.headers().get(header::DATE).and_then(|v| v.to_str().ok()).unwrap_or("unknown").to_string();
    let gql_res: GraphQLResponse<UserData> = response.json().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Error procesando respuesta JSON de GitHub: {}", e)))?;

    let user_data = match gql_res.data.and_then(|d| d.user) {
        Some(user) => user,
        None => {
            if let Some(errors) = gql_res.errors {
                let msg = errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>().join("; ");
                return Err((StatusCode::BAD_REQUEST, format!("Error devuelto por GitHub: {}", msg)));
            }
            return Err((StatusCode::NOT_FOUND, format!("Usuario '{}' no encontrado en GitHub", username)));
        }
    };

    let name = user_data.name;
    let bio = user_data.bio;
    let avatar_url = user_data.avatar_url;

    Ok(Json(UserInfo { name, bio, avatar_url, browser_user_agent: browser_ua, github_date }))
}

async fn get_user_repos(
    Path(username): Path<String>,
    State(state): State<Arc<AppState>>,
    _headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let query = r#"
        query($login: String!) {
            user(login: $login) {
                repositories(first: 50, privacy: PUBLIC, orderBy: {field: NAME, direction: ASC}) {
                    nodes {
                        name
                        nameWithOwner
                        url
                    }
                }
            }
        }
    "#;

    let response = state.client.post("https://api.github.com/graphql")
        .header(header::AUTHORIZATION, format!("Bearer {}", state.token))
        .json(&serde_json::json!({ "query": query, "variables": { "login": username } }))
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Error conectando con GitHub: {}", e)))?;

    let gql_res: GraphQLResponse<ReposData> = response.json().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Error procesando respuesta JSON de GitHub: {}", e)))?;

    let user_data = match gql_res.data.and_then(|d| d.user) {
        Some(user) => user,
        None => {
            if let Some(errors) = gql_res.errors {
                let msg = errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>().join("; ");
                return Err((StatusCode::BAD_REQUEST, format!("Error devuelto por GitHub: {}", msg)));
            }
            return Err((StatusCode::NOT_FOUND, format!("Usuario '{}' no encontrado en GitHub", username)));
        }
    };

    let repo_infos = user_data.repositories.nodes.into_iter().map(|r| RepoInfo {
        name: r.name,
        full_name: r.name_with_owner,
        html_url: r.url,
    }).collect();

    Ok(Json(RepoResponse { repos: repo_infos }))
}
