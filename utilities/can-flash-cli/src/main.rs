use std::{
    fs::File,
    io::{self, BufRead, BufReader, IsTerminal, Read, Write},
    path::PathBuf,
    process::ExitCode,
    time::Duration,
};

use anyhow::{Context, Result, bail, ensure};
use clap::{Args, Parser, Subcommand};
use reqwest::{
    StatusCode, Url,
    blocking::{Client, multipart},
    redirect::Policy,
};

#[derive(Parser)]
#[command(name = "can", version, about = "Firmware tools")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Upload an ELF to the server for CAN flashing.
    Flash(CanFlash),
}

#[derive(Args)]
struct CanFlash {
    /// Server base URL, including scheme and port.
    #[arg(long, env = "CAN_FLASH_HOST", hide_env_values = true)]
    host: Url,
    /// ECU name; overrides package.metadata.can-flash.ecu from Cargo.toml.
    #[arg(long)]
    ecu: Option<String>,
    /// Application ELF to flash instead of building.
    #[arg(long, conflicts_with_all = ["package", "features", "all_features", "no_default_features"])]
    elf: Option<PathBuf>,
    /// Cargo package to build and flash when running from a workspace root.
    #[arg(short, long)]
    package: Option<String>,
    /// Features to build with, as for cargo build.
    #[arg(short = 'F', long)]
    features: Vec<String>,
    /// Build with all features enabled.
    #[arg(long)]
    all_features: bool,
    /// Build without the default features.
    #[arg(long)]
    no_default_features: bool,
    /// Skip requesting entry when the ECU is already in the bootloader.
    #[arg(long)]
    already_in_bootloader: bool,
}

// Maximum wait for the server to respond or send the next progress update.
// Keep above the server's 60 s upload limit so its error is reported instead.
const SERVER_TIMEOUT: Duration = Duration::from_secs(120);

fn cargo_json(arguments: &[&str]) -> Result<serde_json::Value> {
    let output = std::process::Command::new("cargo")
        .args(arguments)
        .output()
        .context("cannot run Cargo; supply --elf and --ecu to flash without Cargo metadata")?;
    ensure!(
        output.status.success(),
        "Cargo failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    serde_json::from_slice(&output.stdout).context("invalid Cargo metadata")
}

fn find_package(package_name: Option<&str>) -> Result<serde_json::Value> {
    let metadata = cargo_json(&[
        "metadata",
        "--no-deps",
        "--offline",
        "--format-version",
        "1",
    ])?;
    let current = cargo_json(&["locate-project", "--message-format", "json"])?;
    let packages = metadata["packages"]
        .as_array()
        .context("Cargo returned no packages")?;
    let members = metadata["workspace_members"]
        .as_array()
        .context("Cargo returned no workspace members")?;
    let workspace_packages: Vec<_> = packages
        .iter()
        .filter(|p| members.contains(&p["id"]))
        .collect();
    let package = if let Some(name) = package_name {
        workspace_packages
            .iter()
            .copied()
            .find(|p| p["name"].as_str() == Some(name))
            .with_context(|| format!("package {name} is not in this workspace"))?
    } else if let Some(package) = workspace_packages
        .iter()
        .copied()
        .find(|p| p["manifest_path"] == current["root"])
    {
        package
    } else if workspace_packages.len() == 1 {
        workspace_packages[0]
    } else {
        bail!("select a project with -p <package>, or run from its directory");
    };
    Ok(package.clone())
}

/// Build the package's release binary and return the built ELF.
fn build_elf(package: &serde_json::Value, args: &CanFlash) -> Result<PathBuf> {
    let name = package["name"].as_str().context("package has no name")?;
    let mut command = std::process::Command::new("cargo");
    command.args([
        "build",
        "--release",
        "--package",
        name,
        "--message-format=json-render-diagnostics",
    ]);
    for features in &args.features {
        command.args(["--features", features]);
    }
    if args.all_features {
        command.arg("--all-features");
    }
    if args.no_default_features {
        command.arg("--no-default-features");
    }
    // Build progress and errors stay on stderr; stdout carries the artifact messages.
    let output = command
        .stderr(std::process::Stdio::inherit())
        .output()
        .context("cannot run Cargo; supply --elf and --ecu to flash without building")?;
    ensure!(output.status.success(), "build failed");

    let mut binaries = Vec::new();
    for line in output.stdout.split(|&b| b == b'\n') {
        let Ok(message) = serde_json::from_slice::<serde_json::Value>(line) else {
            continue;
        };
        let is_binary = message["reason"] == "compiler-artifact"
            && message["package_id"] == package["id"]
            && message["target"]["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"));
        if let (true, Some(path)) = (is_binary, message["executable"].as_str()) {
            binaries.push(PathBuf::from(path));
        }
    }
    match binaries.as_slice() {
        [path] => Ok(path.clone()),
        [] => bail!("package has no binary file to flash"),
        _ => bail!("multiple binary files found in package; select one with --elf"),
    }
}

