mod jsonpath;
mod regex_tester;
mod xml_tester;

use crate::tool::Tool;

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(jsonpath::JsonPathTester::default()),
        Box::new(regex_tester::RegexTester::default()),
        Box::new(xml_tester::XmlTester::default()),
    ]
}
