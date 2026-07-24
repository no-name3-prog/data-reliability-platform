//! PostgreSQL connector — discover databases, schemas, tables, columns; sample rows.

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Column, PgPool, Row, TypeInfo};
use tracing::{debug, info};

use drp_common::{AssetKind, Error, Result, SourceLocation};
use drp_core::{
    map_sql_type, Asset, CatalogDatabase, CatalogSchema, CatalogTable, CatalogTree, ColumnMeta,
    ConnectorPlugin, Plugin, PluginCapability, PluginContext, PluginInfo,
};

/// PostgreSQL connector (`id = "postgres"`).
///
/// Connection is taken from `location.uri` as a Postgres URL, e.g.
/// `postgres://user:pass@host:5432/dbname`.
///
/// Optional properties:
/// - `schemas` — comma-separated schema filter (default: non-system schemas)
/// - `sample_schema` / `sample_table` — preferred for sampling when asset props missing
pub struct PostgresConnector {
    info: PluginInfo,
}

impl PostgresConnector {
    /// Create the connector.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new(
                "postgres",
                "PostgreSQL Connector",
                env!("CARGO_PKG_VERSION"),
            )
            .with_description("Discover databases/schemas/tables/columns and sample rows via SQL")
            .with_capability(PluginCapability::Connector),
        }
    }

    async fn pool(location: &SourceLocation) -> Result<PgPool> {
        let url = if location.uri.is_empty() {
            location
                .properties
                .get("database_url")
                .cloned()
                .ok_or_else(|| Error::connector("postgres: location.uri (database URL) required"))?
        } else {
            location.uri.clone()
        };
        PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect(&url)
            .await
            .map_err(|e| Error::connector(format!("postgres connect: {e}")))
    }
}

impl Default for PostgresConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for PostgresConnector {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl ConnectorPlugin for PostgresConnector {
    async fn test_connection(&self, location: &SourceLocation, _ctx: &PluginContext) -> Result<()> {
        let pool = Self::pool(location).await?;
        sqlx::query("SELECT 1")
            .fetch_one(&pool)
            .await
            .map_err(|e| Error::connector(format!("postgres ping: {e}")))?;
        Ok(())
    }

    async fn discover(&self, location: &SourceLocation, ctx: &PluginContext) -> Result<Vec<Asset>> {
        let tree = self.discover_catalog(location, ctx).await?;
        let mut assets = Vec::new();
        for t in tree.all_tables() {
            let mut a = Asset::new(t.fqn.clone(), t.name.clone(), t.kind, location.clone())
                .with_columns(t.columns.clone());
            for (k, v) in &t.properties {
                a = a.with_tag(k, v);
            }
            assets.push(a);
        }
        Ok(assets)
    }

    async fn discover_catalog(
        &self,
        location: &SourceLocation,
        _ctx: &PluginContext,
    ) -> Result<CatalogTree> {
        let pool = Self::pool(location).await?;
        let mut tree = CatalogTree::new(self.info.id.clone(), location.clone());

        // Current database name
        let db_name: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&pool)
            .await
            .map_err(|e| Error::connector(format!("current_database: {e}")))?;

