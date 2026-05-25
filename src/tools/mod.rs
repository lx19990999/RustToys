pub mod async_utils;
pub mod io_layout;
pub mod converters;
pub mod encoders;
pub mod encryption;
pub mod formatters;
pub mod generators;
pub mod graphic;
pub mod testers;
pub mod text;
pub mod network;

use crate::tool::Tool;

pub fn all_tools() -> Vec<Box<dyn Tool>> {
    let mut tools: Vec<Box<dyn Tool>> = Vec::new();
    tools.extend(converters::tools());
    tools.extend(encoders::tools());
    tools.extend(encryption::tools());
    tools.extend(formatters::tools());
    tools.extend(generators::tools());
    tools.extend(graphic::tools());
    tools.extend(testers::tools());
    tools.extend(text::tools());
    tools.extend(network::tools());
    tools
}
