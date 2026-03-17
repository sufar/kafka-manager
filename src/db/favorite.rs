use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::db::DbPool;

/// 收藏分组
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct FavoriteGroup {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 收藏项
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct FavoriteItem {
    pub id: i64,
    pub group_id: i64,
    pub cluster_id: String,
    pub topic_name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建分组请求
#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

/// 更新分组请求
#[derive(Debug, Deserialize)]
pub struct UpdateGroupRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

/// 创建收藏请求
#[derive(Debug, Deserialize)]
pub struct CreateFavoriteRequest {
    pub group_id: i64,
    pub cluster_id: String,
    pub topic_name: String,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

/// 更新收藏请求
#[derive(Debug, Deserialize)]
pub struct UpdateFavoriteRequest {
    pub group_id: Option<i64>,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

/// 带收藏项的分组
#[derive(Debug, Serialize)]
pub struct GroupWithFavorites {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub items: Vec<FavoriteItem>,
}

/// 初始化收藏相关表
pub async fn init_tables(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // 创建收藏分组表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS favorite_groups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建收藏项表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS favorite_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            group_id INTEGER NOT NULL,
            cluster_id TEXT NOT NULL,
            topic_name TEXT NOT NULL,
            description TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (group_id) REFERENCES favorite_groups(id) ON DELETE CASCADE,
            UNIQUE(cluster_id, topic_name)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建索引
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_favorite_items_group_id ON favorite_items(group_id)"
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_favorite_items_cluster ON favorite_items(cluster_id, topic_name)"
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_favorite_groups_sort ON favorite_groups(sort_order)"
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// 创建分组
pub async fn create_group(
    pool: &DbPool,
    req: &CreateGroupRequest,
) -> Result<FavoriteGroup, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let sort_order = req.sort_order.unwrap_or(0);

    let group = sqlx::query_as::<_, FavoriteGroup>(
        r#"
        INSERT INTO favorite_groups (name, description, sort_order, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(&req.name)
    .bind(&req.description)
    .bind(sort_order)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool.inner())
    .await?;

    Ok(group)
}

/// 收藏分组（带数量）
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct FavoriteGroupWithCount {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
    pub item_count: i64,
}

/// 获取所有分组（带收藏数量）
pub async fn get_all_groups_with_count(pool: &DbPool) -> Result<Vec<FavoriteGroupWithCount>, sqlx::Error> {
    let groups = sqlx::query_as::<_, FavoriteGroupWithCount>(
        r#"
        SELECT
            fg.*,
            COUNT(fi.id) as item_count
        FROM favorite_groups fg
        LEFT JOIN favorite_items fi ON fi.group_id = fg.id
        GROUP BY fg.id
        ORDER BY fg.sort_order ASC, fg.created_at ASC
        "#,
    )
    .fetch_all(pool.inner())
    .await?;

    Ok(groups)
}

/// 获取所有分组
pub async fn get_all_groups(pool: &DbPool) -> Result<Vec<FavoriteGroup>, sqlx::Error> {
    let groups = sqlx::query_as::<_, FavoriteGroup>(
        r#"
        SELECT * FROM favorite_groups
        ORDER BY sort_order ASC, created_at ASC
        "#,
    )
    .fetch_all(pool.inner())
    .await?;

    Ok(groups)
}

/// 获取单个分组
pub async fn get_group_by_id(pool: &DbPool, id: i64) -> Result<Option<FavoriteGroup>, sqlx::Error> {
    let group = sqlx::query_as::<_, FavoriteGroup>(
        r#"
        SELECT * FROM favorite_groups WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool.inner())
    .await?;

    Ok(group)
}

/// 更新分组
pub async fn update_group(
    pool: &DbPool,
    id: i64,
    req: &UpdateGroupRequest,
) -> Result<Option<FavoriteGroup>, sqlx::Error> {
    let now = Utc::now().to_rfc3339();

    // 获取当前分组
    let current = get_group_by_id(pool, id).await?;
    if current.is_none() {
        return Ok(None);
    }

    let name = req.name.as_ref().unwrap_or(&current.as_ref().unwrap().name);
    let description = req.description.as_ref().or(current.as_ref().unwrap().description.as_ref());
    let sort_order = req.sort_order.unwrap_or(current.as_ref().unwrap().sort_order);

    let group = sqlx::query_as::<_, FavoriteGroup>(
        r#"
        UPDATE favorite_groups
        SET name = ?, description = ?, sort_order = ?, updated_at = ?
        WHERE id = ?
        RETURNING *
        "#,
    )
    .bind(name)
    .bind(description)
    .bind(sort_order)
    .bind(&now)
    .bind(id)
    .fetch_optional(pool.inner())
    .await?;

    Ok(group)
}

/// 删除分组
pub async fn delete_group(pool: &DbPool, id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM favorite_groups WHERE id = ?")
        .bind(id)
        .execute(pool.inner())
        .await?;

    Ok(result.rows_affected() > 0)
}

/// 创建收藏
pub async fn create_favorite(
    pool: &DbPool,
    req: &CreateFavoriteRequest,
) -> Result<FavoriteItem, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let sort_order = req.sort_order.unwrap_or(0);

    let item = sqlx::query_as::<_, FavoriteItem>(
        r#"
        INSERT INTO favorite_items (group_id, cluster_id, topic_name, description, sort_order, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(req.group_id)
    .bind(&req.cluster_id)
    .bind(&req.topic_name)
    .bind(&req.description)
    .bind(sort_order)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool.inner())
    .await?;

    Ok(item)
}

/// 获取收藏项
pub async fn get_favorite_by_id(pool: &DbPool, id: i64) -> Result<Option<FavoriteItem>, sqlx::Error> {
    let item = sqlx::query_as::<_, FavoriteItem>(
        r#"
        SELECT * FROM favorite_items WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool.inner())
    .await?;

    Ok(item)
}

/// 获取分组下的所有收藏
pub async fn get_favorites_by_group(
    pool: &DbPool,
    group_id: i64,
) -> Result<Vec<FavoriteItem>, sqlx::Error> {
    let items = sqlx::query_as::<_, FavoriteItem>(
        r#"
        SELECT * FROM favorite_items
        WHERE group_id = ?
        ORDER BY sort_order ASC, created_at ASC
        "#,
    )
    .bind(group_id)
    .fetch_all(pool.inner())
    .await?;

    Ok(items)
}

/// 获取所有收藏（带分组）
pub async fn get_all_favorites_with_groups(
    pool: &DbPool,
) -> Result<Vec<GroupWithFavorites>, sqlx::Error> {
    let groups = get_all_groups(pool).await?;
    let mut result = Vec::new();

    for group in groups {
        let items = get_favorites_by_group(pool, group.id).await?;
        result.push(GroupWithFavorites {
            id: group.id,
            name: group.name,
            description: group.description,
            sort_order: group.sort_order,
            items,
        });
    }

    Ok(result)
}

/// 更新收藏
pub async fn update_favorite(
    pool: &DbPool,
    id: i64,
    req: &UpdateFavoriteRequest,
) -> Result<Option<FavoriteItem>, sqlx::Error> {
    let now = Utc::now().to_rfc3339();

    // 获取当前收藏
    let current = get_favorite_by_id(pool, id).await?;
    if current.is_none() {
        return Ok(None);
    }

    let group_id = req.group_id.unwrap_or(current.as_ref().unwrap().group_id);
    let description = req.description.as_ref().or(current.as_ref().unwrap().description.as_ref());
    let sort_order = req.sort_order.unwrap_or(current.as_ref().unwrap().sort_order);

    let item = sqlx::query_as::<_, FavoriteItem>(
        r#"
        UPDATE favorite_items
        SET group_id = ?, description = ?, sort_order = ?, updated_at = ?
        WHERE id = ?
        RETURNING *
        "#,
    )
    .bind(group_id)
    .bind(description)
    .bind(sort_order)
    .bind(&now)
    .bind(id)
    .fetch_optional(pool.inner())
    .await?;

    Ok(item)
}

/// 删除收藏
pub async fn delete_favorite(pool: &DbPool, id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM favorite_items WHERE id = ?")
        .bind(id)
    .execute(pool.inner())
    .await?;

    Ok(result.rows_affected() > 0)
}

/// 根据 cluster_id 和 topic_name 删除收藏
pub async fn delete_favorite_by_topic(
    pool: &DbPool,
    cluster_id: &str,
    topic_name: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM favorite_items WHERE cluster_id = ? AND topic_name = ?"
    )
    .bind(cluster_id)
    .bind(topic_name)
    .execute(pool.inner())
    .await?;

    Ok(result.rows_affected() > 0)
}

/// 检查 topic 是否已收藏
pub async fn is_topic_favorite(
    pool: &DbPool,
    cluster_id: &str,
    topic_name: &str,
) -> Result<bool, sqlx::Error> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM favorite_items WHERE cluster_id = ? AND topic_name = ?"
    )
    .bind(cluster_id)
    .bind(topic_name)
    .fetch_one(pool.inner())
    .await?;

    Ok(count > 0)
}
