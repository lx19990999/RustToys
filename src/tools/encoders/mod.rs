mod base64_image;
mod base64_text;
mod certificate;
mod gzip;
mod html;
mod jwt;
mod morse;
mod qrcode;
mod url;

use crate::tool::Tool;

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(base64_text::Base64Text::default()),
        Box::new(base64_image::Base64Image::default()),
        Box::new(certificate::CertificateDecoder::default()),
        Box::new(gzip::GZip::default()),
        Box::new(html::HtmlEncoder::default()),
        Box::new(jwt::JwtDecoder::default()),
        Box::new(morse::MorseCode::default()),
        Box::new(qrcode::QrCode::default()),
        Box::new(url::UrlEncoder::default()),
    ]
}
