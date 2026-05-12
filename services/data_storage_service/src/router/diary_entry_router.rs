use super::error::Result;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use greenhouse_core::data_storage_service_dto::diary_dtos::{
    get_diary::GetDiaryResponseDto, get_diary_entry::DiaryEntryResponseDto,
    post_diary_entry::PostDiaryEntryDtoRequest, post_diary_tag::PostDiaryTagDtoRequest,
    put_diary_entry::PutDiaryEntryDtoRequest,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    database::{diary_tag::DiaryTag, models::DiaryEntry},
    router::error::Error,
    AppState,
};

#[derive(Deserialize)]
pub struct Params {
    start: String,
    end: String,
}

pub(crate) fn routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(create_diary_entry))
        .route("/tags/:tag_name", get(search_entries_by_tag))
        .route("/:id", put(update_diary_entry))
        .route("/:id", get(get_diary_entry))
        .route("/:start/:end", get(get_diary))
        .route("/:id/tags", post(add_tag_to_entry))
        .route("/:id/tags/:tag_name", delete(remove_tag_from_entry))
        .with_state(state)
}

#[axum::debug_handler]
pub(crate) async fn update_diary_entry(
    State(AppState { config: _, pool }): State<AppState>,
    Path(id): Path<Uuid>,
    Json(update): Json<PutDiaryEntryDtoRequest>,
) -> Result<impl IntoResponse> {
    let mut entry = DiaryEntry::find_by_id(id, &pool).await?;
    entry.title = update.title.clone();
    entry.entry_date = update.date.parse::<DateTime<Utc>>().map_err(|e| {
        sentry::configure_scope(|scope| {
            let mut map = std::collections::BTreeMap::new();
            map.insert(String::from("time"), update.date.clone().into());

            scope.set_context("time_string", sentry::protocol::Context::Other(map));
        });

        sentry::capture_error(&e);
        Error::TimeError
    })?;

    entry.content = update.content.clone();
    entry.flush(&pool).await?;
    let response = entry.into_response_with_tags(&pool).await?;
    Ok(Json(response))
}

#[axum::debug_handler]
pub(crate) async fn create_diary_entry(
    State(AppState { config: _, pool }): State<AppState>,
    Json(entry): Json<PostDiaryEntryDtoRequest>,
) -> Result<impl IntoResponse> {
    let mut entry = DiaryEntry::new(
        entry.date.parse::<DateTime<Utc>>().map_err(|e| {
            sentry::configure_scope(|scope| {
                let mut map = std::collections::BTreeMap::new();
                map.insert(String::from("time"), entry.date.clone().into());

                scope.set_context("time_string", sentry::protocol::Context::Other(map));
            });

            sentry::capture_error(&e);
            Error::TimeError
        })?,
        &entry.title,
        &entry.content,
    );
    entry.flush(&pool).await?;
    let response: DiaryEntryResponseDto = entry.into();
    Ok(Json(response))
}

#[axum::debug_handler]
pub(crate) async fn get_diary_entry(
    State(AppState { config: _, pool }): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let entry = DiaryEntry::find_by_id(id, &pool).await?;
    let response = entry.into_response_with_tags(&pool).await?;
    Ok(Json(response))
}

#[axum::debug_handler]
pub(crate) async fn get_diary(
    State(AppState { config: _, pool }): State<AppState>,
    Path(Params { start, end }): Path<Params>,
) -> Result<impl IntoResponse> {
    let start = start.parse::<DateTime<Utc>>().map_err(|e| {
        sentry::configure_scope(|scope| {
            let mut map = std::collections::BTreeMap::new();
            map.insert(String::from("time"), start.clone().into());

            scope.set_context("time_string", sentry::protocol::Context::Other(map));
        });

        sentry::capture_error(&e);
        Error::TimeError
    })?;
    let end = end.parse::<DateTime<Utc>>().map_err(|e| {
        sentry::configure_scope(|scope| {
            let mut map = std::collections::BTreeMap::new();
            map.insert(String::from("time"), end.clone().into());

            scope.set_context("time_string", sentry::protocol::Context::Other(map));
        });

        sentry::capture_error(&e);
        Error::TimeError
    })?;
    let entries = DiaryEntry::find_by_date_range(start, end, &pool).await?;
    let mut entry_dtos = Vec::with_capacity(entries.len());
    for entry in entries {
        entry_dtos.push(entry.into_response_with_tags(&pool).await?);
    }
    let response = GetDiaryResponseDto {
        entries: entry_dtos,
    };
    Ok(Json(response))
}

#[axum::debug_handler]
pub(crate) async fn add_tag_to_entry(
    State(AppState { config: _, pool }): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<PostDiaryTagDtoRequest>,
) -> Result<impl IntoResponse> {
    let entry = DiaryEntry::find_by_id(id, &pool).await?;
    entry.add_tag(&body.tag_name, &pool).await?;
    let response = entry.into_response_with_tags(&pool).await?;
    Ok(Json(response))
}

#[axum::debug_handler]
pub(crate) async fn remove_tag_from_entry(
    State(AppState { config: _, pool }): State<AppState>,
    Path((id, tag_name)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse> {
    let entry = DiaryEntry::find_by_id(id, &pool).await?;
    entry.remove_tag(&tag_name, &pool).await?;
    let response = entry.into_response_with_tags(&pool).await?;
    Ok(Json(response))
}

#[axum::debug_handler]
pub(crate) async fn search_entries_by_tag(
    State(AppState { config: _, pool }): State<AppState>,
    Path(tag_name): Path<String>,
) -> Result<impl IntoResponse> {
    let entries = DiaryTag::find_entries_by_partial_name(&tag_name, &pool).await?;
    let mut entry_dtos = Vec::with_capacity(entries.len());
    for entry in entries {
        entry_dtos.push(entry.into_response_with_tags(&pool).await?);
    }
    let response = GetDiaryResponseDto {
        entries: entry_dtos,
    };
    Ok(Json(response))
}
