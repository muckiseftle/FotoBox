//! The camera actor: a dedicated OS thread that exclusively owns the camera
//! backend. libgphoto2 calls are blocking, so they must never run on the async
//! runtime — they run here instead, serialized one at a time.
//!
//! Live-view fan-out: the actor grabs preview frames only while at least one
//! viewer is registered (see [`PreviewGuard`]) and publishes the latest frame
//! into a `watch` channel. Every live-view client reads that same shared frame,
//! so there is exactly one camera read regardless of how many phones/kiosks are
//! watching.

use crate::{CameraBackend, CameraError, CameraInfo, CameraState, CaptureResult, PreviewFrame};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, watch};

/// Target preview pacing (~20 fps). The real camera may deliver fewer.
const PREVIEW_INTERVAL: Duration = Duration::from_millis(50);

type BackendMaker = Box<dyn FnOnce() -> Result<Box<dyn CameraBackend>, CameraError> + Send>;

enum Command {
    Detect(oneshot::Sender<Result<CameraInfo, CameraError>>),
    Capture(oneshot::Sender<Result<CaptureResult, CameraError>>),
    /// Replace the backend (constructed on the actor thread) — e.g. switch webcam.
    Reconfigure(BackendMaker, oneshot::Sender<Result<(), CameraError>>),
    AddViewer,
    RemoveViewer,
    Shutdown,
}

/// Cheaply-clonable async handle to the camera actor.
#[derive(Clone)]
pub struct CameraHandle {
    cmd_tx: mpsc::Sender<Command>,
    preview_rx: watch::Receiver<PreviewFrame>,
    state_rx: watch::Receiver<CameraState>,
    info_rx: watch::Receiver<CameraInfo>,
}

impl CameraHandle {
    /// Current info about the camera this actor owns (a snapshot).
    pub fn info(&self) -> CameraInfo {
        self.info_rx.borrow().clone()
    }

    /// Trigger the shutter and return the captured JPEG. Any in-progress
    /// preview is paused for the duration and resumes afterwards.
    pub async fn capture(&self) -> Result<CaptureResult, CameraError> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::Capture(tx))
            .await
            .map_err(|_| CameraError::ActorGone)?;
        rx.await.map_err(|_| CameraError::ActorGone)?
    }

    /// Re-probe the camera and return its info.
    pub async fn detect(&self) -> Result<CameraInfo, CameraError> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::Detect(tx))
            .await
            .map_err(|_| CameraError::ActorGone)?;
        rx.await.map_err(|_| CameraError::ActorGone)?
    }

    /// Replace the camera backend at runtime (built on the actor thread).
    pub async fn reconfigure(&self, maker: BackendMaker) -> Result<(), CameraError> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::Reconfigure(maker, tx))
            .await
            .map_err(|_| CameraError::ActorGone)?;
        rx.await.map_err(|_| CameraError::ActorGone)?
    }

    /// Switch to a specific webcam by index, live.
    pub async fn use_webcam(&self, index: u32) -> Result<(), CameraError> {
        self.reconfigure(Box::new(move || {
            crate::webcam::WebcamCamera::open_index(index)
                .map(|c| Box::new(c) as Box<dyn CameraBackend>)
        }))
        .await
    }

    /// Subscribe to the shared live-view frame stream.
    pub fn subscribe_preview(&self) -> watch::Receiver<PreviewFrame> {
        self.preview_rx.clone()
    }

    /// Subscribe to camera state changes (for honest UI status).
    pub fn subscribe_state(&self) -> watch::Receiver<CameraState> {
        self.state_rx.clone()
    }

    /// Register as a live-view viewer; preview grabbing runs while ≥1 guard is
    /// alive. Dropping the guard releases the viewer (best-effort).
    pub async fn preview_guard(&self) -> PreviewGuard {
        let _ = self.cmd_tx.send(Command::AddViewer).await;
        PreviewGuard {
            cmd_tx: self.cmd_tx.clone(),
        }
    }

    /// Ask the actor to shut down its thread.
    pub async fn shutdown(&self) {
        let _ = self.cmd_tx.send(Command::Shutdown).await;
    }
}

