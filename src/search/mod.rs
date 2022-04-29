use tantivy::Document;
use tantivy::schema::Schema;

mod index;
mod queries;
mod tokenizer;
mod readers;

pub trait FromTantivyDoc: Sized {
    fn from_doc(schema: &Schema, doc: Document) -> anyhow::Result<Self>;
}