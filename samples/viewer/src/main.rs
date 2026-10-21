//! A remote 3D viewer: the object is rendered by the server and streamed here as JPEG frames. What is
//! shown (object, material, light, camera, turntable) is one scene shared by everyone connected to
//! the server: the page asks for changes and its controls follow what the server announces. It also
//! keeps a live account of its own stream (frame rate, round trip, bandwidth, what the server spent
//! rendering and encoding).

mod orbit;
mod outbox;
mod session;
mod stats;

use std::{any::Any, cell::RefCell, rc::Rc};

use moth::{
	Backoff, Body, Canvas, Component, Dropdown, Error, Label, Link, LinkEvent, LinkOptions, Listener,
	Panel, Parent, PushButton, Replica, Slider, ToggleButton, WeakProp, console,
};
use moth_sample_protocol::viewer::{ClientMsg, Display, Hub, Material, Model, Scene};
use wasm_bindgen::JsCast;
use web_sys::{PointerEvent, WheelEvent};

use session::{Hud, Session};

/// Everything the page owns. The link is declared first so that it is dropped first: no frame can
/// reach components that are being torn down.
struct Page {
	_link: Link<Hub, ClientMsg>,
	_replica: Replica<Hub>,
	_listeners: Vec<Listener>,
	_session: Rc<Session>,
	_components: Vec<Box<dyn Any>>,
}

fn class<C: Component>(component: C, name: &str) -> C {
	component.element().set_class_name(name);
	component
}

fn id<C: Component>(component: C, name: &str) -> C {
	component.element().set_id(name);
	component
}

/// A titled card in the sidebar.
fn card(parent: &impl Parent, title: &str, keep: &mut Vec<Box<dyn Any>>) -> Result<Panel, Error> {
	let card = class(Panel::new(parent)?, "card");
	keep.push(Box::new(class(Label::new(&card, title)?, "card-title")));
	Ok(card)
}

/// A labelled row; the caller puts the control in it.
fn row(parent: &impl Parent, name: &str, keep: &mut Vec<Box<dyn Any>>) -> Result<Panel, Error> {
	let row = class(Panel::new(parent)?, "row");
	keep.push(Box::new(class(Label::new(&row, name)?, "row-name")));
	Ok(row)
}

/// A row with a slider and its value. `show` turns the slider's number into the text beside it.
/// `on_change` is told what the user does; the function returned shows a value that came from
/// elsewhere (the server), slider and text, without calling `on_change`.
#[allow(clippy::too_many_arguments)]
fn slider_row(
	parent: &impl Parent,
	element_id: &str,
	name: &str,
	range: (f64, f64, f64),
	value: f64,
	show: fn(f64) -> String,
	on_change: impl Fn(f64) + 'static,
	keep: &mut Vec<Box<dyn Any>>,
) -> Result<impl Fn(f64) + 'static, Error> {
	let row = class(Panel::new(parent)?, "row with-value");
	keep.push(Box::new(class(Label::new(&row, name)?, "row-name")));
	let readout = class(Label::new(&row, "")?, "row-value");
	let text = readout.text_handle();
	text.set(show(value));
	let slider = Slider::new(&row, 0.0, move |value| {
		text.set(show(value));
		on_change(value);
	})?;
	let (min, max, step) = range;
	slider.set_min(min);
	slider.set_max(max);
	slider.set_step(step);
	slider.set_value(value);
	slider.element().set_id(element_id);
	// the value label comes after the slider in the row
	row.element().append_child(readout.element())?;
	let (handle, shown) = (slider.handle(), readout.text_handle());
	keep.extend([Box::new(slider) as Box<dyn Any>, Box::new(readout), Box::new(row)]);
	Ok(move |value: f64| {
		handle.set_value(value);
		shown.set(show(value));
	})
}

/// A HUD readout: a small name over a value. Returns the handle that sets the value.
fn stat(parent: &impl Parent, name: &str, keep: &mut Vec<Box<dyn Any>>) -> Result<WeakProp<String>, Error> {
	let cell = class(Panel::new(parent)?, "stat");
	keep.push(Box::new(class(Label::new(&cell, name)?, "stat-name")));
	let value = class(Label::new(&cell, "\u{2013}")?, "stat-value");
	let handle = value.text_handle();
	keep.extend([Box::new(value) as Box<dyn Any>, Box::new(cell)]);
	Ok(handle)
}

fn with_thousands(number: u32) -> String {
	let digits = number.to_string();
	let mut grouped = String::new();
	for (index, digit) in digits.chars().enumerate() {
		if index > 0 && (digits.len() - index) % 3 == 0 {
			grouped.push(',');
		}
		grouped.push(digit);
	}
	grouped
}

