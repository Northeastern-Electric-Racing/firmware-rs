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
    /// ECU name from the server's configuration.
    #[arg(long)]
    ecu: String,
    /// Application ELF; extensionless Cargo outputs are supported.
    #[arg(long, conflicts_with_all = ["package", "bin"])]
    elf: Option<PathBuf>,
    /// Cargo package to flash when running from a workspace root.
    #[arg(short, long)]
    package: Option<String>,
    /// Binary to flash when the package provides multiple binaries.
    #[arg(long)]
    bin: Option<String>,
    /// Skip requesting entry when the ECU is already in the bootloader.
    #[arg(long)]
    already_in_bootloader: bool,
    /// Maximum upload and flashing wait in seconds.
    #[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u64).range(1..))]
    timeout: u64,
}

fn cargo_json(arguments: &[&str]) -> Result<serde_json::Value> {
    let output = std::process::Command::new("cargo")
        .args(arguments)
        .output()
        .context("cannot run Cargo; supply --elf to flash without Cargo metadata")?;
    ensure!(
        output.status.success(),
        "Cargo failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    serde_json::from_slice(&output.stdout).context("invalid Cargo metadata")
}

fn find_elf(package_name: Option<&str>, binary_name: Option<&str>) -> Result<PathBuf> {
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
    let targets = package["targets"]
        .as_array()
        .context("package has no targets")?;
    let binaries: Vec<_> = targets
        .iter()
        .filter(|t| {
            t["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"))
        })
        .filter_map(|t| t["name"].as_str())
        .collect();
    let name = match binary_name.or_else(|| package["default_run"].as_str()) {
        Some(name) if binaries.contains(&name) => name,
        Some(name) => bail!("package has no binary named {name}"),
        None if binaries.len() == 1 => binaries[0],
        _ => bail!("select an application binary with --bin <name>"),
    };
    let target = PathBuf::from(
        metadata["target_directory"]
            .as_str()
            .context("missing Cargo target directory")?,
    );
    // Cargo stores release binaries directly or below a target triple directory.
    let mut candidates = vec![target.join("release").join(name)];
    if target.is_dir() {
        for entry in std::fs::read_dir(&target)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                candidates.push(entry.path().join("release").join(name));
            }
        }
    }
    candidates.retain(|path| path.is_file());
    candidates.sort();
    candidates.dedup();
    match candidates.as_slice() {
        [path] => Ok(path.clone()),
        [] => bail!(
            "no release ELF found for {name}; build the application normally first, or pass --elf"
        ),
        _ => bail!(
            "multiple release binaries found; select one with --elf:\n{}",
            candidates
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join("\n")
        ),
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
    let elf = match &args.elf {
        Some(path) => path.clone(),
        None => find_elf(args.package.as_deref(), args.bin.as_deref())?,
    };
    let url = endpoint(args.host)?;
    ensure!(!args.ecu.trim().is_empty(), "--ecu must not be empty");
    let mut file = File::open(&elf).with_context(|| format!("cannot open {}", elf.display()))?;
    ensure!(file.metadata()?.is_file(), "--elf must be a regular file");
    let mut magic = [0; 4];
    file.read_exact(&mut magic)
        .context("cannot read ELF header")?;
    ensure!(magic == *b"\x7fELF", "--elf must point to an ELF file");

    let form = multipart::Form::new()
        .text("ecu", args.ecu.clone())
        .text(
            "already_in_bootloader",
            args.already_in_bootloader.to_string(),
        )
        .file("file", &elf)?;
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(args.timeout))
        // A flash request must not be silently forwarded to another endpoint.
        .redirect(Policy::none())
        .build()?;
    println!(
        "Flashing {} via {url} using {}",
        args.ecu.to_ascii_uppercase(),
        elf.display()
    );
    // Do not retry: a lost response does not mean the flash operation failed.
    let response = client.post(url).multipart(form).send().context(
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
        .context("cannot read server result; check server status before retrying")?;
    if status == StatusCode::CONFLICT {
        bail!("CAN interface busy: {}", body.trim());
    }
    ensure!(
        status == StatusCode::OK,
        "Server returned {status}: {}",
        body.trim()
    );
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

fn main() -> ExitCode {
    let Cli {
        command: Command::Flash(args),
    } = Cli::parse();
    match flash(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, net::TcpListener, thread};

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
                ecu: "bms".into(),
                elf: Some(path.clone()),
                package: None,
                bin: None,
                already_in_bootloader: true,
                timeout: 5,
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