fn endpoint(mut base: Url) -> Result<Url> {
    ensure!(
        matches!(base.scheme(), "http" | "https") && base.host_str().is_some(),
        "--host must be an http:// or https:// server URL"
    );
    ensure!(
        base.username().is_empty()
            && base.password().is_none()
            && base.query().is_none()
            && base.fragment().is_none(),
        "--host must not contain credentials, a query, or a fragment"
    );
    let path = format!("{}/flash", base.path().trim_end_matches('/'));
    base.set_path(&path);
    Ok(base)
}

fn flash(args: CanFlash) -> Result<()> {
    let project = if args.elf.is_none() || args.ecu.is_none() {
        Some(find_package(args.package.as_deref()).context(
            "cannot select application; use -p <package> or run from its directory; for an explicit ELF outside a project, also pass --ecu",
        )?)
    } else {
        None
    };
    let ecu = match args.ecu.as_deref() {
        Some(ecu) => ecu,
        None => project
            .as_ref()
            .and_then(|package| package["metadata"]["can-flash"]["ecu"].as_str())
            .context("missing or invalid package.metadata.can-flash.ecu in Cargo.toml; pass --ecu explicitly")?,
    };
    ensure!(!ecu.trim().is_empty(), "ECU name must not be empty");
    let elf = match &args.elf {
        Some(path) => path.clone(),
        None => {
            let package = project.as_ref().context("missing Cargo project")?;
            build_elf(package, &args)?
        }
    };
    let url = endpoint(args.host)?;
    let mut file = File::open(&elf).with_context(|| format!("cannot open {}", elf.display()))?;
    ensure!(file.metadata()?.is_file(), "--elf must be a regular file");
    let mut magic = [0; 4];
    file.read_exact(&mut magic)
        .context("cannot read ELF header")?;
    ensure!(magic == *b"\x7fELF", "--elf must point to an ELF file");

    let form = multipart::Form::new()
        .text("ecu", ecu.to_owned())
        .text(
            "already_in_bootloader",
            args.already_in_bootloader.to_string(),
        )
        .file("file", &elf)?;
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(SERVER_TIMEOUT)
        // A flash request must not be silently forwarded to another endpoint.
        .redirect(Policy::none())
        .build()?;
    println!(
        "Flashing {} via {url} using {}",
        ecu.to_ascii_uppercase(),
        elf.display()
    );
    let mut request = client.post(url).multipart(form).build()?;
    // Buffer the upload so an early HTTP rejection is not masked by a closed body channel.
    if let Some(body) = request.body_mut() {
        body.buffer().context("cannot prepare firmware upload")?;
    }
    // Do not retry: a lost response does not mean the flash operation failed.
    let response = client.execute(request).context(
        "Server request failed; the ECU may still be flashing. Check server status before retrying",
    )?;
    let status = response.status();
    let streaming = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.split(';').next() == Some("application/x-ndjson"));
    if status == StatusCode::OK && streaming {
        let result = display_progress(response, &elf);
        if result.is_err() && io::stdout().is_terminal() {
            println!();
        }
        return result;
    }
    let mut body = String::new();
    response
        .take(16 * 1024)
        .read_to_string(&mut body)
        .with_context(|| {
            format!(
                "Server returned {status}, but its response body could not be read; check server status before retrying"
            )
        })?;
    if status != StatusCode::OK {
        bail!("{}", http_error_message(status, &body));
    }
    println!(
        "{}",
        if body.trim().is_empty() {
            "Flash completed"
        } else {
            body.trim()
        }
    );
    Ok(())
}

