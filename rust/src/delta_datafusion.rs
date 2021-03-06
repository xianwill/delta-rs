use std::any::Any;
use std::sync::Arc;

use arrow::datatypes::{
    DataType as ArrowDataType, Field as ArrowField, Schema as ArrowSchema, TimeUnit,
};
use datafusion::datasource::datasource::Statistics;
use datafusion::datasource::TableProvider;
use datafusion::physical_plan::parquet::ParquetExec;
use datafusion::physical_plan::ExecutionPlan;

use crate::delta;
use crate::schema;

impl TableProvider for delta::DeltaTable {
    fn schema(&self) -> Arc<ArrowSchema> {
        Arc::new(<ArrowSchema as From<&schema::Schema>>::from(
            delta::DeltaTable::schema(&self).unwrap(),
        ))
    }

    fn scan(
        &self,
        projection: &Option<Vec<usize>>,
        batch_size: usize,
    ) -> datafusion::error::Result<Arc<dyn ExecutionPlan>> {
        let schema =
            <ArrowSchema as From<&schema::Schema>>::from(delta::DeltaTable::schema(&self).unwrap());
        let filenames = self.get_file_paths();

        Ok(Arc::new(ParquetExec::new(
            filenames,
            schema,
            projection.clone(),
            batch_size,
        )))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn statistics(&self) -> Statistics {
        // TODO: proxy delta table stats after https://github.com/delta-io/delta.rs/issues/45 has
        // been completed
        Statistics::default()
    }
}

impl From<&schema::Schema> for ArrowSchema {
    fn from(s: &schema::Schema) -> Self {
        let fields = s
            .get_fields()
            .iter()
            .map(|field| <ArrowField as From<&schema::SchemaField>>::from(field))
            .collect();

        ArrowSchema::new(fields)
    }
}

impl From<&schema::SchemaField> for ArrowField {
    fn from(f: &schema::SchemaField) -> Self {
        ArrowField::new(
            f.get_name(),
            ArrowDataType::from(f.get_type()),
            f.is_nullable(),
        )
    }
}

impl From<&schema::SchemaTypeArray> for ArrowField {
    fn from(a: &schema::SchemaTypeArray) -> Self {
        ArrowField::new(
            "",
            ArrowDataType::from(a.get_element_type()),
            a.contains_null(),
        )
    }
}

impl From<&schema::SchemaDataType> for ArrowDataType {
    fn from(t: &schema::SchemaDataType) -> Self {
        match t {
            schema::SchemaDataType::primitive(p) => {
                match p.as_str() {
                    "string" => ArrowDataType::Utf8,
                    "long" => ArrowDataType::Int64, // undocumented type
                    "integer" => ArrowDataType::Int32,
                    "short" => ArrowDataType::Int16,
                    "byte" => ArrowDataType::Int8,
                    "float" => ArrowDataType::Float32,
                    "double" => ArrowDataType::Float64,
                    "boolean" => ArrowDataType::Boolean,
                    "binary" => ArrowDataType::Binary,
                    "date" => {
                        // A calendar date, represented as a year-month-day triple without a
                        // timezone.
                        panic!("date is not supported in arrow");
                    }
                    "timestamp" => {
                        // Microsecond precision timestamp without a timezone.
                        ArrowDataType::Time64(TimeUnit::Microsecond)
                    }
                    s => {
                        panic!("unexpected delta schema type: {}", s);
                    }
                }
            }
            schema::SchemaDataType::r#struct(s) => ArrowDataType::Struct(
                s.get_fields()
                    .iter()
                    .map(|f| <ArrowField as From<&schema::SchemaField>>::from(f))
                    .collect(),
            ),
            schema::SchemaDataType::array(a) => ArrowDataType::List(Box::new(
                <ArrowField as From<&schema::SchemaTypeArray>>::from(a),
            )),
            schema::SchemaDataType::map(m) => ArrowDataType::Dictionary(
                Box::new(<ArrowDataType as From<&schema::SchemaDataType>>::from(
                    m.get_key_type(),
                )),
                Box::new(<ArrowDataType as From<&schema::SchemaDataType>>::from(
                    m.get_value_type(),
                )),
            ),
        }
    }
}
