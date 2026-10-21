//! A todo list whose model lives on the server. The page holds a replica: user gestures become
//! actions, the server answers every action with the new model for every connected client, and
//! bindings turn the model into components (rows are created and dropped as items come and go).

use std::{cell::{Cell, RefCell}, rc::Rc};

use moth::{
	Actions, Backoff, Body, Canvas, Component, Edit, ElementRef, Error, Host, Keyed, Label, Link,
	LinkEvent, LinkOptions, Panel, Parent, PushButton, Replica, ToggleButton, console,
};
use moth_sample_protocol::todo::{Action, Item, Todos};

/// An action that cannot be sent (the connection is down) is reported, never fatal. Returns whether
/// it was sent, so that what the user typed is only discarded when it went somewhere.
fn send(actions: &Actions<Action>, action: Action) -> bool {
	match actions.send(&action) {
		Ok(()) => true,
		Err(error) => {
			console::error(&format!("action not sent: {error}"));
			false
		}
	}
}

/// One item: a panel with its text, a toggle and a remove button. Dropping a row removes all of it.
struct Row {
	// the children are declared before the panel that holds them
	label: Label,
	toggle: ToggleButton,
	_remove: PushButton,
	panel: Panel,
}

impl Row {
	fn new(parent: &impl Parent, item: &Item, actions: &Actions<Action>) -> Result<Self, Error> {
		let panel = Panel::new(parent)?;
		let label = Label::new(&panel, &item.text)?;
		let id = item.id;
		let on_toggle = actions.clone();
		// controlled: a click only asks, the model decides, so the toggle can never disagree with it
		let toggle = ToggleButton::controlled(&panel, "done", move |_| {
			send(&on_toggle, Action::Toggle { id });
		})?;
		toggle.set_toggled(item.done);
		let on_remove = actions.clone();
		let remove = PushButton::new(&panel, "\u{d7}", move || {
			send(&on_remove, Action::Remove { id });
		})?;
		Ok(Self { label, toggle, _remove: remove, panel })
	}

	fn update(&mut self, item: &Item) {
		self.label.set_text(&item.text);
		self.toggle.set_toggled(item.done);
	}
}

impl Component for Row {
	fn host(&self) -> &Host {
		self.panel.host()
	}
}

/// The page owns everything. The link is declared first so it is dropped first: no snapshot can
/// reach components that are being torn down.
struct Page {
	_link: Link<Todos, Action>,
	_replica: Replica<Todos>,
	_body: Body,
	_title: Label,
	_status: Label,
	_edit: Edit,
	_add: PushButton,
	_list: Panel,
	_summary: Label,
	_chart: Canvas,
	_clear: PushButton,
}

impl Page {
	fn new() -> Result<Self, Error> {
		let replica = Replica::new(Todos::default());

		let body = Body::new()?;
		let title = Label::new(&body, "> moth todo (shared: open it in two tabs)")?;
		let status = Label::new(&body, "connecting...")?;
		status.element().set_id("todo-status");
		let edit = Edit::new(&body, "", |_| {})?;
		edit.element().set_id("todo-input");
		edit.set_placeholder("what needs doing?");

		let status_text = status.text_handle();
		// the page keeps showing the last model while the server is away, says so, and is brought up
		// to date by the first snapshot of the next connection
		let options = LinkOptions { reconnect: Some(Backoff::default()) };
		let link = Link::connect_with(&url()?, &replica, options, move |event| {
			status_text.set(match event {
				LinkEvent::Open => "connected".to_owned(),
				LinkEvent::Closed(_) => "disconnected".to_owned(),
				LinkEvent::Reconnecting { delay_ms, .. } => {
					format!("disconnected, retrying in {:.1} s", f64::from(delay_ms) / 1000.0)
				}
				LinkEvent::BadSnapshot(error) => format!("bad data from the server: {error}"),
				// this server sends no stream; there is nothing to show for one
				LinkEvent::Binary(_) => return,
			});
		})?;
		let actions = link.actions();

		let (on_add, input) = (actions.clone(), edit.text_handle());
		let add = PushButton::new(&body, "Add", move || {
			if let Some(text) = input.get() {
				// keep what was typed if it could not be sent
				if send(&on_add, Action::Add { text }) {
					input.set(String::new());
				}
			}
		})?;
		add.element().set_id("todo-add");

		let list = Panel::new(&body)?;
		list.element().set_id("todo-list");
		let summary = Label::new(&body, "")?;
		summary.element().set_id("todo-summary");

		// the canvas reads what the binding below stores, and is asked to redraw on change
		let progress = Rc::new(Cell::new((0usize, 0usize)));
		let shown = progress.clone();
		let chart = Canvas::new(&body, move |context, size| {
			let (done, total) = shown.get();
			context.set_fill_style_str("rgba(128, 128, 128, 0.3)");
			context.fill_rect(0.0, 0.0, size.width, size.height);
			if total > 0 {
				context.set_fill_style_str("rgb(46, 160, 67)");
				context.fill_rect(0.0, 0.0, size.width * done as f64 / total as f64, size.height);
			}
		})?;

		let on_clear = actions.clone();
		let clear = PushButton::new(&body, "Clear done", move || {
			send(&on_clear, Action::ClearDone);
		})?;
		clear.element().set_id("todo-clear");

		// bindings: each runs when its own part of the model changes
		let (rows, root, on_row) = (RefCell::new(Keyed::<u32, Row>::new()), ElementRef::new(list.element()), actions);
		replica.bind(
			|model| model.items.clone(),
			move |items| {
				let result = rows.borrow_mut().sync(
					root.element(),
					items,
					|item| item.id,
					|item| Row::new(&root, item, &on_row),
					|row, item| row.update(item),
				);
				if let Err(error) = result {
					console::error(&format!("cannot render the list: {error}"));
				}
			},
		);
		let (summary_text, canvas) = (summary.text_handle(), chart.handle());
		replica.bind(
			|model| (model.done_count(), model.items.len()),
			move |&(done, total)| {
				summary_text.set(format!("{done} of {total} done"));
				progress.set((done, total));
				canvas.invalidate();
			},
		);

		Ok(Self {
			_link: link,
			_replica: replica,
			_body: body,
			_title: title,
			_status: status,
			_edit: edit,
			_add: add,
			_list: list,
			_summary: summary,
			_chart: chart,
			_clear: clear,
		})
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
	Ok(format!("{scheme}://{host}/todo/ws"))
}

fn main() {
	moth::init();
	let page = Page::new().expect("page creation failed");
	moth::mount(page).expect("mount failed");
}
