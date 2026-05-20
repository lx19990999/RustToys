mod cron_parser;
mod date_converter;
mod json_table;
mod json_yaml;
mod number_base;

use crate::tool::Tool;

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(cron_parser::CronParser::default()),
        Box::new(date_converter::DateConverter::default()),
        Box::new(json_table::JsonTable::default()),
        Box::new(json_yaml::JsonYaml::default()),
        Box::new(number_base::NumberBase::default()),
    ]
}
