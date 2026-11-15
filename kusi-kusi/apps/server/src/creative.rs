use crate::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use kusi_application::creative::{
    CreativeError, CreativeRecipe, CreativeRepository, prepare_proposal,
};
use kusi_domain::creative::CreativeJob;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

pub type Recipes =
    std::collections::HashMap<Uuid, std::collections::HashMap<String, CreativeRecipe>>;

pub fn load(simulation: bool) -> Result<Recipes, Box<dyn std::error::Error>> {
    let recipes: Recipes = match std::env::var("KUSI_CREATIVE_RECIPES") {
        Ok(value) => serde_json::from_str(&value)?,
        Err(std::env::VarError::NotPresent) => Recipes::new(),
        Err(e) => return Err(e.into()),
    };
    for tenant in recipes.values() {
        for (name, recipe) in tenant {
            if name.is_empty()
                || name.len() > 80
                || !name
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
            {
                return Err("invalid creative recipe identifier".into());
            }
            kusi_infrastructure::creative::validate_recipe(recipe, simulation)?;
        }
    }
    Ok(recipes)
}
async fn identity(state: &AppState, headers: &HeaderMap) -> Result<(Uuid, String), StatusCode> {
    crate::authorize_admin(state, headers).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let key = crate::bearer_token(headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let org = kusi_application::GetOrganizationByKey::new(state.repository.clone())
        .execute(key)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    Ok((org.id, kusi_application::hash_api_key(key)))
}
fn status(error: CreativeError) -> StatusCode {
    match error {
        CreativeError::Invalid => StatusCode::BAD_REQUEST,
        CreativeError::NotFound => StatusCode::NOT_FOUND,
        CreativeError::Conflict => StatusCode::CONFLICT,
        CreativeError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    }
}

pub async fn recipes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    let (org, _) = identity(&state, &headers).await?;
    let mut items:Vec<Value>=state.creative_recipes.get(&org).into_iter().flat_map(|v|v.iter()).filter(|(_,recipe)|recipe.enabled).map(|(name,recipe)|json!({"recipe":name,"provider":recipe.provider,"model":recipe.model,"input_schema":recipe.input_schema,"defaults":recipe.defaults,"estimated_cost_microusd":recipe.estimated_cost_microusd})).collect();
    if state.local.store.is_some() {
        for kind in ["video", "image"] {
            let r = kusi_application::local::simulated_recipe(kind);
            items.push(json!({"recipe":format!("local_product_{kind}"),"provider":r.provider,"model":r.model,"input_schema":r.input_schema,"estimated_cost_microusd":0}));
        }
    }
    Ok(Json(
        json!({"recipes":items,"worker_enabled":state.creative_worker_enabled}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Create {
    pub recipe: String,
    pub input: Value,
    pub idempotency_key: Uuid,
}
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<Create>,
) -> Result<(StatusCode, Json<CreativeJob>), StatusCode> {
    let (org, principal) = identity(&state, &headers).await?;
    let builtin = if state.local.store.is_some() {
        match input.recipe.as_str() {
            "local_product_video" => Some(kusi_application::local::simulated_recipe("video")),
            "local_product_image" => Some(kusi_application::local::simulated_recipe("image")),
            _ => None,
        }
    } else {
        None
    };
    if let Some(storage) = state.local.store.as_ref() {
        let id = input
            .input
            .get("asset_id")
            .and_then(Value::as_str)
            .and_then(|v| v.parse().ok())
            .ok_or(StatusCode::BAD_REQUEST)?;
        storage
            .asset(org, id)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;
    }
    let recipe = builtin
        .as_ref()
        .or_else(|| {
            state
                .creative_recipes
                .get(&org)
                .and_then(|r| r.get(&input.recipe))
        })
        .ok_or(StatusCode::NOT_FOUND)?;
    let proposal = prepare_proposal(
        &input.recipe,
        recipe,
        input.input,
        &kusi_infrastructure::JsonSchemaValidator,
    )
    .map_err(status)?;
    let job = state
        .repository
        .create(org, &principal, input.idempotency_key, proposal)
        .await
        .map_err(status)?;
    Ok((StatusCode::CREATED, Json(job)))
}
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<CreativeJob>, StatusCode> {
    let (org, _) = identity(&state, &headers).await?;
    Ok(Json(state.repository.get(org, id).await.map_err(status)?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    pub confirm: bool,
    pub approval_digest: String,
    pub max_cost_microusd: i64,
}
pub async fn approve(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<Approval>,
) -> Result<Json<CreativeJob>, StatusCode> {
    let (org, principal) = identity(&state, &headers).await?;
    if !input.confirm || input.approval_digest.len() != 64 {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(Json(
        state
            .repository
            .approve(
                org,
                id,
                &principal,
                &input.approval_digest,
                input.max_cost_microusd,
            )
            .await
            .map_err(status)?,
    ))
}
pub async fn cancel(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<CreativeJob>, StatusCode> {
    let (org, principal) = identity(&state, &headers).await?;
    Ok(Json(
        state
            .repository
            .cancel(org, id, &principal)
            .await
            .map_err(status)?,
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    pub daily_limit_microusd: i64,
}
pub async fn set_budget(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<Budget>,
) -> Result<StatusCode, StatusCode> {
    let (org, principal) = identity(&state, &headers).await?;
    state
        .repository
        .set_daily_budget(org, &principal, input.daily_limit_microusd)
        .await
        .map_err(status)?;
    Ok(StatusCode::NO_CONTENT)
}
pub async fn budget(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    let (org, _) = identity(&state, &headers).await?;
    let (limit, reserved) = state.repository.daily_budget(org).await.map_err(status)?;
    Ok(Json(
        json!({"daily_limit_microusd":limit,"reserved_estimate_microusd":reserved,"timezone":"UTC"}),
    ))
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<CreativeJob>>, StatusCode> {
    let (org, _) = identity(&state, &headers).await?;
    Ok(Json(state.repository.list(org).await.map_err(status)?))
}
