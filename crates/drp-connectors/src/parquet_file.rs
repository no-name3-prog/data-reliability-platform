//! Parquet file connector — schema discovery and row sampling via Arrow.

use std::fs::File;
use std::path::{Path, PathBuf};

use arrow_array::cast::AsArray;
use arrow_array::types::{Float64Type, Int32Type, Int64Type};
use arrow_array::{Array, RecordBatch};
use arrow_schema::{DataType as ArrowType, Schema};
use async_trait::async_trait;
use indexmap::IndexMap;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde_json::{json, Value};
use tracing::info;

use drp_common::{AssetKind, DataType, Error, Result, SourceLocation};
use drp_core::{
    Asset, CatalogDatabase, CatalogSchema, CatalogTable, CatalogTree, ColumnMeta, ConnectorPlugin,
    Plugin, PluginCapability, PluginContext, PluginInfo,
};

type SampleRows = Vec<IndexMap<String, Value>>;
type SchemaSample = (Vec<ColumnMeta>, SampleRows);

/// Parquet connector (`id = "parquet"`).
pub struct ParquetConnector {
    info: PluginInfo,
}

impl ParquetConnector {
    /// Create the connector.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new(
                "parquet",
                "Parquet File Connector",
                env!("CARGO_PKG_VERSION"),
            )
            .with_description("Discover Parquet files, map Arrow schemas, sample rows")
            .with_capability(PluginCapability::Connector),
        }
    }

    fn resolve_paths(location: &SourceLocation) -> Result<Vec<PathBuf>> {
        let path = PathBuf::from(&location.uri);
        if !path.exists() {
            return Err(Error::connector(format!(
                "parquet path does not exist: {}",
                path.display()
            )));
        }
        if path.is_file() {
            return Ok(vec![path]);
        }
        let mut files = Vec::new();
        for entry in std::fs::read_dir(&path)
            .map_err(|e| Error::connector(format!("read dir {}: {e}", path.display())))?
        {
            let entry = entry.map_err(|e| Error::connector(e.to_string()))?;
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("parquet") {
                files.push(p);
            }
        }
        files.sort();
        Ok(files)
    }

    fn schema_and_sample(path: &Path, limit: usize) -> Result<SchemaSample> {
        let file = File::open(path)
            .map_err(|e| Error::connector(format!("open {}: {e}", path.display())))?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .map_err(|e| Error::connector(format!("parquet open: {e}")))?;
        let schema = builder.schema().clone();
        let columns = columns_from_schema(&schema);
        let reader = builder
            .build()
            .map_err(|e| Error::connector(format!("parquet reader: {e}")))?;

        let mut rows = Vec::new();
        for batch in reader {
            let batch = batch.map_err(|e| Error::connector(format!("parquet batch: {e}")))?;
            rows.extend(batch_to_rows(&batch));
            if rows.len() >= limit {
                rows.truncate(limit);
                break;
            }
        }
        Ok((columns, rows))
    }
}

impl Default for ParquetConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ParquetConnector {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl ConnectorPlugin for ParquetConnector {
    async fn test_connection(&self, location: &SourceLocation, _ctx: &PluginContext) -> Result<()> {
        let paths = Self::resolve_paths(location)?;
        if paths.is_empty() {
            return Err(Error::connector(
                "parquet: no .parquet files found at location",
            ));
        }
        let _ = Self::schema_and_sample(&paths[0], 1)?;
        Ok(())
    }

    async fn discover(&self, location: &SourceLocation, ctx: &PluginContext) -> Result<Vec<Asset>> {
        let tree = self.discover_catalog(location, ctx).await?;
        Ok(tree
            .all_tables()
            .into_iter()
            .map(|t| {
                Asset::new(t.fqn.clone(), t.name.clone(), t.kind, location.clone())
                    .with_columns(t.columns.clone())
                    .with_tag(
                        "path",
                        t.properties.get("path").cloned().unwrap_or_default(),
                    )
                    .with_tag("format", "parquet")
            })
            .collect())
    }

