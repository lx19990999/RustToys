mod ping;
mod udp;
mod websocket;

use crate::tool::Tool;

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ping::PingSpeedTest::default()),
        Box::new(udp::UdpTool::default()),
        Box::new(websocket::WebSocketTool::default()),
    ]
}