fn http_error_message(status: StatusCode, body: &str) -> String {
    let message = body.trim();
    if message.is_empty() {
        format!("HTTP {status}")
    } else {
        message.to_owned()
    }
}

fn display_progress(response: impl Read, elf: &std::path::Path) -> Result<()> {
    let mut reader = BufReader::new(response);
    let terminal = io::stdout().is_terminal();
    loop {
        let mut line = String::new();
        let count = reader
            .by_ref()
            .take(16 * 1024 + 1)
            .read_line(&mut line)
            .context("progress connection lost; check server status before retrying")?;
        ensure!(
            count > 0,
            "progress stream ended without a result; check server status before retrying"
        );
        ensure!(count <= 16 * 1024, "server progress event is too large");
        let event: serde_json::Value =
            serde_json::from_str(&line).context("invalid server progress event")?;
        match event["type"].as_str() {
            Some("firmware") => {
                let format = event["format"]
                    .as_str()
                    .context("missing firmware format")?;
                let address = event["address"]
                    .as_u64()
                    .context("missing firmware address")?;
                let size = event["size"].as_u64().context("missing firmware size")?;
                let crc32 = event["crc32"].as_u64().context("missing firmware CRC32")?;
                println!("Firmware: {}", elf.display());
                println!("Format:   {format}");
                println!("Address:  0x{address:08X}");
                println!("Size:     {size} bytes");
                println!("CRC32:    0x{crc32:08X}");
            }
            Some("progress") => {
                let percent = event["percent"]
                    .as_f64()
                    .context("missing progress percentage")?;
                ensure!(
                    percent.is_finite() && (0.0..=100.0).contains(&percent),
                    "invalid progress percentage"
                );
                let stage = event["stage"].as_str().context("missing progress stage")?;
                let detail = event["detail"].as_str().unwrap_or("");
                let filled = (percent / 100.0 * 24.0).round() as usize;
                let bar = format!("{}{}", "#".repeat(filled), "-".repeat(24 - filled));
                if terminal {
                    print!("\r\x1b[2K[{bar}] {percent:5.1}%  {stage:<20} {detail}");
                    io::stdout().flush()?;
                } else {
                    println!("[{bar}] {percent:5.1}%  {stage} {detail}");
                }
            }
            Some("complete") => {
                let message = event["message"]
                    .as_str()
                    .context("missing completion message")?;
                if terminal {
                    println!();
                }
                println!("{message}");
                return Ok(());
            }
            Some("error") => bail!(
                "{}",
                event["message"].as_str().unwrap_or("server flash failed")
            ),
            _ => bail!("unknown server progress event"),
        }
    }
}

