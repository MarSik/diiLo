use super::Part;

#[derive(Debug)]
pub struct Query {
    query: String,
}

impl Query {
    pub fn new(query: &str) -> Result<Self, QueryError> {
        Ok(Self {
            query: query.to_lowercase(),
        })
    }

    pub fn matches(&self, part: &Part) -> bool {
        part.metadata.name.to_lowercase().contains(&self.query)
            || part.metadata.summary.to_lowercase().contains(&self.query)
            || part.content.to_lowercase().contains(&self.query)
    }

    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }

    pub(crate) fn current_query(&self) -> String {
        self.query.clone()
    }
}

impl std::fmt::Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("query: {}", &self.query))
    }
}

#[derive(Debug)]
pub enum QueryError {}
