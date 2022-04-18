use poem_openapi::Object;

#[derive(Object)]
#[oai(rename_all = "camelCase")]
pub struct Bot {}
