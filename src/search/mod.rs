use tantivy::schema::Schema;
use tantivy::Document;

mod index;
mod queries;
pub mod readers;
mod tokenizer;
mod writer;
pub mod index_impls;

pub trait FromTantivyDoc: Sized {
    fn from_doc(schema: &Schema, doc: Document) -> anyhow::Result<Self>;
}