        let schema_filter = location
            .properties
            .get("schemas")
            .map(|s| {
                s.split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let schemas: Vec<String> = if schema_filter.is_empty() {
            sqlx::query_scalar(
                r#"
                SELECT schema_name
                FROM information_schema.schemata
                WHERE schema_name NOT IN ('pg_catalog', 'information_schema', 'pg_toast')
                ORDER BY schema_name
                "#,
            )
            .fetch_all(&pool)
            .await
            .map_err(|e| Error::connector(format!("list schemas: {e}")))?
        } else {
            schema_filter
        };

        let mut db = CatalogDatabase::new(&db_name);

        for schema_name in schemas {
            let mut schema = CatalogSchema::new(&schema_name);

            let tables = sqlx::query(
                r#"
                SELECT table_name, table_type
                FROM information_schema.tables
                WHERE table_schema = $1
                ORDER BY table_name
                "#,
            )
            .bind(&schema_name)
            .fetch_all(&pool)
            .await
            .map_err(|e| Error::connector(format!("list tables: {e}")))?;

            for row in tables {
                let table_name: String = row
                    .try_get("table_name")
                    .map_err(|e| Error::connector(format!("table_name: {e}")))?;
                let table_type: String = row
                    .try_get("table_type")
                    .unwrap_or_else(|_| "BASE TABLE".into());
                let kind = if table_type.contains("VIEW") {
                    AssetKind::View
                } else {
                    AssetKind::Table
                };

                let cols = sqlx::query(
                    r#"
                    SELECT column_name, data_type, is_nullable, ordinal_position
                    FROM information_schema.columns
                    WHERE table_schema = $1 AND table_name = $2
                    ORDER BY ordinal_position
                    "#,
                )
                .bind(&schema_name)
                .bind(&table_name)
                .fetch_all(&pool)
                .await
                .map_err(|e| Error::connector(format!("list columns: {e}")))?;

                let mut columns = Vec::new();
                for c in cols {
                    let name: String = c.try_get("column_name").unwrap_or_default();
                    let dtype: String = c.try_get("data_type").unwrap_or_else(|_| "unknown".into());
                    let nullable: String =
                        c.try_get("is_nullable").unwrap_or_else(|_| "YES".into());
                    let ordinal: i32 = c.try_get("ordinal_position").unwrap_or(0);
                    let mut col = ColumnMeta::new(name, map_sql_type(&dtype)).at(ordinal as u32);
                    col.nullable = nullable.eq_ignore_ascii_case("YES");
                    columns.push(col);
                }

                let mut table = CatalogTable::new(&db_name, &schema_name, table_name, kind)
                    .with_columns(columns);
                table = table
                    .with_property("database", &db_name)
                    .with_property("schema", &schema_name)
                    .with_property("table_type", &table_type);
                schema.tables.push(table);
            }

            info!(
                database = %db_name,
                schema = %schema_name,
                tables = schema.tables.len(),
                "postgres catalog schema discovered"
            );
            db.schemas.push(schema);
        }

        tree.databases.push(db);
        debug!(tables = tree.table_count(), "postgres catalog complete");
        Ok(tree)
    }

    async fn sample_rows(
        &self,
        asset: &Asset,
        limit: usize,
        _ctx: &PluginContext,
    ) -> Result<Vec<IndexMap<String, Value>>> {
        let pool = Self::pool(&asset.location).await?;
        let schema = asset
            .tags
            .get("schema")
            .cloned()
            .or_else(|| asset.location.properties.get("schema").cloned())
            .unwrap_or_else(|| "public".into());
        let table = asset
            .tags
            .get("table")
            .cloned()
            .unwrap_or_else(|| asset.name.clone());

        // Quote identifiers safely (simple validation)
        validate_ident(&schema)?;
        validate_ident(&table)?;

        let sql = format!(
            r#"SELECT * FROM "{}"."{}" LIMIT {}"#,
            schema.replace('"', ""),
            table.replace('"', ""),
            limit.max(1)
        );
        let rows = sqlx::query(&sql)
            .fetch_all(&pool)
            .await
            .map_err(|e| Error::connector(format!("sample rows: {e}")))?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let mut map = IndexMap::new();
            for col in row.columns() {
                let name = col.name().to_string();
                let val = pg_value_to_json(&row, col.ordinal(), col.type_info().name());
                map.insert(name, val);
            }
            out.push(map);
        }
        Ok(out)
    }
}

fn validate_ident(s: &str) -> Result<()> {
    if s.is_empty()
        || !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
    {
        return Err(Error::validation(format!("invalid SQL identifier: {s}")));
    }
    Ok(())
}

fn pg_value_to_json(row: &sqlx::postgres::PgRow, idx: usize, type_name: &str) -> Value {
    // Best-effort decoding for common types.
    if let Ok(v) = row.try_get::<Option<bool>, _>(idx) {
        return match v {
            Some(b) => json!(b),
            None => Value::Null,
        };
    }
    if let Ok(v) = row.try_get::<Option<i64>, _>(idx) {
        return match v {
            Some(n) => json!(n),
            None => Value::Null,
        };
    }
    if let Ok(v) = row.try_get::<Option<i32>, _>(idx) {
        return match v {
            Some(n) => json!(n),
            None => Value::Null,
        };
    }
    if let Ok(v) = row.try_get::<Option<f64>, _>(idx) {
        return match v {
            Some(n) => json!(n),
            None => Value::Null,
        };
    }
    if let Ok(v) = row.try_get::<Option<String>, _>(idx) {
        return match v {
            Some(s) => json!(s),
            None => Value::Null,
        };
    }
    // Fallback: type name marker
    json!({ "_unparsed_type": type_name })
}
