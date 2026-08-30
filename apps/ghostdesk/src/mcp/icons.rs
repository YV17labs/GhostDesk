use std::sync::LazyLock;

use nest_rs::mcp::model::Icon;

const LOGO_SVG: &str = concat!(
    r##"<svg width="400" height="400" viewBox="240 210 200 240" "##,
    r##"xmlns="http://www.w3.org/2000/svg">"##,
    r##"<path d="M260 430 L260 310 C260 265, 280 230, 340 230 "##,
    r##"C400 230, 420 265, 420 310 L420 430 L400 400 L380 430 "##,
    r##"L360 400 L340 430 L320 400 L300 430 L280 400 Z" "##,
    r##"fill="#7B6FDE" opacity="0.85"/>"##,
    r##"<rect x="285" y="275" width="110" height="80" rx="6" fill="#0D0E1A"/>"##,
    r##"<rect x="295" y="290" width="45" height="4" rx="2" fill="#5DCAA5" opacity="0.9"/>"##,
    r##"<rect x="295" y="302" width="70" height="4" rx="2" fill="#5DCAA5" opacity="0.6"/>"##,
    r##"<rect x="295" y="314" width="35" height="4" rx="2" fill="#5DCAA5" opacity="0.4"/>"##,
    r##"<rect x="335" y="314" width="8" height="4" rx="1" fill="#5DCAA5" opacity="0.9"/>"##,
    r##"<rect x="295" y="326" width="55" height="4" rx="2" fill="#5DCAA5" opacity="0.3"/>"##,
    r##"<circle cx="310" cy="260" r="10" fill="#E8E6FF"/>"##,
    r##"<circle cx="370" cy="260" r="10" fill="#E8E6FF"/>"##,
    r##"<circle cx="313" cy="261" r="5" fill="#1A1B2E"/>"##,
    r##"<circle cx="373" cy="261" r="5" fill="#1A1B2E"/>"##,
    r##"<path d="M385 345 L385 365 L392 358 L400 370 L404 368 L396 356 L405 353 Z" "##,
    r##"fill="#FFFFFF" opacity="0.9"/>"##,
    r##"</svg>"##,
);

static DATA_URI: LazyLock<String> = LazyLock::new(|| {
    use base64::Engine as _;
    format!(
        "data:image/svg+xml;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(LOGO_SVG)
    )
});

pub fn icons() -> Vec<Icon> {
    vec![Icon::new(DATA_URI.clone()).with_mime_type("image/svg+xml")]
}
