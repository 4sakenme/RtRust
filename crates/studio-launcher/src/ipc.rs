use anyhow::{Context, Result};
use std::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{error, info};
#[cfg(unix)]
use tracing::warn;

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};

#[cfg(windows)]
use tokio::net::{TcpListener, TcpStream};

#[cfg(unix)]
pub struct IpcServer {
    socket_path: PathBuf,
}

#[cfg(unix)]
impl IpcServer {
    /// Determines socket path adhering to XDG runtime directory.
    pub fn socket_path() -> PathBuf {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let path = PathBuf::from(runtime_dir).join("rtrust");
            let _ = fs::create_dir_all(&path);
            path.join("ipc.sock")
        } else {
            let uid = libc_uid();
            PathBuf::from(format!("/tmp/rtrust-{}.sock", uid))
        }
    }

    pub fn new() -> Self {
        Self {
            socket_path: Self::socket_path(),
        }
    }

    /// Starts the asynchronous IPC listener loop.
    pub async fn run(&self) -> Result<()> {
        if self.socket_path.exists() {
            let _ = fs::remove_file(&self.socket_path);
        }

        let listener = UnixListener::bind(&self.socket_path)
            .with_context(|| format!("Failed to bind IPC socket at {:?}", self.socket_path))?;

        info!("IPC listening on {:?}", self.socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream).await {
                            warn!("Error handling IPC client: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("IPC accept error: {}", e);
                }
            }
        }
    }

    /// Attempts to forward a payload URL to an existing active instance.
    #[allow(dead_code)]
    pub async fn forward_url(url: &str) -> Result<bool> {
        let path = Self::socket_path();
        if !path.exists() {
            return Ok(false);
        }

        use tokio::io::AsyncWriteExt;
        if let Ok(mut stream) = UnixStream::connect(path).await {
            let message = format!("{}\n", url);
            let _ = stream.write_all(message.as_bytes()).await;
            return Ok(true);
        }
        Ok(false)
    }
}

#[cfg(windows)]
pub struct IpcServer {
    port: u16,
}

#[cfg(windows)]
impl IpcServer {
    pub fn new() -> Self {
        Self { port: 48921 }
    }

    pub async fn run(&self) -> Result<()> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr)
            .await
            .with_context(|| format!("Failed to bind IPC listener at {}", addr))?;

        info!("Windows IPC listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((socket, _)) => {
                    tokio::spawn(async move {
                        let mut reader = BufReader::new(socket);
                        let mut line = String::new();
                        while let Ok(n) = reader.read_line(&mut line).await {
                            if n == 0 {
                                break;
                            }
                            let trimmed = line.trim();
                            if !trimmed.is_empty() {
                                info!("Received Windows IPC command: {}", trimmed);
                                if trimmed.starts_with("https://") || trimmed.starts_with("http://")
                                {
                                    let _ = Command::new("cmd")
                                        .args(["/C", "start", "", trimmed])
                                        .spawn();
                                }
                            }
                            line.clear();
                        }
                    });
                }
                Err(e) => {
                    error!("Windows IPC accept error: {}", e);
                }
            }
        }
    }

    #[allow(dead_code)]
    pub async fn forward_url(url: &str) -> Result<bool> {
        use tokio::io::AsyncWriteExt;
        let addr = "127.0.0.1:48921";
        if let Ok(mut stream) = TcpStream::connect(addr).await {
            let message = format!("{}\n", url);
            let _ = stream.write_all(message.as_bytes()).await;
            return Ok(true);
        }
        Ok(false)
    }
}

#[cfg(unix)]
async fn handle_client(stream: UnixStream) -> Result<()> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    while reader.read_line(&mut line).await? > 0 {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            info!("Received IPC command: {}", trimmed);
            if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
                let _ = Command::new("xdg-open").arg(trimmed).spawn();
            } else if trimmed.starts_with("roblox-studio:")
                || trimmed.starts_with("roblox-studio-auth:")
            {
                info!("Forwarding auth URL to Studio handler: {}", trimmed);
            }
        }
        line.clear();
    }

    Ok(())
}

#[cfg(unix)]
fn libc_uid() -> u32 {
    std::process::id()
}
