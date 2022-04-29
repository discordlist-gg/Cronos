use tantivy::query::{BooleanQuery, BoostQuery, FuzzyTermQuery, Occur, Query};
use tantivy::schema::Field;
use tantivy::Term;

use crate::search::tokenizer::{SimpleTokenStream, SimpleUnicodeTokenizer};

macro_rules! add_if_exists {
    ($collector:expr, $qry:expr) => {{
        if let Some(query) = $qry {
            $collector.push(query);
        }
    }};
}

pub fn parse_query(query: &str, fields: &[Field]) -> Vec<Box<dyn Query>> {
    let tokenizer = SimpleUnicodeTokenizer::with_limit(10);
    let mut token_stream = tokenizer.token_stream(query);
    let mut stages = vec![];

    add_if_exists!(stages, build_fuzzy_stage(0, 0, fields, &mut token_stream));
    add_if_exists!(stages, build_fuzzy_stage(1, 4, fields, &mut token_stream));
    add_if_exists!(stages, build_fuzzy_stage(2, 8, fields, &mut token_stream));

    stages
}


fn build_fuzzy_stage(dist: u8, length_cut_off: usize, fields: &[Field], token_stream: &mut SimpleTokenStream) -> Option<Box<dyn Query>> {
    let mut stage = {
        let mut inner = vec![];
        for _ in 0..fields.len() {
            inner.push(vec![]);
        }

        inner
    };

    while let Some(token) = token_stream.next() {
        if token.text.len() < length_cut_off {
            continue;
        }

        for (i, field) in fields.iter().copied().enumerate() {
            let term = Term::from_field_text(field, token.text.as_str());
            stage[i].push((
                Occur::Should,
                Box::new(FuzzyTermQuery::new_prefix(term, dist, true)) as Box<dyn Query>
            ));
        }
    }
    token_stream.reset();

    if stage[0].is_empty() {
        return None
    }

    let mut boost_factor = 1.0;
    let mut built_queries = vec![];
    for field_stage in stage {
        let boolean = Box::new(BooleanQuery::new(field_stage));
        let boosted = Box::new(BoostQuery::new(boolean, boost_factor)) as Box<dyn Query>;

        built_queries.push((Occur::Should, boosted));
        boost_factor -= 0.10;
    }

    Some(Box::new(BooleanQuery::new(built_queries)))
}