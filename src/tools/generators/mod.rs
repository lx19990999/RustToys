mod hash;
mod lorem_ipsum;
mod password;
mod uuid_gen;

use crate::tool::Tool;

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(hash::HashGenerator::default()),
        Box::new(lorem_ipsum::LoremIpsum::default()),
        Box::new(password::PasswordGenerator::default()),
        Box::new(uuid_gen::UuidGenerator::default()),
    ]
}
