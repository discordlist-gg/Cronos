use tantivy::schema::Schema;
use tantivy::Document;

mod index;
mod queries;
mod readers;
mod tokenizer;

pub trait FromTantivyDoc: Sized {
    fn from_doc(schema: &Schema, doc: Document) -> anyhow::Result<Self>;
}