/// RAII viewer registration. While held, the actor produces preview frames;
/// on drop it decrements the viewer count (best-effort, non-blocking).
pub struct PreviewGuard {
    cmd_tx: mpsc::Sender<Command>,
}

impl Drop for PreviewGuard {
    fn drop(&mut self) {
        let _ = self.cmd_tx.try_send(Command::RemoveViewer);
    }
}

/// Spawn the camera actor on a dedicated thread, returning an async handle.
///
/// The backend is **constructed on the actor thread** via `make` (which must be
/// `Send`, but the backend it returns need not be — important for webcam SDK
/// handles that are not `Send`). Info is published once the backend is ready.
pub fn spawn(make: Box<dyn FnOnce() -> Box<dyn CameraBackend> + Send>) -> CameraHandle {
    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(32);
    let (preview_tx, preview_rx) = watch::channel::<PreviewFrame>(None);
    let (state_tx, state_rx) = watch::channel(CameraState::Idle);
    let (info_tx, info_rx) = watch::channel(CameraInfo {
        model: "Verbinde …".to_string(),
        port: String::new(),
        backend: "none",
    });

    std::thread::Builder::new()
        .name("camera-actor".to_string())
        .spawn(move || {
            let backend = make();
            let _ = info_tx.send(backend.info());
            run(backend, cmd_rx, preview_tx, state_tx, info_tx);
        })
        .expect("failed to spawn camera-actor thread");

    CameraHandle {
        cmd_tx,
        preview_rx,
        state_rx,
        info_rx,
    }
}

fn run(
    mut backend: Box<dyn CameraBackend>,
    mut cmd_rx: mpsc::Receiver<Command>,
    preview_tx: watch::Sender<PreviewFrame>,
    state_tx: watch::Sender<CameraState>,
    info_tx: watch::Sender<CameraInfo>,
) {
    let mut viewers: usize = 0;
    let set_state = |s: CameraState| {
        let _ = state_tx.send_if_modified(|cur| {
            if *cur != s {
                *cur = s.clone();
                true
            } else {
                false
            }
        });
    };

    loop {
        if viewers > 0 {
            // Drain any pending commands without blocking the preview loop.
            loop {
                match cmd_rx.try_recv() {
                    Ok(cmd) => {
                        if process(cmd, &mut backend, &mut viewers, &set_state, &info_tx) {
                            return;
                        }
                    }
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => return,
                }
            }

            if viewers > 0 {
                set_state(CameraState::Previewing);
                match backend.preview_frame() {
                    Ok(jpeg) => {
                        let _ = preview_tx.send(Some(Arc::new(jpeg)));
                    }
                    Err(e) => set_state(CameraState::Error {
                        message: e.to_string(),
                    }),
                }
                std::thread::sleep(PREVIEW_INTERVAL);
            }
        } else {
            set_state(CameraState::Idle);
            match cmd_rx.blocking_recv() {
                Some(cmd) => {
                    if process(cmd, &mut backend, &mut viewers, &set_state, &info_tx) {
                        return;
                    }
                }
                None => return,
            }
        }
    }
}

/// Handle one command. Returns `true` if the actor should shut down.
fn process(
    cmd: Command,
    backend: &mut Box<dyn CameraBackend>,
    viewers: &mut usize,
    set_state: &impl Fn(CameraState),
    info_tx: &watch::Sender<CameraInfo>,
) -> bool {
    match cmd {
        Command::Detect(resp) => {
            let _ = resp.send(Ok(backend.info()));
        }
        Command::Capture(resp) => {
            set_state(CameraState::Capturing);
            let result = backend.capture();
            if let Err(e) = &result {
                set_state(CameraState::Error {
                    message: e.to_string(),
                });
            }
            let _ = resp.send(result);
        }
        Command::Reconfigure(maker, resp) => match maker() {
            Ok(new_backend) => {
                *backend = new_backend;
                let _ = info_tx.send(backend.info());
                set_state(CameraState::Idle);
                let _ = resp.send(Ok(()));
            }
            Err(e) => {
                set_state(CameraState::Error {
                    message: e.to_string(),
                });
                let _ = resp.send(Err(e));
            }
        },
        Command::AddViewer => *viewers += 1,
        Command::RemoveViewer => *viewers = viewers.saturating_sub(1),
        Command::Shutdown => return true,
    }
    false
}
