mod json;
mod sql;
mod xml;

use crate::tool::Tool;

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(json::JsonFormatter::default()),
        Box::new(sql::SqlFormatter::default()),
        Box::new(xml::XmlFormatter::default()),
    ]
}
