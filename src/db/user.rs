/// 用户和角色管理模块

use crate::error::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};

/// 用户
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub email: Option<String>,
    pub role_id: i64,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// 角色
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Role {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub permissions: String, // JSON 格式的权限列表
    pub created_at: String,
}

/// 用户创建请求
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    pub role_id: i64,
}

/// 角色创建请求
#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
}

/// 用户响应
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
    pub email: Option<String>,
    pub role_id: i64,
    pub role_name: Option<String>,
    pub is_active: bool,
    pub created_at: String,
}

/// 角色响应
#[derive(Debug, Serialize)]
pub struct RoleResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub created_at: String,
}

/// 存储操作
pub struct UserStore;

impl UserStore {
    /// 创建用户
    pub async fn create(
        pool: &sqlx::SqlitePool,
        username: &str,
        password_hash: &str,
        email: Option<&str>,
        role_id: i64,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            r#"
            INSERT INTO users (username, password_hash, email, role_id, is_active, created_at, updated_at)
            VALUES (?, ?, ?, ?, 1, ?, ?)
            "#,
        )
        .bind(username)
        .bind(password_hash)
        .bind(email)
        .bind(role_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// 获取用户 by username
    pub async fn get_by_username(
        pool: &sqlx::SqlitePool,
        username: &str,
    ) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE username = ?"
        )
        .bind(username)
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }

    /// 获取用户 by ID
    pub async fn get_by_id(
        pool: &sqlx::SqlitePool,
        id: i64,
    ) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }

    /// 获取所有用户
    pub async fn list(pool: &sqlx::SqlitePool) -> Result<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            "SELECT * FROM users ORDER BY id DESC"
        )
        .fetch_all(pool)
        .await?;

        Ok(users)
    }

    /// 更新用户
    pub async fn update(
        pool: &sqlx::SqlitePool,
        id: i64,
        email: Option<&str>,
        role_id: Option<i64>,
        is_active: Option<bool>,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE users SET updated_at = ?, email = COALESCE(?, email), role_id = COALESCE(?, role_id), is_active = COALESCE(?, is_active) WHERE id = ?"
        )
        .bind(&now)
        .bind(email)
        .bind(role_id)
        .bind(is_active)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 更新用户密码
    pub async fn update_password(
        pool: &sqlx::SqlitePool,
        id: i64,
        password_hash: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?"
        )
        .bind(password_hash)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 删除用户
    pub async fn delete(pool: &sqlx::SqlitePool, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// 获取用户详情（带角色名）
    pub async fn get_with_role(pool: &sqlx::SqlitePool, id: i64) -> Result<Option<UserResponse>> {
        let row = sqlx::query(
            r#"
            SELECT u.id, u.username, u.email, u.role_id, u.is_active, u.created_at, r.name as role_name
            FROM users u
            LEFT JOIN roles r ON u.role_id = r.id
            WHERE u.id = ?
            "#
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => {
                Ok(Some(UserResponse {
                    id: row.get("id"),
                    username: row.get("username"),
                    email: row.get("email"),
                    role_id: row.get("role_id"),
                    role_name: row.get("role_name"),
                    is_active: row.get("is_active"),
                    created_at: row.get("created_at"),
                }))
            }
            None => Ok(None),
        }
    }

    /// 获取所有用户详情
    pub async fn list_with_roles(pool: &sqlx::SqlitePool) -> Result<Vec<UserResponse>> {
        let rows = sqlx::query(
            r#"
            SELECT u.id, u.username, u.email, u.role_id, u.is_active, u.created_at, r.name as role_name
            FROM users u
            LEFT JOIN roles r ON u.role_id = r.id
            ORDER BY u.id DESC
            "#
        )
        .fetch_all(pool)
        .await?;

        let mut users = Vec::new();
        for row in rows {
            users.push(UserResponse {
                id: row.get("id"),
                username: row.get("username"),
                email: row.get("email"),
                role_id: row.get("role_id"),
                role_name: row.get("role_name"),
                is_active: row.get("is_active"),
                created_at: row.get("created_at"),
            });
        }

        Ok(users)
    }
}

/// 角色存储操作
pub struct RoleStore;

impl RoleStore {
    /// 创建角色
    pub async fn create(
        pool: &sqlx::SqlitePool,
        name: &str,
        description: Option<&str>,
        permissions: &[String],
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let permissions_json = serde_json::to_string(permissions)?;

        let result = sqlx::query(
            r#"
            INSERT INTO roles (name, description, permissions, created_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(name)
        .bind(description)
        .bind(&permissions_json)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// 获取角色 by ID
    pub async fn get_by_id(pool: &sqlx::SqlitePool, id: i64) -> Result<Option<Role>> {
        let role = sqlx::query_as::<_, Role>(
            "SELECT * FROM roles WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(role)
    }

    /// 获取角色 by name
    pub async fn get_by_name(pool: &sqlx::SqlitePool, name: &str) -> Result<Option<Role>> {
        let role = sqlx::query_as::<_, Role>(
            "SELECT * FROM roles WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;

        Ok(role)
    }

    /// 获取所有角色
    pub async fn list(pool: &sqlx::SqlitePool) -> Result<Vec<Role>> {
        let roles = sqlx::query_as::<_, Role>(
            "SELECT * FROM roles ORDER BY id DESC"
        )
        .fetch_all(pool)
        .await?;

        Ok(roles)
    }

    /// 更新角色
    pub async fn update(
        pool: &sqlx::SqlitePool,
        id: i64,
        name: Option<&str>,
        description: Option<&str>,
        permissions: Option<&[String]>,
    ) -> Result<()> {
        let permissions_json = permissions.map(|p| serde_json::to_string(p))
            .transpose()
            .map_err(|e| crate::error::AppError::Internal(format!("Failed to serialize permissions: {}", e)))?;

        sqlx::query(
            "UPDATE roles SET name = COALESCE(?, name), description = COALESCE(?, description), permissions = COALESCE(?, permissions) WHERE id = ?"
        )
        .bind(name)
        .bind(description)
        .bind(permissions_json)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 删除角色
    pub async fn delete(pool: &sqlx::SqlitePool, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM roles WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// 检查权限
    pub async fn check_permission(pool: &sqlx::SqlitePool, role_id: i64, permission: &str) -> Result<bool> {
        let role = Self::get_by_id(pool, role_id).await?;

        match role {
            Some(r) => {
                let permissions: Vec<String> = serde_json::from_str(&r.permissions)?;
                Ok(permissions.contains(&permission.to_string()) || permissions.contains(&"*".to_string()))
            }
            None => Ok(false),
        }
    }
}
