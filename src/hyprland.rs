use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

pub struct HyprlandIntegration {
    socket_path: Option<String>,
}

impl HyprlandIntegration {
    pub fn new() -> Self {
        let socket_path = env::var("HYPRLAND_INSTANCE_SIGNATURE").ok().map(|sig| {
            let xdg = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
            format!("{}/hypr/{}/.socket.sock", xdg, sig)
        });

        Self { socket_path }
    }

    pub fn is_hyprland(&self) -> bool {
        self.socket_path.is_some()
    }

    pub fn dispatch(&self, command: &str) -> anyhow::Result<String> {
        let path = self
            .socket_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not running on Hyprland"))?;

        let mut stream = UnixStream::connect(path)?;
        stream.write_all(command.as_bytes())?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        Ok(response)
    }

    /// Fullscreen toggle (mode 1 = maximize tanpa decoration)
    pub fn toggle_fullscreen(&self) -> anyhow::Result<()> {
        self.dispatch("dispatch fullscreen 1")?;
        Ok(())
    }

    pub fn toggle_float(&self) -> anyhow::Result<()> {
        self.dispatch("dispatch togglefloating active")?;
        Ok(())
    }
}
