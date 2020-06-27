use std::io::Write;
use std::process::{Command, Stdio};
use web_view::WebView;

pub struct RenderContext {
    pub content: String,
    pub filter: String,
}

pub fn render(wv: &mut WebView<RenderContext>) {
    let context = wv.user_data();
    let mut filter = match Command::new("sh")
        .arg("-c")
        .arg(context.filter.clone())
        .stdin(Stdio::piped())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(v) => v,
        Err(e) => {
            let js = format!("console.error('Error spawning process: {}')", e);
            let _ = wv.eval(&js);
            return;
        }
    };

    if let Some(stdin) = filter.stdin.as_mut() {
        if let Err(e) = stdin.write_all(context.content.as_bytes()) {
            let js = format!("console.error('Error writing input: {}')", e);
            let _ = wv.eval(&js);
            return;
        }
    } else {
        let js = "console.error('Could not open stdin.')";
        let _ = wv.eval(&js);
        return;
    };

    match filter.wait_with_output() {
        Ok(output) => {
            let js = format!(
                "document.getElementById('output').srcdoc = '{}'",
                String::from_utf8_lossy(&output.stdout)
                    .trim_end()
                    .escape_default()
            );
            let _ = wv.eval(&js);
        }
        Err(e) => {
            let js = format!("console.error('Error reading output: {}')", e);
            let _ = wv.eval(&js);
        }
    };
}