fn format_error(error: &anyhow::Error) -> String {
    let details = format!("{error:#}").to_ascii_lowercase();
    let request = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<reqwest::Error>());

    if details.contains("can interface is busy") || details.contains("http 409") {
        "Server is busy. Try again when the current operation finishes.".into()
    } else if details.contains("progress connection lost")
        || details.contains("progress stream ended without a result")
    {
        "Connection lost. Check ECU status before retrying.".into()
    } else if details.contains("response body could not be read") {
        error.to_string()
    } else if let Some(request) = request {
        request_error(request, &details).into()
    } else if details.contains("can") && details.contains("network is down") {
        "CAN interface is down.".into()
    } else if details.contains("can transmit failed") {
        "Could not send data over CAN.".into()
    } else if details.contains("cannot open") && details.contains("failed to open can interface") {
        "Cannot open the CAN interface. Check the server's interface configuration.".into()
    } else if details.contains("upload timed out") {
        "Firmware upload timed out.".into()
    } else if details.contains("server progress event")
        || details.starts_with("missing firmware ")
        || details.starts_with("missing progress ")
        || details.starts_with("missing completion ")
    {
        "Flashing failed.".into()
    } else {
        error.to_string()
    }
}

fn request_error(error: &reqwest::Error, details: &str) -> &'static str {
    if error.is_connect()
        && (details.contains("dns")
            || details.contains("failed to lookup address")
            || details.contains("name or service not known"))
    {
        "Cannot resolve server address."
    } else if error.is_connect() {
        "Cannot connect to server. Check the address and that the server is running."
    } else if error.is_timeout() {
        "Server request timed out. Check ECU status before retrying."
    } else if details.contains("server request failed") {
        "Connection lost. Check ECU status before retrying."
    } else {
        "Could not prepare the server request."
    }
}

