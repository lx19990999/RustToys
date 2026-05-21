mod ping;

use crate::tool::Tool;

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ping::PingSpeedTest::default()),
    ]
}
