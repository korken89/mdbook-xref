use std::{
    io::Write,
    process::{Command, Stdio},
};

pub fn generate(typst: &str) -> anyhow::Result<String> {
    let mut command = Command::new("typst");
    command
        .args(["compile", "--format", "svg", "-", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn()?;
    child.stdin.take().unwrap().write_all(typst.as_bytes())?;

    let output = child.wait_with_output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "compiling typst failed: {stderr}");

    String::from_utf8(output.stdout).map_err(Into::into)
}