    async fn discover_catalog(
        &self,
        location: &SourceLocation,
        _ctx: &PluginContext,
    ) -> Result<CatalogTree> {
        let paths = Self::resolve_paths(location)?;
        let mut tree = CatalogTree::new(self.info.id.clone(), location.clone());
        let mut db = CatalogDatabase::new("files");
        let mut schema = CatalogSchema::new("parquet");

        for path in paths {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("table")
                .to_string();
            let (columns, _) = Self::schema_and_sample(&path, 1)?;
            let mut table = CatalogTable::file(path.display().to_string())
                .with_columns(columns)
                .with_property("path", path.display().to_string())
                .with_property("format", "parquet");
            table.name = name;
            table.kind = AssetKind::File;
            table.fqn = format!("files.parquet.{}", table.name);
            info!(
                path = %path.display(),
                columns = table.columns.len(),
                "parquet table discovered"
            );
            schema.tables.push(table);
        }
        db.schemas.push(schema);
        tree.databases.push(db);
        Ok(tree)
    }

    async fn sample_rows(
        &self,
        asset: &Asset,
        limit: usize,
        _ctx: &PluginContext,
    ) -> Result<Vec<IndexMap<String, Value>>> {
        let path = asset
            .tags
            .get("path")
            .cloned()
            .unwrap_or_else(|| asset.location.uri.clone());
        let path = if Path::new(&path).is_file() {
            PathBuf::from(path)
        } else {
            PathBuf::from(&asset.location.uri).join(format!("{}.parquet", asset.name))
        };
        let (_, rows) = Self::schema_and_sample(&path, limit)?;
        Ok(rows)
    }
}

fn columns_from_schema(schema: &Schema) -> Vec<ColumnMeta> {
    schema
        .fields()
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let mut col = ColumnMeta::new(f.name(), map_arrow_type(f.data_type())).at(i as u32);
            col.nullable = f.is_nullable();
            col
        })
        .collect()
}

fn map_arrow_type(dt: &ArrowType) -> DataType {
    match dt {
        ArrowType::Boolean => DataType::Boolean,
        ArrowType::Int8
        | ArrowType::Int16
        | ArrowType::Int32
        | ArrowType::Int64
        | ArrowType::UInt8
        | ArrowType::UInt16
        | ArrowType::UInt32
        | ArrowType::UInt64 => DataType::Integer,
        ArrowType::Float16
        | ArrowType::Float32
        | ArrowType::Float64
        | ArrowType::Decimal128(_, _) => DataType::Float,
        ArrowType::Utf8 | ArrowType::LargeUtf8 => DataType::String,
        ArrowType::Timestamp(_, _) => DataType::Timestamp,
        ArrowType::Date32 | ArrowType::Date64 => DataType::Date,
        ArrowType::Binary | ArrowType::LargeBinary | ArrowType::FixedSizeBinary(_) => {
            DataType::Binary
        }
        ArrowType::List(_) | ArrowType::Struct(_) | ArrowType::Map(_, _) => DataType::Complex,
        _ => DataType::Unknown,
    }
}

fn batch_to_rows(batch: &RecordBatch) -> SampleRows {
    let n = batch.num_rows();
    let mut rows = vec![IndexMap::new(); n];
    for (col_idx, field) in batch.schema().fields().iter().enumerate() {
        let name = field.name();
        let array = batch.column(col_idx);
        for (row_idx, row) in rows.iter_mut().enumerate().take(n) {
            let v = if array.is_null(row_idx) {
                Value::Null
            } else {
                arrow_cell_json(array.as_ref(), row_idx)
            };
            row.insert(name.clone(), v);
        }
    }
    rows
}

fn arrow_cell_json(array: &dyn Array, row: usize) -> Value {
    match array.data_type() {
        ArrowType::Boolean => {
            let a = array.as_boolean();
            json!(a.value(row))
        }
        ArrowType::Int64 => {
            let a = array.as_primitive::<Int64Type>();
            json!(a.value(row))
        }
        ArrowType::Int32 => {
            let a = array.as_primitive::<Int32Type>();
            json!(a.value(row))
        }
        ArrowType::Float64 => {
            let a = array.as_primitive::<Float64Type>();
            json!(a.value(row))
        }
        ArrowType::Utf8 => {
            let a = array.as_string::<i32>();
            json!(a.value(row))
        }
        ArrowType::LargeUtf8 => {
            let a = array.as_string::<i64>();
            json!(a.value(row))
        }
        other => json!(format!("<{other:?}>")),
    }
}
