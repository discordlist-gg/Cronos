use tantivy::schema::Field;
use tantivy::Document;

mod index;
pub mod index_impls;
mod queries;
pub mod readers;
mod tokenizer;
mod writer;

pub trait FromTantivyDoc: Sized {
    fn from_doc(id_field: Field, doc: Document) -> anyhow::Result<Self>;
}
