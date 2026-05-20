mod symmetric;
mod asymmetric;

use crate::tool::Tool;

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(symmetric::SymmetricEncryption::default()),
        Box::new(asymmetric::AsymmetricEncryption::default()),
    ]
}
