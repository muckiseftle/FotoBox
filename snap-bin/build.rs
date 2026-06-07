//! Build script: embeds the application icon into the Windows executable so the
//! installed app, taskbar and shortcuts show the SnapStation logo. No-op on
//! other platforms.

fn main() {
    #[cfg(windows)]
    {
        let icon = "../branding/icon.ico";
        println!("cargo:rerun-if-changed={icon}");
        let mut res = winresource::WindowsResource::new();
        res.set_icon(icon);
        // Non-fatal: if the resource compiler is unavailable locally, the build
        // still succeeds (the icon simply isn't embedded).
        if let Err(e) = res.compile() {
            println!("cargo:warning=icon embed skipped: {e}");
        }
    }
}
