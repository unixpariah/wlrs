mod surface;

use crate::{error::WlrsError, helpers::resize, WallpaperData};
use rayon::prelude::*;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm,
    output::{OutputHandler, OutputState},
    reexports::{calloop, calloop_wayland_source::WaylandSource},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        wlr_layer::{
            Anchor, Layer, LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
    shm::{slot::SlotPool, Shm, ShmHandler},
};
use std::sync::mpsc;
use surface::Surface;
use wayland_client::{
    globals::{registry_queue_init, GlobalList},
    protocol::{wl_output, wl_shm, wl_surface},
    Connection, QueueHandle,
};

pub(crate) struct Wlrs {
    registry_state: RegistryState,
    output_state: OutputState,
    compositor_state: CompositorState,
    layer_shell: LayerShell,
    surfaces: Vec<Surface>,
    shm: Shm,
}

impl Wlrs {
    fn new(
        globals: &GlobalList,
        qh: &wayland_client::QueueHandle<Self>,
    ) -> Result<Self, WlrsError> {
        let compositor_state = CompositorState::bind(globals, qh)?;
        let layer_shell = LayerShell::bind(globals, qh)?;
        let shm = Shm::bind(globals, qh)?;

        Ok(Self {
            compositor_state,
            layer_shell,
            output_state: OutputState::new(globals, qh),
            registry_state: RegistryState::new(globals),
            surfaces: Vec::new(),
            shm,
        })
    }
}

impl CompositorHandler for Wlrs {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for Wlrs {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        let surface = self.compositor_state.create_surface(qh);
        let layer = self.layer_shell.create_layer_surface(
            qh,
            surface,
            Layer::Background,
            Some("wlrs"),
            Some(&output),
        );

        if let Some(output_info) = self.output_state.info(&output) {
            layer.set_anchor(Anchor::all());
            layer.set_exclusive_zone(-1);
            layer.commit();

            let (width, height) = (
                output_info.logical_size.unwrap().0,
                output_info.logical_size.unwrap().1,
            );
            let pool = SlotPool::new((width * height * 4) as usize, &self.shm).unwrap();

            let pool = Box::leak(Box::new(pool));

            let (buffer, canvas) = pool
                .create_buffer(width, height, width * 4, wl_shm::Format::Xbgr8888)
                .unwrap();

            self.surfaces.push(Surface {
                layer_surface: layer,
                output_info,
                width: 0,
                height: 0,
                buffer,
                canvas,
            });
        }
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        if let Some(output_info) = self.output_state.info(&output) {
            self.surfaces
                .retain(|info| info.output_info.id != output_info.id);
        }
    }
}

impl LayerShellHandler for Wlrs {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {}

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        _configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let surface = self
            .surfaces
            .iter_mut()
            .find(|surface| &surface.layer_surface == layer)
            .unwrap();
        surface.change_size();
    }
}

impl ShmHandler for Wlrs {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

pub fn wayland(
    rx: mpsc::Receiver<WallpaperData>,
    tx: mpsc::Sender<Result<(), WlrsError>>,
    ping_source: calloop::ping::PingSource,
) -> Result<(), WlrsError> {
    let conn = Connection::connect_to_env()?;
    let (globals, event_queue) = registry_queue_init(&conn)?;
    let qh = event_queue.handle();
    let mut wlrs = Wlrs::new(&globals, &qh)?;

    let mut event_loop = calloop::EventLoop::try_new()?;
    WaylandSource::new(conn, event_queue).insert(event_loop.handle())?;
    event_loop
        .handle()
        .insert_source(ping_source, |_, _, _| {})
        .map_err(|_| WlrsError::WaylandError("Failed to insert listener".to_string()))?;

    loop {
        event_loop.dispatch(None, &mut wlrs)?;
        if !wlrs.surfaces.iter().any(|surface| surface.is_configured()) {
            continue;
        }

        if let Ok(wallpaper) = rx.try_recv() {
            let drawn = wlrs
                .surfaces
                .par_iter_mut()
                .map(|surface| {
                    if surface.is_configured()
                        && (wallpaper
                            .outputs
                            .contains(surface.output_info.name.as_ref().unwrap())
                            || wallpaper.outputs.is_empty())
                    {
                        surface
                            .canvas
                            .copy_from_slice(&resize(&wallpaper, [surface.width, surface.height])?);
                        surface.draw();
                        return Ok::<bool, WlrsError>(true);
                    }
                    Ok(false)
                })
                .reduce_with(|a, b| match (a, b) {
                    (Ok(a), Ok(b)) => Ok(a || b),
                    (Err(e), _) | (_, Err(e)) => Err(e),
                })
                .unwrap_or(Ok(false))?;

            if drawn {
                tx.send(Ok(()))?;
            }
        }
    }
}

delegate_compositor!(Wlrs);
delegate_output!(Wlrs);
delegate_shm!(Wlrs);
delegate_layer!(Wlrs);
delegate_registry!(Wlrs);

impl ProvidesRegistryState for Wlrs {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}
