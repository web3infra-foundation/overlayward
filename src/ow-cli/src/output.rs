use crate::OutputFormat;

#[inline]
pub fn print(fmt: OutputFormat, value: &serde_json::Value) {
    match fmt {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(value).unwrap_or_default()),
        OutputFormat::Text => print_text(value, 0),
    }
}

#[inline]
pub fn msg(fmt: OutputFormat, message: &str) {
    match fmt {
        OutputFormat::Json => println!("{{\"message\":\"{message}\"}}"),
        OutputFormat::Text => println!("{message}"),
    }
}

fn print_text(val: &serde_json::Value, indent: usize) {
    let pad = "  ".repeat(indent);
    match val {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                if v.is_object() || v.is_array() {
                    println!("{pad}{k}:");
                    print_text(v, indent + 1);
                } else {
                    println!("{pad}{k}: {}", format_scalar(v));
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr { print_text(item, indent); if indent == 0 { println!("---"); } }
        }
        other => println!("{pad}{}", format_scalar(other)),
    }
}

#[inline]
fn format_scalar(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => "(none)".into(),
        other => other.to_string(),
    }
}
