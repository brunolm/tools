use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Tool {
    pub name: &'static str,
    folder: &'static str,
    exe: &'static str,
    args: &'static [&'static str],
}

pub const TOOLS: &[Tool] = &[
    Tool {
        name: "Disk Cleanup",
        folder: "disk-cleanup",
        exe: "target/release/disk-cleanup.exe",
        args: &[],
    },
    Tool {
        name: "Extract Images from Video",
        folder: "extract-images-from-video",
        exe: "src-tauri/target/release/extract-images-from-video.exe",
        args: &[],
    },
    Tool {
        name: "File Finder",
        folder: "filefinder",
        exe: "node_modules/electron/dist/electron.exe",
        args: &["."],
    },
    Tool {
        name: "Temp Monitor",
        folder: "temp-monitor",
        exe: "target/release/temp-monitor.exe",
        args: &[],
    },
];

impl Tool {
    pub fn is_built(&self) -> bool {
        self.exe_path().is_file()
    }

    pub fn launch(&self) -> io::Result<()> {
        Command::new(self.exe_path())
            .args(self.args)
            .current_dir(self.dir())
            .spawn()
            .map(drop)
    }

    fn exe_path(&self) -> PathBuf {
        self.dir().join(self.exe)
    }

    fn dir(&self) -> PathBuf {
        repo_root().join(self.folder)
    }
}

/// The repo root is fixed at compile time: the launcher runs from its own build output inside the repo.
fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}
