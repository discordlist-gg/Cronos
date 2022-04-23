use poem_openapi::Object;
use backend_common::FieldNamesAsArray;

#[derive(Object, FieldNamesAsArray)]
#[oai(rename_all = "camelCase")]
pub struct Bot {

}