impl Page {
	fn new() -> Result<Page, Error> {
		let session = Session::new();
		let mut keep: Vec<Box<dyn Any>> = Vec::new();
		let mut listeners: Vec<Listener> = Vec::new();

		let body = Body::new()?;
		let app = id(Panel::new(&body)?, "app");

		// ---- the top bar
		let topbar = id(Panel::new(&app)?, "topbar");
		keep.push(Box::new(class(Label::new(&topbar, "moth viewer")?, "brand")));
		keep.push(Box::new(class(
			Label::new(&topbar, "a 3D object rendered on the server and streamed as JPEG frames")?,
			"tagline",
		)));
		keep.push(Box::new(class(Panel::new(&topbar)?, "spacer")));
		let viewers = id(Label::new(&topbar, "1 viewer")?, "viewers");
		let status = id(Label::new(&topbar, "connecting")?, "status");
		status.element().set_attribute("data-state", "down")?;

		// ---- the stage: the stream, a HUD on it, a hint
		let stage = id(Panel::new(&app)?, "stage");
		let viewport = {
			let session = session.clone();
			Canvas::new(&stage, move |context, size| session.draw(context, size))?
		};
		viewport.element().set_id("viewport");

		let hud_panel = id(Panel::new(&stage)?, "hud");
		let hud = Hud {
			fps: stat(&hud_panel, "frames/s", &mut keep)?,
			latency: stat(&hud_panel, "latency", &mut keep)?,
			frame: stat(&hud_panel, "frame", &mut keep)?,
			rate: stat(&hud_panel, "stream", &mut keep)?,
			size: stat(&hud_panel, "size", &mut keep)?,
			render: stat(&hud_panel, "server render", &mut keep)?,
			encode: stat(&hud_panel, "server encode", &mut keep)?,
		};
		let sparkline = {
			let session = session.clone();
			Canvas::new(&hud_panel, move |context, size| session.draw_sparkline(context, size))?
		};
		sparkline.element().set_id("spark");
		keep.push(Box::new(id(
			Label::new(&stage, "drag to orbit \u{b7} scroll to zoom \u{b7} double-click to reset")?,
			"hint",
		)));

		// ---- the pointer drives the camera
		let element = viewport.element().clone();
		listeners.push(viewport.on("pointerdown", {
			let (session, element) = (session.clone(), element.clone());
			move |event| {
				let Ok(pointer) = event.dyn_into::<PointerEvent>() else { return };
				if pointer.button() == 0 {
					// keep receiving the moves when the pointer leaves the canvas
					let _ = element.set_pointer_capture(pointer.pointer_id());
					let _ = element.set_attribute("data-dragging", "");
					session.pointer_down(f64::from(pointer.client_x()), f64::from(pointer.client_y()));
				}
			}
		})?);
		listeners.push(viewport.on("pointermove", {
			let session = session.clone();
			move |event| {
				if let Ok(pointer) = event.dyn_into::<PointerEvent>() {
					session.pointer_move(f64::from(pointer.client_x()), f64::from(pointer.client_y()));
				}
			}
		})?);
		for name in ["pointerup", "pointercancel"] {
			listeners.push(viewport.on(name, {
				let (session, element) = (session.clone(), element.clone());
				move |_| {
					session.pointer_up();
					let _ = element.remove_attribute("data-dragging");
				}
			})?);
		}
		listeners.push(viewport.on("wheel", {
			let session = session.clone();
			move |event| {
				if let Ok(wheel) = event.dyn_into::<WheelEvent>() {
					wheel.prevent_default(); // the page must not scroll under the object
					session.wheel(wheel.delta_y());
				}
			}
		})?);
		listeners.push(viewport.on("dblclick", {
			let session = session.clone();
			move |_| session.reset_camera()
		})?);

		// ---- the sidebar
		let sidebar = id(Panel::new(&app)?, "sidebar");

		let object = card(&sidebar, "Object", &mut keep)?;
		let models: Rc<RefCell<Vec<Model>>> = Rc::new(RefCell::new(Model::ALL.to_vec()));
		let model_row = row(&object, "Model", &mut keep)?;
		let model_menu = {
			let (session, models) = (session.clone(), models.clone());
			Dropdown::new(
				&model_row,
				Model::ALL.iter().map(|model| model.label().to_owned()).collect(),
				move |chosen| {
					if let Some(model) = chosen.and_then(|index| models.borrow().get(index).copied()) {
						session.set_model(model);
					}
				},
			)?
		};
		model_menu.set_selected(Some(0));
		model_menu.element().set_id("model-menu");
		let material_row = row(&object, "Material", &mut keep)?;
		let material_menu = {
			let session = session.clone();
			Dropdown::new(
				&material_row,
				Material::ALL.iter().map(|material| material.label().to_owned()).collect(),
				move |chosen| {
					if let Some(material) = chosen.and_then(|index| Material::ALL.get(index).copied()) {
						session.set_material(material);
					}
				},
			)?
		};
		material_menu.set_selected(Some(0));
		material_menu.element().set_id("material-menu");
		let material_menu = Rc::new(material_menu);
		keep.extend([
			Box::new(model_row) as Box<dyn Any>,
			Box::new(material_row),
			Box::new(object),
		]);

		let defaults = Scene::default();
		let lighting = card(&sidebar, "Lighting", &mut keep)?;
		let show_light = slider_row(
			&lighting,
			"light-slider",
			"Light angle",
			(-180.0, 180.0, 1.0),
			f64::from(defaults.light).to_degrees().round(),
			|degrees| format!("{degrees:.0}\u{b0}"),
			{
				let session = session.clone();
				move |degrees| session.set_light((degrees as f32).to_radians())
			},
			&mut keep,
		)?;
		let show_shininess = slider_row(
			&lighting,
			"shininess-slider",
			"Shininess",
			(2.0, 200.0, 1.0),
			f64::from(defaults.shininess),
			|value| format!("{value:.0}"),
			{
				let session = session.clone();
				move |value| session.set_shininess(value as f32)
			},
			&mut keep,
		)?;
		keep.push(Box::new(lighting));

		// how the picture is made for this browser is its own, not part of the shared scene: its size and
		// compression when the scene rests, and again while it moves
		let at_rest = card(&sidebar, "At rest", &mut keep)?;
		let _ = slider_row(
			&at_rest,
			"resolution-slider",
			"Resolution",
			(25.0, 100.0, 5.0),
			f64::from(Display::default().scale) * 100.0,
			|percent| format!("{percent:.0}%"),
			{
				let session = session.clone();
				move |percent| session.set_scale((percent / 100.0) as f32)
			},
			&mut keep,
		)?;
		let _ = slider_row(
			&at_rest,
			"quality-slider",
			"JPEG quality",
			(10.0, 100.0, 5.0),
			f64::from(Display::default().quality),
			|quality| format!("{quality:.0}"),
			{
				let session = session.clone();
				move |quality| session.set_quality(quality as u32)
			},
			&mut keep,
		)?;
		keep.push(Box::new(at_rest));

		let moving = card(&sidebar, "While moving", &mut keep)?;
		let _ = slider_row(
			&moving,
			"preview-slider",
			"Resolution",
			(10.0, 100.0, 5.0),
			f64::from(Display::default().preview) * 100.0,
			|percent| format!("{percent:.0}%"),
			{
				let session = session.clone();
				move |percent| session.set_preview((percent / 100.0) as f32)
			},
			&mut keep,
		)?;
		let _ = slider_row(
			&moving,
			"preview-quality-slider",
			"JPEG quality",
			(10.0, 100.0, 5.0),
			f64::from(Display::default().preview_quality),
			|quality| format!("{quality:.0}"),
			{
				let session = session.clone();
				move |quality| session.set_preview_quality(quality as u32)
			},
			&mut keep,
		)?;
		keep.push(Box::new(moving));

		let rendering = card(&sidebar, "View", &mut keep)?;
		let toggles = class(Panel::new(&rendering)?, "toggles");
		// controlled: a click is a request, and the button shows what the server then announces
		let wireframe = {
			let session = session.clone();
			ToggleButton::controlled(&toggles, "Wireframe", move |on| session.set_wireframe(on))?
		};
		let spin = {
			let session = session.clone();
			ToggleButton::controlled(&toggles, "Auto-rotate", move |on| session.set_spin(on))?
		};
		let (wireframe_state, spin_state) = (wireframe.toggled_handle(), spin.toggled_handle());
		let reset = {
			let session = session.clone();
			PushButton::new(&toggles, "Reset camera", move || session.reset_camera())?
		};
		wireframe.element().set_id("wireframe-toggle");
		spin.element().set_id("spin-toggle");
		reset.element().set_id("reset-button");
		keep.extend([
			Box::new(wireframe) as Box<dyn Any>,
			Box::new(spin),
			Box::new(reset),
			Box::new(toggles),
			Box::new(rendering),
		]);

		let about = card(&sidebar, "How it works", &mut keep)?;
		keep.push(Box::new(class(
			Label::new(
				&about,
				"The object, the light and the camera are shared: everyone connected to this server steers and sees \
				 the same scene (another server, another scene). The server rasterizes it on its CPU cores and sends \
				 each viewer a JPEG at its own size. While the scene moves you get fast previews, and one \
				 anti-aliased frame when it rests. Only the newest state is rendered, and at most two frames are \
				 ever in flight.",
			)?,
			"about",
		)));
		keep.push(Box::new(about));

		// ---- the server: whole-model snapshots (who is watching) and the frame stream
		let replica = Replica::new(Hub::default());
		let viewers_text = viewers.text_handle();
		replica.bind(
			|hub| hub.viewers,
			move |count| {
				viewers_text.set(format!("{count} viewer{}", if *count == 1 { "" } else { "s" }));
			},
		);
		// the menu is rebuilt from what the server lists, which the replica's binding below does
		let menu = Rc::new(RefCell::new(Some(model_menu)));
		// the scene is the server's: whenever it announces one (somebody else's change, the turntable,
		// the answer to this page's own request) the controls show it. Setting a control from code
		// never calls its callback, so this cannot loop back to the server.
		replica.bind(
			|hub| hub.scene,
			{
				let (session, models, menu, material_menu) = (session.clone(), models.clone(), menu.clone(), material_menu.clone());
				move |scene| {
					if !session.scene_announced(scene) {
						return;
					}
					if let Some(menu) = menu.borrow().as_ref() {
						menu.set_selected(models.borrow().iter().position(|model| *model == scene.model));
					}
					material_menu.set_selected(Material::ALL.iter().position(|material| *material == scene.material));
					show_light(f64::from(scene.light).to_degrees().round());
					show_shininess(f64::from(scene.shininess).round());
					wireframe_state.set(scene.wireframe);
					spin_state.set(scene.spin);
				}
			},
		);
		{
			let (menu, models) = (menu.clone(), models.clone());
			replica.bind(
				|hub| {
					hub.models
						.iter()
						.map(|info| (info.model, format!("{} \u{b7} {} tris", info.name, with_thousands(info.triangles))))
						.collect::<Vec<_>>()
				},
				move |listed| {
					if listed.is_empty() {
						return;
					}
					*models.borrow_mut() = listed.iter().map(|(model, _)| *model).collect();
					if let Some(menu) = menu.borrow().as_ref() {
						menu.set_options(listed.iter().map(|(_, label)| label.clone()).collect());
					}
				},
			);
		}

		let (status_text, status_element) = (status.text_handle(), status.element().clone());
		let link = Link::connect_with(
			&url()?,
			&replica,
			LinkOptions { reconnect: Some(Backoff::default()) },
			{
				let session = session.clone();
				move |event| {
					let state = |state: &str, text: String| {
						let _ = status_element.set_attribute("data-state", state);
						status_text.set(text);
					};
					match event {
						LinkEvent::Open => {
							session.connection(true);
							state("live", "live".to_owned());
						}
						LinkEvent::Closed(_) => {
							session.connection(false);
							state("down", "offline".to_owned());
						}
						LinkEvent::Reconnecting { delay_ms, .. } => {
							state("down", format!("reconnecting in {:.1} s", f64::from(delay_ms) / 1000.0));
						}
						LinkEvent::BadSnapshot(error) => console::error(&format!("bad data from the server: {error}")),
						LinkEvent::Binary(frame) => session.frame(frame),
					}
				}
			},
		)?;
		session.attach(link.actions(), viewport.handle(), sparkline.handle(), hud);
		session.start_ticker()?;

		keep.extend([
			Box::new(body) as Box<dyn Any>,
			Box::new(app),
			Box::new(topbar),
			Box::new(viewers),
			Box::new(status),
			Box::new(stage),
			Box::new(viewport),
			Box::new(hud_panel),
			Box::new(sparkline),
			Box::new(sidebar),
			Box::new(menu),
			Box::new(material_menu),
		]);
		Ok(Page { _link: link, _replica: replica, _listeners: listeners, _session: session, _components: keep })
	}
}

/// The back-end's WebSocket URL: the same origin as the page (the back-end serves the page, and
/// allows only its own pages by default), over `wss` if the page is `https`. For a page that comes
/// from elsewhere (`trunk serve`), `?backend=127.0.0.1:2000` names the back-end.
fn url() -> Result<String, Error> {
	let location = web_sys::window().ok_or_else(|| Error::new("no window"))?.location();
	let scheme = if location.protocol()? == "https:" { "wss" } else { "ws" };
	let backend = web_sys::UrlSearchParams::new_with_str(&location.search()?)?.get("backend");
	let host = match backend {
		Some(backend) => backend,
		None => location.host()?,
	};
	Ok(format!("{scheme}://{host}/viewer/ws"))
}

fn main() {
	moth::init();
	let page = Page::new().expect("page creation failed");
	moth::mount(page).expect("mount failed");
}