fn main() -> ExitCode {
    let Cli {
        command: Command::Flash(args),
    } = Cli::parse();
    match flash(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {}", format_error(&error));
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, net::TcpListener, thread};

    #[test]
    fn error_messages() {
        for (details, expected) in [
            (
                "CAN interface is busy",
                "Server is busy. Try again when the current operation finishes.",
            ),
            (
                "Flash failed: CAN transmit failed: Network is down (os error 100)",
                "CAN interface is down.",
            ),
            (
                "Flash failed: CAN transmit failed: No buffer space available (os error 105)",
                "Could not send data over CAN.",
            ),
            (
                "progress stream ended without a result; check server status before retrying",
                "Connection lost. Check ECU status before retrying.",
            ),
        ] {
            assert_eq!(format_error(&anyhow::anyhow!(details)), expected);
        }
        let error = anyhow::anyhow!("file does not exist").context("cannot open firmware.elf");
        assert_eq!(format_error(&error), "cannot open firmware.elf");
    }

    #[test]
    fn server_errors_pass_through() {
        for message in [
            "Flash failed: get-status failed after 3 attempts: timed out waiting for ACK to command 0xA2",
            "Flash failed: bootloader rejected command 0xA5",
            "Flash failed: write block timed out",
            "Flash failed: bootloader returned CRC 0x12345678; expected 0x87654321",
            "firmware is larger than the application region",
            "ELF is not ARM",
            "Flash failed: unexpected internal error",
            "HTTP 500 Internal Server Error",
        ] {
            assert_eq!(format_error(&anyhow::anyhow!(message)), message);
        }
    }

    #[test]
    fn server_url_validation() {
        assert_eq!(
            endpoint(Url::parse("http://localhost:8080/api/").unwrap())
                .unwrap()
                .as_str(),
            "http://localhost:8080/api/flash"
        );
        for url in [
            "ftp://localhost",
            "http://user:pass@localhost",
            "http://localhost/?x=1",
        ] {
            assert!(endpoint(Url::parse(url).unwrap()).is_err());
        }
    }

    #[test]
    fn http_error_fallback() {
        for (status, body, expected) in [
            (
                StatusCode::CONFLICT,
                " CAN interface is busy\n",
                "CAN interface is busy",
            ),
            (
                StatusCode::BAD_REQUEST,
                "invalid firmware",
                "invalid firmware",
            ),
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "Cannot open can0",
                "Cannot open can0",
            ),
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                " \n",
                "HTTP 500 Internal Server Error",
            ),
        ] {
            assert_eq!(http_error_message(status, body), expected);
        }
    }

    #[test]
    fn early_rejections() {
        let path =
            std::env::temp_dir().join(format!("can-flash-cli-early-test-{}", std::process::id()));
        let mut file = File::create(&path).unwrap();
        file.write_all(b"\x7fELF").unwrap();
        file.set_len(8 * 1024 * 1024).unwrap();
        drop(file);

        for (status, message, expected) in [
            (
                "409 Conflict",
                "CAN interface is busy",
                "CAN interface is busy",
            ),
            (
                "413 Payload Too Large",
                "upload too large",
                "upload too large",
            ),
            (
                "401 Unauthorized",
                "authentication required",
                "authentication required",
            ),
            (
                "500 Internal Server Error",
                "server failed",
                "server failed",
            ),
        ] {
            let listener = TcpListener::bind(("localhost", 0)).unwrap();
            let host = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
            let worker =
                thread::spawn(move || {
                    let (mut socket, _) = listener.accept().unwrap();
                    socket
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut reader = BufReader::new(socket.try_clone().unwrap());
                    // Reject after the headers, while the large upload is still in progress.
                    loop {
                        let mut line = String::new();
                        assert!(reader.read_line(&mut line).unwrap() > 0);
                        if line == "\r\n" {
                            break;
                        }
                    }
                    write!(socket,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{message}",
                    message.len()
                ).unwrap();
                    socket.shutdown(std::net::Shutdown::Write).unwrap();
                    // Drain already-sent bytes to avoid a TCP reset hiding the HTTP response.
                    let _ = io::copy(&mut reader, &mut io::sink());
                });
            let error = flash(CanFlash {
                host,
                ecu: Some("bms".into()),
                elf: Some(path.clone()),
                package: None,
                features: Vec::new(),
                all_features: false,
                no_default_features: false,
                already_in_bootloader: true,
            })
            .unwrap_err();
            worker.join().unwrap();
            let error = format!("{error:#}");
            assert_eq!(error, expected);
            assert!(error.contains(message), "{error}");
            assert!(!error.contains("receiver is gone"), "{error}");
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn multipart_upload_and_http_results() {
        let path = std::env::temp_dir().join(format!("can-flash-cli-test-{}", std::process::id()));
        std::fs::write(&path, b"\x7fELFtest-payload").unwrap();
        for (status, succeeds) in [
            ("200 OK", true),
            ("409 Conflict", false),
            ("500 Internal Server Error", false),
            ("202 Accepted", false),
        ] {
            let listener = TcpListener::bind(("localhost", 0)).unwrap();
            let host = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
            let worker = thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                let mut chunk = [0; 4096];
                loop {
                    let n = socket.read(&mut chunk).unwrap();
                    assert!(n > 0);
                    request.extend_from_slice(&chunk[..n]);
                    if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&request[..end]).to_lowercase();
                        let length: usize = headers
                            .lines()
                            .find_map(|line| line.strip_prefix("content-length: "))
                            .unwrap()
                            .parse()
                            .unwrap();
                        if request.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                write!(
                    socket,
                    "HTTP/1.1 {status}\r\nContent-Length: 6\r\nConnection: close\r\n\r\nresult"
                )
                .unwrap();
                request
            });
            let result = flash(CanFlash {
                host,
                ecu: Some("bms".into()),
                elf: Some(path.clone()),
                package: None,
                features: Vec::new(),
                all_features: false,
                no_default_features: false,
                already_in_bootloader: true,
            });
            assert_eq!(result.is_ok(), succeeds, "{result:?}");
            let request = String::from_utf8(worker.join().unwrap()).unwrap();
            assert!(request.starts_with("POST /flash HTTP/1.1"));
            assert!(request.contains("multipart/form-data; boundary="));
            assert!(request.contains("name=\"ecu\"\r\n\r\nbms"));
            assert!(request.contains("name=\"already_in_bootloader\"\r\n\r\ntrue"));
            assert!(request.contains("name=\"file\""));
            assert!(request.contains("\x7fELFtest-payload"));
        }
        std::fs::remove_file(path).unwrap();
    }
}
