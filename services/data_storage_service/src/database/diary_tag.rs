use super::{
    schema::{diary_entry, diary_entry_tag, diary_tag},
    Error, Result,
};
use crate::{database::models::DiaryEntry, Pool};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::database::schema::diary_tag)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DiaryTag {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::database::schema::diary_entry_tag)]
pub struct DiaryEntryTag {
    pub diary_entry_id: Uuid,
    pub diary_tag_id: Uuid,
}

impl DiaryTag {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::from(name),
        }
    }

    pub async fn find_or_create(name: &str, pool: &Pool) -> Result<Self> {
        let mut conn = pool.get().await.map_err(|e| {
            sentry::capture_error(&e);
            Error::DatabaseConnection
        })?;

        diesel::insert_into(diary_tag::table)
            .values((diary_tag::id.eq(Uuid::new_v4()), diary_tag::name.eq(name)))
            .on_conflict(diary_tag::name)
            .do_nothing()
            .execute(&mut conn)
            .await
            .map_err(|e| {
                sentry::capture_error(&e);
                Error::CreationError
            })?;

        diary_tag::table
            .filter(diary_tag::name.eq(name))
            .first(&mut conn)
            .await
            .map_err(|e| {
                sentry::capture_error(&e);
                Error::FindError
            })
    }

    pub async fn get_tags_for_entry(entry_id: Uuid, pool: &Pool) -> Result<Vec<Self>> {
        let mut conn = pool.get().await.map_err(|e| {
            sentry::capture_error(&e);
            Error::DatabaseConnection
        })?;

        diary_entry_tag::table
            .inner_join(diary_tag::table)
            .filter(diary_entry_tag::diary_entry_id.eq(entry_id))
            .select(diary_tag::all_columns)
            .load::<DiaryTag>(&mut conn)
            .await
            .map_err(|e| {
                sentry::capture_error(&e);
                Error::FindError
            })
    }

    pub async fn find_entries_by_partial_name(partial: &str, pool: &Pool) -> Result<Vec<DiaryEntry>> {
        let mut conn = pool.get().await.map_err(|e| {
            sentry::capture_error(&e);
            Error::DatabaseConnection
        })?;

        diary_entry::table
            .inner_join(diary_entry_tag::table.inner_join(diary_tag::table))
            .filter(diary_tag::name.ilike(format!("%{}%", partial)))
            .select(diary_entry::all_columns)
            .distinct()
            .load::<DiaryEntry>(&mut conn)
            .await
            .map_err(|e| {
                sentry::capture_error(&e);
                Error::FindError
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_diary_tag() {
        let tag = DiaryTag::new("rust");
        assert_eq!(tag.name, "rust");
        assert!(!tag.id.is_nil());
    }

    #[test]
    fn test_diary_tag_id_uniqueness() {
        let tag1 = DiaryTag::new("rust");
        let tag2 = DiaryTag::new("rust");
        assert_ne!(tag1.id, tag2.id);
    }
}
