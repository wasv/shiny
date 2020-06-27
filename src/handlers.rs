use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Clone)]
pub struct RenderContext {
    pub content: String,
    pub filter: String,
}

pub fn render(ctx: &RenderContext) -> std::result::Result<String, String> {
    let mut filter = match Command::new("sh")
        .arg("-c")
        .arg(ctx.filter.clone())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(v) => v,
        Err(e) => {
            return Err(format!("Error spawning process: {}", e));
        }
    };

    if let Some(stdin) = filter.stdin.as_mut() {
        if let Err(e) = stdin.write_all(ctx.content.as_bytes()) {
            return Err(format!("'Error writing input: {}'", e));
        }
    } else {
        return Err("Could not open stdin.".to_owned());
    };

    match filter.wait_with_output() {
        Ok(output) => Ok(String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .escape_default()
            .to_string()),
        Err(e) => Err(format!("Error reading output: {}", e)),
    }
}
