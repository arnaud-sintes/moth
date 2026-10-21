use moth::{Body, Component, Label, Panel, ToggleButton, event_detail, events};

fn main() {
	let body = Body::new().unwrap();
	let panel = Panel::new(&body).unwrap();
	let status = Label::new(&body, "off").unwrap();

	// the toggle knows nothing about the panel that holds it or the label that reports on it...
	let power = ToggleButton::new(&panel, "Power", |_| {}).unwrap();

	// ...it raises a bubbling, composed event that any ancestor (or script in the page) can hear
	let text = status.text_handle();
	let listener = panel
		.on(events::TOGGLE, move |event| {
			let on = event_detail(&event).and_then(|detail| detail.as_bool()).unwrap_or(false);
			text.set(if on { "on" } else { "off" }.to_owned());
		})
		.unwrap();

	// components raise their own the same way: `component.emit("my-event", &detail)`
	moth::mount((body, panel, status, power, listener)).unwrap();
}
